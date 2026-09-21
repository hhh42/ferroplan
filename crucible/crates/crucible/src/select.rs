//! A SUBSET of a set: the rows a question is actually about.
//!
//! A sweep measures a whole set -- thirty-two boards, eight thousand cells,
//! days of the box. Most questions asked of an engine during a cycle are not
//! that size: "did this lane convert the rows it was built for", "did it cost
//! anything on the boards it was NOT built for", "is this one row a loss or a
//! lucky third attempt". Until this module they were asked OUTSIDE the
//! harness, with a shell loop around the board runner, and so without any of
//! what the harness exists for. The 0.28 lanes were measured that way, beside
//! a game running at 210 % CPU and then a Bevy build at load 23, and nothing
//! in the loop knew: no throttle, no referee, no re-run of a timeout the box
//! caused. The rows were kept, as rows always are, but nobody could say which
//! of them meant anything.
//!
//! So a subset is a SWEEP -- the same runner, the same referee, the same
//! owed-row cascade, the same database -- over fewer cells. Everything that
//! makes a full sweep trustworthy applies unchanged, because nothing about
//! measuring a cell depends on which other cells are being measured.
//!
//! What does depend on it is everything said about a BOARD, and a subset must
//! say none of it:
//!
//! - It stages under `benchmarks/probes/<name>/<engine>/`, never the set's own
//!   stage. A board raw holding a third of its rows, written where the full
//!   one goes, is exactly the artifact `promote` would read.
//! - It records no `board_pass`. That table is the `.done` marker with
//!   provenance, and its live row is unique per (board, engine): a subset that
//!   banked its forty cells would overwrite it with `clean`, and the next
//!   reader would take the board for measured.
//!
//! The ROWS are another matter, and deliberately so. A cell measured under a
//! subset is keyed exactly as a full sweep keys it -- engine hash, board
//! identity, instance -- so the full sweep that follows reads it back and owes
//! one cell fewer. A probe is never wasted work.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Which rows, judged by what the PROMOTED raw says about them -- the
/// predecessor's published row, the same source the scheduler classes a cell
/// from. `Unsolved` is "what is there to gain"; `Solved` is "what is there to
/// lose", the regression read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Prior {
    /// Rows the promoted raw records as unsolved, or does not record at all.
    Unsolved,
    /// Rows the promoted raw records as solved.
    Solved,
}

/// The flags, shared verbatim by `sweep` and `backfill` so a candidate and a
/// tag can be measured over the SAME cells -- which is the whole of an honest
/// differential.
#[derive(clap::Args, Clone, Debug, Default)]
pub struct SelectArgs {
    /// Measure only these boards of the set (repeatable). Turns the run into
    /// a SUBSET: staged under benchmarks/probes/, no board is marked done.
    #[arg(long = "board", value_name = "ID")]
    pub boards: Vec<String>,
    /// Measure only rows whose `variant/label` matches this regex, e.g.
    /// '^tpp-metric-time/(8|9)$' or 'preferences-qualitative'.
    #[arg(long, value_name = "REGEX")]
    pub only: Option<String>,
    /// Measure only the rows listed in this file: one `variant/label` per
    /// line, `#` comments. `crucible compare --lost` writes the format.
    #[arg(long, value_name = "FILE")]
    pub rows: Option<PathBuf>,
    /// Measure only rows the promoted raw records as unsolved (what there is
    /// to gain) or solved (what there is to lose).
    #[arg(long, value_enum)]
    pub prior: Option<Prior>,
    /// Name the subset: its artifacts go to benchmarks/probes/<NAME>/<engine>/.
    /// Defaults to the set's name.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// (`sweep` only) Measure THIS binary instead of <repo>/target/release/ff.
    /// Subsets only: the instrument and the promoted raws come from --repo,
    /// the engine from wherever it was built -- a worktree, a branch, a
    /// bisect. The set's own stage belongs to the working tree's candidate
    /// and no other binary may write there.
    #[arg(long, value_name = "PATH")]
    pub engine: Option<PathBuf>,
}

/// A parsed [`SelectArgs`]. `Select::default()` is the whole set, and a sweep
/// given it behaves exactly as it did before this module existed.
#[derive(Clone, Debug, Default)]
pub struct Select {
    /// The binary to measure instead of the candidate; `None` is the candidate.
    pub engine: Option<PathBuf>,
    name: Option<String>,
    boards: Vec<String>,
    only: Option<regex::Regex>,
    rows: Option<BTreeSet<String>>,
    prior: Option<Prior>,
}

