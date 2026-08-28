//! Is somebody playing a game on this machine?
//!
//! This is the one contention source that deserves a different answer from all
//! the others. A browser or a Spotlight reindex is a nuisance: demote to the
//! efficiency cores, accept a dirty timing, keep the work. A game is somebody
//! actually using the computer they own, and the polite response is to get
//! entirely out of the way -- SIGSTOP everything and hold.
//!
//! PRESENCE IS NOT ENOUGH, and getting this wrong in either direction is bad.
//! Steam sits idle in the background for weeks; suspending a three-day sweep
//! because a launcher is open would be its own kind of failure. So the trigger
//! is a game process actually BURNING CPU, sustained past a dwell. Conversely,
//! a game that is running must not be missed because it was not on a list --
//! hence descendants of the Steam client count, whatever they are called.
//!
//! `Timberborn` seeds the list because it is simulation-heavy and will
//! absolutely fight a sweep for cores. The list is the operator's to edit.

use crate::platform::{Pid, Platform};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GameRules {
    /// Matched case-insensitively against a process's basename.
    pub known_games: Vec<String>,
    /// Anything descended from one of these is treated as a game, whatever it
    /// is called -- a launcher's children are the games it launched.
    pub steam_process_names: Vec<String>,
}

impl Default for GameRules {
    fn default() -> Self {
        Self {
            known_games: vec!["Timberborn".into()],
            steam_process_names: vec!["steam_osx".into(), "steamwebhelper".into()],
        }
    }
}

/// One row of the process table, as the sampler already collects it.
#[derive(Debug, Clone, PartialEq)]
pub struct Proc {
    pub pid: Pid,
    pub ppid: Pid,
    /// Full command as reported; matching uses the basename.
    pub command: String,
    pub cpu_pct: f64,
}

fn basename(cmd: &str) -> &str {
    cmd.rsplit('/').next().unwrap_or(cmd)
}

impl GameRules {
    fn is_steam(&self, p: &Proc) -> bool {
        let b = basename(&p.command).to_ascii_lowercase();
        self.steam_process_names
            .iter()
            .any(|n| b.contains(&n.to_ascii_lowercase()))
    }

    fn is_named_game(&self, p: &Proc) -> bool {
        let b = basename(&p.command).to_ascii_lowercase();
        self.known_games
            .iter()
            .any(|n| b.contains(&n.to_ascii_lowercase()))
    }

    /// Every process that counts as a game: named ones, plus everything
    /// descended from the Steam client.
    pub fn game_pids(&self, procs: &[Proc]) -> HashSet<Pid> {
        let mut steam: HashSet<Pid> = procs
            .iter()
            .filter(|p| self.is_steam(p))
            .map(|p| p.pid)
            .collect();
        // Walk descendants to a fixed point, bounded: a cycle in the ppid map
        // (which a racing exit can briefly produce) must not hang the monitor.
        for _ in 0..8 {
            let before = steam.len();
            for p in procs {
                if steam.contains(&p.ppid) {
                    steam.insert(p.pid);
                }
            }
            if steam.len() == before {
                break;
            }
        }
        // The Steam client itself is not a game -- it idles for weeks. Only its
        // descendants are.
        let clients: HashSet<Pid> = procs
            .iter()
            .filter(|p| self.is_steam(p))
            .map(|p| p.pid)
            .collect();
        let mut out: HashSet<Pid> = steam.difference(&clients).copied().collect();
        for p in procs {
            if self.is_named_game(p) {
                out.insert(p.pid);
            }
        }
        out
    }

    /// The busiest game process this tick, if any. The throttle state machine
    /// applies the CPU threshold and the dwell; this only reports.
    pub fn busiest(&self, procs: &[Proc]) -> Option<(String, f64)> {
        let games = self.game_pids(procs);
        let mut best: Option<(&str, f64)> = None;
        for p in procs.iter().filter(|p| games.contains(&p.pid)) {
            // Strictly greater: the FIRST maximum wins on a tie, matching the
            // convention used everywhere else in this port.
            if best.is_none() || best.is_some_and(|(_, c)| p.cpu_pct > c) {
                best = Some((basename(&p.command), p.cpu_pct));
            }
        }
        best.map(|(n, c)| (n.to_string(), c))
    }
}

