//! Which planner is being measured, and how to get another one.
//!
//! THE ENGINE'S IDENTITY IS ITS HASH, NOT ITS VERSION STRING. This is the one
//! place crucible deliberately does something the Python does not.
//! `PER-INSTANCE-RETRY.md` names the risk exactly -- "a stitched board must
//! never mix rows from two different `ff` builds" -- and then gates reuse on
//! the `ff --version` string, adding "probably also the git SHA if the binary
//! carries one". It does not. Every dev build of a cycle reports `ff 0.25.0`,
//! so today two different 0.25.0 builds stitch together silently. Under a
//! candidate-driven trigger, where the working-tree binary is rebuilt many
//! times a day, that is the likeliest way this harness would produce a
//! chimeric board. So the resume gate compares BLAKE3, and `ver` is still
//! written into every row for artifact compatibility.
//!
//! THE TRIGGER RUNS THE OTHER WAY FROM THE SPEC. `crucible-spec.md` §4 polls
//! for new tags and sweeps them. But this project sweeps the cut CANDIDATE --
//! an unreleased 0.N.0 working-tree build -- and tags only afterwards, at
//! publish. `cut25-sweeps.sh` refuses to start unless `ff --version` already
//! reports the candidate version. A tag-triggered harness could only ever
//! re-verify history, never gate a cut. So the candidate is the primary
//! trigger and tags are the BACKFILL path.

use std::path::{Path, PathBuf};