impl Select {
    pub fn from_args(a: &SelectArgs) -> anyhow::Result<Self> {
        let only = match &a.only {
            Some(src) => {
                Some(regex::Regex::new(src).map_err(|e| anyhow::anyhow!("--only {src:?}: {e}"))?)
            }
            None => None,
        };
        let rows = match &a.rows {
            Some(path) => {
                let src = std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("--rows {}: {e}", path.display()))?;
                let set = parse_rows(&src);
                anyhow::ensure!(
                    !set.is_empty(),
                    "--rows {}: no `variant/label` lines in it",
                    path.display()
                );
                Some(set)
            }
            None => None,
        };
        if let Some(n) = &a.name {
            anyhow::ensure!(
                !n.is_empty()
                    && n.chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                    && !n.starts_with('.'),
                "--name {n:?}: letters, digits, '-', '_' and '.' only -- it becomes a directory"
            );
        }
        let sel = Select {
            engine: a.engine.clone(),
            name: a.name.clone(),
            boards: a.boards.clone(),
            only,
            rows,
            prior: a.prior,
        };
        anyhow::ensure!(
            sel.is_subset() || sel.name.is_none(),
            "--name names a SUBSET's staging directory; give it something to select \
             (--board, --only, --rows or --prior)"
        );
        anyhow::ensure!(
            sel.is_subset() || sel.engine.is_none(),
            "--engine measures an arbitrary binary, and only a SUBSET may: the set's own \
             stage belongs to the working tree's candidate (--board, --only, --rows or --prior)"
        );
        Ok(sel)
    }

    /// Does this select anything short of the whole set? Every behaviour a
    /// subset changes hangs off this one question.
    pub fn is_subset(&self) -> bool {
        !self.boards.is_empty()
            || self.only.is_some()
            || self.rows.is_some()
            || self.prior.is_some()
    }

    /// The named boards that the set does not hold -- a typo must stop the
    /// run, not silently measure nothing.
    pub fn unknown_boards<'a>(&'a self, set_boards: &[String]) -> Vec<&'a str> {
        self.boards
            .iter()
            .filter(|b| !set_boards.contains(b))
            .map(String::as_str)
            .collect()
    }

    pub fn wants_board(&self, id: &str) -> bool {
        self.boards.is_empty() || self.boards.iter().any(|b| b == id)
    }

    /// Whether [`Self::admits`] will need the promoted raw read for it.
    pub fn needs_prior(&self) -> bool {
        self.prior.is_some()
    }

    /// Is this cell in the subset? `prior` is the promoted raw's rows for the
    /// cell's board, keyed `variant/label` (`sweep::prior_rows`).
    pub fn admits(
        &self,
        variant: &str,
        label: &str,
        prior: &BTreeMap<String, (bool, Option<f64>)>,
    ) -> bool {
        let key = format!("{variant}/{label}");
        if self.only.as_ref().is_some_and(|re| !re.is_match(&key)) {
            return false;
        }
        if self.rows.as_ref().is_some_and(|rows| !rows.contains(&key)) {
            return false;
        }
        let was_solved = prior.get(&key).is_some_and(|(solved, _)| *solved);
        match self.prior {
            Some(Prior::Unsolved) => !was_solved,
            Some(Prior::Solved) => was_solved,
            None => true,
        }
    }

    /// `benchmarks/probes/<name>/<ver>-<hash>/`: one directory per engine under
    /// one name, so a candidate and the tag it is being read against sit side
    /// by side and neither can be mistaken for a set's stage.
    pub fn stage(&self, repo: &Path, set: &str, engine_ver: &str, engine_hash: &str) -> PathBuf {
        let ver = engine_ver
            .trim()
            .strip_prefix("ff ")
            .unwrap_or(engine_ver.trim());
        repo.join("benchmarks/probes")
            .join(self.name.as_deref().unwrap_or(set))
            .join(format!("{ver}-{engine_hash}"))
    }

    /// One line for the log: what was asked for, in the words it was asked in.
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        if !self.boards.is_empty() {
            parts.push(format!("boards {}", self.boards.join(",")));
        }
        if let Some(re) = &self.only {
            parts.push(format!("only /{}/", re.as_str()));
        }
        if let Some(rows) = &self.rows {
            parts.push(format!("{} listed row(s)", rows.len()));
        }
        match self.prior {
            Some(Prior::Unsolved) => parts.push("prior unsolved".into()),
            Some(Prior::Solved) => parts.push("prior solved".into()),
            None => {}
        }
        parts.join("; ")
    }
}