/// Collect the process table via the platform.
pub fn snapshot<P: Platform>(_plat: &P, ps_output: &str) -> Vec<Proc> {
    // Parsed from `ps -Ao pid,ppid,pcpu,comm`, kept separate from the syscall so
    // the matching rules are testable against captured text.
    let mut out = Vec::new();
    for line in ps_output.lines().skip(1) {
        // ps pads its columns, so fields are separated by RUNS of spaces --
        // and the command is the remainder, because it can contain spaces of
        // its own ("Brave Browser Helper (Renderer)").
        let line = line.trim();
        let mut fields = line.split_whitespace();
        let (Some(pid), Some(ppid), Some(cpu)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let (Ok(pid), Ok(ppid), Ok(cpu)) =
            (pid.parse::<Pid>(), ppid.parse::<Pid>(), cpu.parse::<f64>())
        else {
            continue;
        };
        let Some(cmd) = fields
            .next()
            .and_then(|first| line.find(first).map(|i| &line[i..]))
        else {
            continue;
        };
        out.push(Proc {
            pid,
            ppid,
            command: cmd.trim().to_string(),
            cpu_pct: cpu,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: Pid, ppid: Pid, cmd: &str, cpu: f64) -> Proc {
        Proc {
            pid,
            ppid,
            command: cmd.into(),
            cpu_pct: cpu,
        }
    }

    /// Steam idling in the background is fine, and must not read as a game --
    /// otherwise the launcher alone suspends every overnight sweep.
    #[test]
    fn the_steam_client_itself_is_not_a_game() {
        let r = GameRules::default();
        let procs = vec![p(100, 1, "/Applications/Steam.app/steam_osx", 2.0)];
        assert!(r.game_pids(&procs).is_empty());
        assert!(r.busiest(&procs).is_none());
    }

    /// A game launched through Steam counts even though nobody put its name on
    /// a list -- which is the entire reason descendants are followed.
    #[test]
    fn a_descendant_of_steam_is_a_game_whatever_it_is_called() {
        let r = GameRules::default();
        let procs = vec![
            p(100, 1, "/Applications/Steam.app/steam_osx", 2.0),
            p(200, 100, "/Users/h/Library/.../SomeUnlistedGame", 180.0),
        ];
        let g = r.game_pids(&procs);
        assert!(g.contains(&200));
        assert!(!g.contains(&100));
        assert_eq!(r.busiest(&procs).unwrap().0, "SomeUnlistedGame");
    }

    /// Grandchildren too: launchers commonly spawn a shim that spawns the game.
    #[test]
    fn descendants_are_followed_transitively() {
        let r = GameRules::default();
        let procs = vec![
            p(100, 1, "/x/steam_osx", 1.0),
            p(200, 100, "/x/reaper", 1.0),
            p(300, 200, "/x/TheGame", 300.0),
        ];
        assert!(r.game_pids(&procs).contains(&300));
    }

    /// The seeded name, matched case-insensitively on the basename.
    #[test]
    fn a_named_game_counts_without_steam_anywhere() {
        let r = GameRules::default();
        let procs = vec![p(400, 1, "/Applications/timberborn.app/timberborn", 240.0)];
        assert!(r.game_pids(&procs).contains(&400));
        assert_eq!(r.busiest(&procs).unwrap().1, 240.0);
    }

    /// The sweep's own processes are not games, however hot they run.
    #[test]
    fn the_sweep_itself_is_never_a_game() {
        let r = GameRules::default();
        let procs = vec![
            p(500, 1, "/repo/target/release/ff", 395.0),
            p(501, 1, "/usr/bin/cargo", 300.0),
        ];
        assert!(r.game_pids(&procs).is_empty());
    }

    /// A cycle in the ppid map -- which a racing exit can briefly produce --
    /// must terminate rather than hang the monitor.
    #[test]
    fn a_cyclic_process_table_terminates() {
        let r = GameRules::default();
        let procs = vec![
            p(100, 1, "/x/steam_osx", 1.0),
            p(200, 300, "/x/a", 1.0),
            p(300, 200, "/x/b", 1.0),
        ];
        let _ = r.game_pids(&procs);
    }

    #[test]
    fn the_process_table_parses() {
        let ps = "  PID  PPID %CPU COMM\n\
                  100     1  2.0 /Applications/Steam.app/steam_osx\n\
                  200   100 180.5 /x/Game With Spaces\n";
        let procs = snapshot(&crate::platform::host(), ps);
        assert_eq!(procs.len(), 2);
        assert_eq!(procs[1].cpu_pct, 180.5);
        assert_eq!(procs[1].command, "/x/Game With Spaces");
    }
}
