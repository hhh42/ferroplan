//! `crucible resident`: the loop the spec's Purpose section asked for --
//! "run continuously, forever, without ever needing to be told what to do
//! next." Every poll it (1) sweeps the working tree's candidate for the
//! named set when the set's stage still owes rows, and (2) backfills each of
//! the newest tags whose stage is not complete, newest first. Both entries
//! resume from the database and return at once when nothing is owed, so an
//! idle cycle costs a version probe and a directory listing. The width
//! policy, the throttle, the referee and the canary do the rest; ^C stops
//! the run in flight with everything banked kept, and the next start picks
//! up where it left off.

use crate::config::Config;
use anyhow::Context;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub struct Opts<'a> {
    pub set: &'a str,
    /// Also sweep the working tree's candidate (`target/release/ff`) for the
    /// set, when its version matches the set's gate.
    pub candidate: bool,
    /// The newest N tags to keep backfilled; the config's `keep_tags` when
    /// unset.
    pub tags: Option<usize>,
    /// One cycle, then exit -- for a cron-shaped operator, or a test.
    pub once: bool,
}

/// Version tags, newest first, by semantic order.
pub fn tags(repo: &Path) -> anyhow::Result<Vec<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["tag", "--list", "v*"])
        .output()
        .context("listing tags")?;
    let mut tags: Vec<(Vec<u64>, String)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|t| {
            let key: Option<Vec<u64>> = t
                .trim_start_matches('v')
                .split('.')
                .map(|p| p.parse::<u64>().ok())
                .collect();
            key.map(|k| (k, t.to_string()))
        })
        .collect();
    tags.sort();
    tags.reverse();
    Ok(tags.into_iter().map(|(_, t)| t).collect())
}

/// Every board of the set has its `.done` marker under `stage`.
fn set_done(repo: &Path, boards: &[String], stage: &Path) -> bool {
    let stage = if stage.is_absolute() {
        stage.to_path_buf()
    } else {
        repo.join(stage)
    };
    boards
        .iter()
        .all(|b| stage.join(format!("{b}.done")).exists())
}

pub fn run(repo: &Path, cfg: &Config, o: Opts<'_>) -> anyhow::Result<()> {
    let manifest = crate::load_manifest(repo)?;
    let set = manifest
        .set(o.set)
        .with_context(|| format!("no set {:?} in the manifest", o.set))?
        .clone();
    let n_tags = o.tags.unwrap_or(cfg.repo.keep_tags);
    let poll = Duration::from_secs(cfg.repo.tag_poll_secs.max(30));
    crucible_core::exec::install_interrupt_handler();
    println!(
        "resident: set {} -- candidate {}, newest {n_tags} tag(s), polling every {}s",
        set.name,
        if o.candidate { "yes" } else { "no" },
        poll.as_secs()
    );
    loop {
        if crucible_core::exec::interrupted() {
            println!("resident: stopped");
            return Ok(());
        }
        let mut did = false;

        // 1. The candidate, when its stage still owes.
        if o.candidate {
            let stage = Path::new(&set.stage);
            if set_done(repo, &set.boards, stage) {
                println!("resident: candidate stage {} complete", set.stage);
            } else {
                match crate::repo::Engine::probe(&crate::repo::candidate_path(repo)) {
                    Ok(engine) => {
                        let gate_ok = set
                            .requires_version
                            .as_deref()
                            .map_or(true, |w| engine.require_version(w).is_ok());
                        if gate_ok {
                            println!(
                                "resident: sweeping the candidate {} for {}",
                                engine.ver, set.name
                            );
                            did = true;
                            if let Err(e) = crate::sweep::run(
                                repo,
                                cfg,
                                crate::sweep::Opts {
                                    set: &set.name,
                                    require_version: None,
                                    headless: true,
                                    quiet_only: false,
                                    dry_run: false,
                                    max_passes: None,
                                    no_db: false,
                                },
                            ) {
                                eprintln!("resident: candidate sweep failed: {e:#}");
                            }
                        } else {
                            println!(
                                "resident: candidate is {} and the set wants {}; skipping",
                                engine.ver,
                                set.requires_version.as_deref().unwrap_or("-")
                            );
                        }
                    }
                    Err(e) => println!("resident: no candidate to sweep ({e})"),
                }
            }
        }
        if crucible_core::exec::interrupted() {
            println!("resident: stopped");
            return Ok(());
        }

        // 2. The newest tags, newest first, each until its stage is complete.
        let tags = tags(repo).unwrap_or_default();
        for tag in tags.iter().take(n_tags) {
            if crucible_core::exec::interrupted() {
                println!("resident: stopped");
                return Ok(());
            }
            let ver = tag.trim_start_matches('v');
            let stage = crate::backfill::stage_for(ver);
            if set_done(repo, &set.boards, &stage) {
                continue;
            }
            println!(
                "resident: backfilling {tag} for {} into {}",
                set.name,
                stage.display()
            );
            did = true;
            if let Err(e) = crate::backfill::run(
                repo,
                cfg,
                crate::backfill::Opts {
                    tag,
                    set: &set.name,
                    stage: None,
                    dry_run: false,
                    max_passes: None,
                    no_db: false,
                },
            ) {
                eprintln!("resident: backfill of {tag} failed: {e:#}");
            }
        }

        if o.once {
            println!(
                "resident: one cycle done{}",
                if did { "" } else { " -- nothing owed" }
            );
            return Ok(());
        }
        if !did {
            println!("resident: nothing owed; next look in {}s", poll.as_secs());
        }
        // Sleep in slices so ^C is honoured promptly.
        let until = std::time::Instant::now() + poll;
        while std::time::Instant::now() < until {
            if crucible_core::exec::interrupted() {
                println!("resident: stopped");
                return Ok(());
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tags come back newest first by SEMANTIC order, and anything that is
    /// not vX.Y.Z is ignored.
    #[test]
    fn tags_are_semantic_and_newest_first() {
        let dir = std::env::temp_dir().join(format!("crucible-resident-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            assert!(Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap()
                .status
                .success());
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "x",
        ]);
        for t in ["v0.9.0", "v0.10.0", "v0.2.3", "junk", "v0.10.1"] {
            git(&["tag", t]);
        }
        assert_eq!(
            tags(&dir).unwrap(),
            vec!["v0.10.1", "v0.10.0", "v0.9.0", "v0.2.3"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_set_is_done_when_every_board_has_its_marker() {
        let dir = std::env::temp_dir().join(format!("crucible-done-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("stage")).unwrap();
        let boards = vec!["a".to_string(), "b".to_string()];
        assert!(!set_done(&dir, &boards, Path::new("stage")));
        std::fs::write(dir.join("stage/a.done"), "").unwrap();
        assert!(!set_done(&dir, &boards, Path::new("stage")));
        std::fs::write(dir.join("stage/b.done"), "").unwrap();
        assert!(set_done(&dir, &boards, Path::new("stage")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