/// A specific planner binary, identified by content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Engine {
    pub path: PathBuf,
    /// Exactly `ff --version`, trimmed. NOT unique across dev builds.
    pub ver: String,
    /// The identity the resume gate actually compares.
    pub blake3: String,
    /// Which `--mode` values this build accepts. An old tag that predates
    /// `Mode::Optimal` must SKIP the proof boards, not record zero coverage --
    /// "the feature does not exist, and recording a zero would be a lie the
    /// standings would then average."
    pub modes: Vec<String>,
    pub tag: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("no planner at {0}; build it with `cargo build --release -p ferroplan-cli`")]
    NoBinary(PathBuf),
    #[error("{path} did not report a version: {detail}")]
    NoVersion { path: PathBuf, detail: String },
    #[error(
        "binary reports {found:?} but this sweep is for {want:?} -- build the \
         cut candidate first"
    )]
    WrongVersion { found: String, want: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl Engine {
    /// Identify the binary at `path`.
    pub fn probe(path: &Path) -> Result<Engine, RepoError> {
        if !path.exists() {
            return Err(RepoError::NoBinary(path.to_path_buf()));
        }
        let out = std::process::Command::new(path).arg("--version").output()?;
        let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if ver.is_empty() {
            return Err(RepoError::NoVersion {
                path: path.to_path_buf(),
                detail: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            });
        }
        let bytes = std::fs::read(path)?;
        let blake3 = blake3::hash(&bytes).to_hex().to_string();
        Ok(Engine {
            path: path.to_path_buf(),
            ver,
            blake3,
            modes: probe_modes(path),
            tag: None,
        })
    }

    /// The version gate every sweep driver opens with: `case "$V" in *0.25*)`.
    /// A substring test, deliberately -- the binary reports `ff 0.25.0` and the
    /// cycle is named `0.25`.
    pub fn require_version(&self, want: &str) -> Result<(), RepoError> {
        if self.ver.contains(want) {
            Ok(())
        } else {
            Err(RepoError::WrongVersion {
                found: self.ver.clone(),
                want: want.to_string(),
            })
        }
    }

    /// Can this build run the board at all?
    ///
    /// Returning false must lead to a board recorded as `feature-absent` with
    /// ZERO rows written -- never a board of zeroes.
    pub fn supports_mode(&self, mode: &str) -> bool {
        if mode.is_empty() || mode == "auto" {
            return true;
        }
        // An empty probe means `--help` told us nothing; assume support rather
        // than silently skipping every board on an unfamiliar build.
        self.modes.is_empty() || self.modes.iter().any(|m| m == mode)
    }

    pub fn short_hash(&self) -> String {
        self.blake3.chars().take(12).collect()
    }
}

/// Read the accepted `--mode` values out of the binary's own help text, which
/// clap renders as a value list. Reading the binary rather than the source is
/// what makes this work against a tag whose tree is not checked out.
fn probe_modes(path: &Path) -> Vec<String> {
    let Ok(out) = std::process::Command::new(path).arg("--help").output() else {
        return Vec::new();
    };
    modes_from_help(&String::from_utf8_lossy(&out.stdout))
}

/// The three shapes clap renders a value enum in, all read (0.26 F6 Part 4:
/// the cut26 enumeration gate found the 0.26 binary "without --mode optimal"
/// because a variant doc comment had switched `--help` to the long form,
/// and every proof board was skipped):
/// - `--mode <auto|ff|optimal>` — the inline placeholder;
/// - `--mode <MODE> ... [possible values: auto, ff, optimal]` — short help;
/// - `--mode <MODE>` followed by a `Possible values:` block of `- name` /
///   `- name: doc` bullets — long help, once any variant carries a doc.
fn modes_from_help(help: &str) -> Vec<String> {
    let mut lines = help.lines();
    let Some(line) = lines.find(|l| l.contains("--mode")) else {
        return Vec::new();
    };
    let words = |s: &str, sep: char| -> Vec<String> {
        s.split(sep)
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect()
    };
    if let Some(rest) = line.split("[possible values:").nth(1) {
        if let Some(end) = rest.find(']') {
            return words(&rest[..end], ',');
        }
    }
    if let (Some(a), Some(b)) = (line.find('<'), line.rfind('>')) {
        let inner = &line[a + 1..b];
        if b > a && inner.contains('|') {
            return words(inner, '|');
        }
    }
    // Long help: the block starts at "Possible values:" and ends at the
    // first line that is neither blank nor a bullet.
    let mut modes = Vec::new();
    let mut in_block = false;
    for l in lines {
        let t = l.trim();
        if !in_block {
            if t.starts_with("Possible values:") {
                in_block = true;
            } else if t.starts_with("--") || t.starts_with('-') && t.len() == 2 {
                break; // the next option: no block for --mode
            }
            continue;
        }
        if t.is_empty() {
            continue;
        }
        let Some(item) = t.strip_prefix("- ") else {
            break;
        };
        let name = item.split(':').next().unwrap_or("").trim();
        if !name.is_empty() {
            modes.push(name.to_string());
        }
    }
    modes
}

/// Where the candidate binary lives in a ferroplan checkout.
pub fn candidate_path(repo: &Path) -> PathBuf {
    repo.join("target/release/ff")
}

/// The instrument is ALWAYS the working tree's; only the ENGINE comes from the
/// tag.
///
/// NOT YET DRIVEN: `crucible backfill` is unbuilt. The rule and its test live
/// here because the rule is the load-bearing part and it must not be
/// rediscovered when the command is written.
///
/// `backfill-air.sh` states the rule and it is right: checking out the old
/// `benchmarks/` too "would vary the INSTRUMENT as well as the engine, and then
/// the delta means nothing." So a backfill builds an old binary in a detached
/// worktree and points the CURRENT harness at it -- which is exactly what
/// `$FERROPLAN_FF` does today.
#[allow(dead_code)] // backfill is not built yet; see the note above
pub fn worktree_for(worktree_dir: &Path, tag: &str) -> PathBuf {
    // A distinct prefix from the operator's hand-made `~/ferroplan-backfill-*`
    // checkouts, so worktree garbage collection can never eat one of those.
    worktree_dir.join(tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(ver: &str, modes: &[&str]) -> Engine {
        Engine {
            path: PathBuf::from("/x/ff"),
            ver: ver.into(),
            blake3: "a".repeat(64),
            modes: modes.iter().map(|s| s.to_string()).collect(),
            tag: None,
        }
    }

    /// The gate every sweep driver opens with.
    #[test]
    fn the_version_gate_is_a_substring_test() {
        let e = engine("ff 0.25.0", &[]);
        assert!(e.require_version("0.25").is_ok());
        assert!(e.require_version("0.24").is_err());
    }

    /// THE POINT OF THIS MODULE: two builds of the same version are NOT the
    /// same engine, and only the hash can tell them apart.
    #[test]
    fn two_builds_of_one_version_are_different_engines() {
        let mut a = engine("ff 0.25.0", &[]);
        let mut b = engine("ff 0.25.0", &[]);
        b.blake3 = "b".repeat(64);
        assert_eq!(a.ver, b.ver, "the version string cannot tell them apart");
        assert_ne!(a.blake3, b.blake3, "the hash can");
        a.blake3 = b.blake3.clone();
        assert_eq!(a, b);
    }

    /// A tag that predates Mode::Optimal must SKIP its proof boards. Recording
    /// zero coverage would be a lie the standings would then average.
    #[test]
    fn an_old_engine_without_optimal_does_not_claim_zero() {
        let old = engine("ff 0.18.0", &["auto", "ff", "temporal"]);
        assert!(!old.supports_mode("optimal"));
        assert!(old.supports_mode("auto"));
        let new = engine("ff 0.25.0", &["auto", "optimal", "sat"]);
        assert!(new.supports_mode("optimal"));
    }

    /// All three clap shapes read the same list — the long-help block is the
    /// one the 0.26 binary renders, and the one the first probe missed.
    #[test]
    fn every_clap_help_shape_yields_the_modes() {
        let inline = "Options:\n      --mode <auto|ff|optimal>  Planning mode\n";
        let short = "      --mode <MODE>          Planning mode [default: auto] \
                     [possible values: auto, ff, optimal, sat]\n      --search <SEARCH>\n";
        let long = concat!(
            "      --mode <MODE>\n",
            "          Planning mode (`auto` routes)\n",
            "\n",
            "          Possible values:\n",
            "          - auto\n",
            "          - ff\n",
            "          - optimal:   Sequential-optimal: A* + admissible h^max\n",
            "          - sat:       Bounded-layer SAT compilation\n",
            "\n",
            "      --search <SEARCH>\n",
            "          Search strategy\n",
        );
        assert_eq!(modes_from_help(inline), ["auto", "ff", "optimal"]);
        assert_eq!(modes_from_help(short), ["auto", "ff", "optimal", "sat"]);
        assert_eq!(modes_from_help(long), ["auto", "ff", "optimal", "sat"]);
        assert!(modes_from_help("no such flag here\n").is_empty());
    }

    /// An unreadable help text must not silently skip every board.
    #[test]
    fn an_unprobeable_engine_is_assumed_capable() {
        assert!(engine("ff 0.9.0", &[]).supports_mode("optimal"));
    }

    /// Crucible's worktrees must never collide with the operator's hand-made
    /// `~/ferroplan-backfill-*` checkouts, or garbage collection eats one.
    #[test]
    fn worktrees_live_under_their_own_prefix() {
        let p = worktree_for(Path::new("/home/h/.crucible/worktrees"), "v0.19.0");
        assert!(p.starts_with("/home/h/.crucible/worktrees"));
        assert!(!p.to_string_lossy().contains("ferroplan-backfill"));
    }

    /// Probing a real binary end to end: any executable that answers
    /// --version will do, and /bin/echo is guaranteed present.
    #[test]
    fn probing_a_real_binary_yields_a_content_hash() {
        let e = Engine::probe(Path::new("/bin/echo")).unwrap();
        assert_eq!(e.blake3.len(), 64);
        assert_eq!(e.short_hash().len(), 12);
        // Same bytes, same identity -- the property the resume gate rests on.
        let again = Engine::probe(Path::new("/bin/echo")).unwrap();
        assert_eq!(e.blake3, again.blake3);
    }

    #[test]
    fn a_missing_binary_is_a_named_error() {
        let err = Engine::probe(Path::new("/nonexistent/ff")).unwrap_err();
        assert!(matches!(err, RepoError::NoBinary(_)));
    }
}