/// `variant/label` per line; blank lines and `#` comments skipped; a trailing
/// comment after whitespace is dropped. `variant:label` is accepted because
/// that is how a row is usually written in prose.
pub fn parse_rows(src: &str) -> BTreeSet<String> {
    src.lines()
        .filter_map(|l| {
            let l = l.split('#').next().unwrap_or("").trim();
            let l = l.split_whitespace().next()?;
            let (variant, label) = l.rsplit_once('/').or_else(|| l.rsplit_once(':'))?;
            (!variant.is_empty() && !label.is_empty()).then(|| format!("{variant}/{label}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(a: SelectArgs) -> Select {
        Select::from_args(&a).unwrap()
    }

    fn prior(rows: &[(&str, bool)]) -> BTreeMap<String, (bool, Option<f64>)> {
        rows.iter()
            .map(|(k, s)| (k.to_string(), (*s, None)))
            .collect()
    }

    #[test]
    fn the_default_is_the_whole_set() {
        let s = Select::default();
        assert!(!s.is_subset());
        assert!(s.wants_board("anything"));
        assert!(s.admits("v", "1", &BTreeMap::new()));
    }

    #[test]
    fn boards_only_and_rows_intersect() {
        let s = sel(SelectArgs {
            boards: vec!["ipc5-time".into()],
            only: Some("^trucks-time/".into()),
            ..Default::default()
        });
        assert!(s.is_subset());
        assert!(s.wants_board("ipc5-time") && !s.wants_board("ipc5-prop"));
        assert!(s.admits("trucks-time", "7", &BTreeMap::new()));
        assert!(!s.admits("storage-time", "7", &BTreeMap::new()));
        assert_eq!(s.unknown_boards(&["ipc5-prop".into()]), ["ipc5-time"]);
    }

    /// `unsolved` is "what is there to gain", so a row the promoted raw never
    /// recorded counts: a new instance is a gain waiting to be measured, and
    /// dropping it would make the subset quietly smaller than the question.
    #[test]
    fn prior_reads_the_promoted_raw_and_treats_absent_as_unsolved() {
        let p = prior(&[("v/1", true), ("v/2", false)]);
        let gain = sel(SelectArgs {
            prior: Some(Prior::Unsolved),
            ..Default::default()
        });
        assert!(!gain.admits("v", "1", &p));
        assert!(gain.admits("v", "2", &p));
        assert!(gain.admits("v", "3", &p), "never recorded = unsolved");
        let lose = sel(SelectArgs {
            prior: Some(Prior::Solved),
            ..Default::default()
        });
        assert!(lose.admits("v", "1", &p));
        assert!(!lose.admits("v", "2", &p) && !lose.admits("v", "3", &p));
    }

    #[test]
    fn a_rows_file_reads_the_way_rows_are_written() {
        let rows = parse_rows(
            "# lost against the published board\n\
             tpp-metric-time/8\n\
             \n\
             rovers-preferences-qualitative:20   # EHC, 14.6 s\n\
             not-a-row\n",
        );
        assert_eq!(
            rows.into_iter().collect::<Vec<_>>(),
            ["rovers-preferences-qualitative/20", "tpp-metric-time/8"]
        );
    }

    #[test]
    fn a_subset_never_stages_where_a_set_does() {
        let s = sel(SelectArgs {
            boards: vec!["b".into()],
            name: Some("lanes-sit".into()),
            ..Default::default()
        });
        let stage = s.stage(Path::new("/r"), "cut27", "ff 0.27.1", "424843b9cfce");
        assert_eq!(
            stage,
            Path::new("/r/benchmarks/probes/lanes-sit/0.27.1-424843b9cfce")
        );
        // unnamed: the set's name, still under probes/
        let s = sel(SelectArgs {
            boards: vec!["b".into()],
            ..Default::default()
        });
        assert!(s
            .stage(Path::new("/r"), "cut27", "ff 0.27.1", "h")
            .starts_with("/r/benchmarks/probes/cut27"));
    }

    #[test]
    fn an_arbitrary_binary_may_be_probed_but_never_swept() {
        assert!(Select::from_args(&SelectArgs {
            engine: Some("/some/where/ff".into()),
            ..Default::default()
        })
        .is_err());
        let s = sel(SelectArgs {
            engine: Some("/some/where/ff".into()),
            boards: vec!["b".into()],
            ..Default::default()
        });
        assert_eq!(s.engine.as_deref(), Some(Path::new("/some/where/ff")));
    }

    #[test]
    fn a_name_is_a_directory_and_needs_something_to_name() {
        for bad in ["../x", "a/b", "", ".hidden"] {
            let r = Select::from_args(&SelectArgs {
                boards: vec!["b".into()],
                name: Some(bad.into()),
                ..Default::default()
            });
            assert!(r.is_err(), "{bad:?} must be refused");
        }
        assert!(Select::from_args(&SelectArgs {
            name: Some("x".into()),
            ..Default::default()
        })
        .is_err());
    }
}
