//! The three published documents, and the one bundle of inputs they all read.
//!
//! `benchmarks/ipc-standings.md`, `STANDINGS.md` and the marked block inside
//! `README.md` are the only numbers this project publishes about itself. They
//! are generated from one sweep's raws by `benchmarks/standings.py`, and this
//! module is the port of the three functions that write them (`main` :761,
//! `write_summary` :583, `_patch_readme` :703).
//!
//! Two rules shape everything below, and both are incidents rather than taste.
//!
//! **A renderer returns a `String`; it never opens a file for writing.** The
//! Python says so about itself: "a bare run overwrites ipc-standings.md in
//! place, and on a box holding only some of the raws that is a destructive
//! act" (:56-58). A box with half the raws renders half a table, and the half
//! it drops does not read as missing -- it reads as *never measured*. Handing
//! the caller a `String` means the decision to overwrite is always somebody's,
//! never a side effect of asking what the numbers are.
//!
//! **A board counts only when its raw AND its `.md` sibling both exist.**
//! `ipc67.py` writes the scoreboard at sweep END, so a lone JSONL is a sweep
//! still in flight. Reading one as a finished board publishes a partial sweep
//! as a result. That gate lives in [`RenderCtx::load`] and nowhere else, so no
//! renderer can forget it.
//!
//! [`RenderCtx`] carries what the renderers cannot compute for themselves --
//! the manifest, the rows, the referee, the field archives, the history, the
//! version and the box -- and nothing they can. Every derived number
//! (coverage, quality, deltas, placements) is computed by the modules that own
//! its rules, and the renderers only decide where the text goes.

pub mod detail;
pub mod readme;
pub mod summary;

use std::path::Path;

use crate::archive::{ArchiveError, Ipc5Archive};
use crate::bounds::BestKnownBounds;
use crate::class::Coverage;
use crate::field::FieldBook;
use crate::fmt::glyph;
use crate::history::{BoxId, ComparablePredecessor, History, VersionKey};
use crate::manifest::{Manifest, ManifestError};
use crate::raw::RawRow;
use crate::referee::{Referee, ValUnavailable};

/// The box the Python defaults to when `$FERROPLAN_BOX` is unset
/// (`write_summary` :605).
///
/// Named here, read nowhere: this crate does not touch the environment, so a
/// caller passes the box in and a test can pass a different one. The default
/// matters because it is the box every committed number in the repository was
/// measured on.
pub const DEFAULT_BOX: &str = "m5-air";

/// The two absent-board texts. Which one renders is a claim about history, not
/// a formatting choice -- see [`RenderCtx::absent_cell`].
const PENDING_TEXT: &str = "sweep in flight / not yet run";
const CLOUD_ERA_TEXT: &str = "cloud-era board, NOT re-baselined — see git history";

/// Anything that stopped [`RenderCtx::load`] assembling a context.
///
/// Deliberately short. Every *optional* input (the archive, the bounds, the
/// field cohorts, the history, the VAL-unavailable map) degrades to empty the
/// way the Python's `os.path.exists` guards do, because on a clean clone they
/// are all absent and the table must still render. Only a missing or unreadable
/// manifest, a corrupt archive, and an unparseable raw are errors: those three
/// are the ones where continuing would publish a number rather than omit one.
#[derive(Debug, thiserror::Error)]
pub enum CtxError {
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    /// A raw that exists, has its `.md` sibling, and will not parse. Python
    /// raises here too; the difference is that this names the file and line.
    #[error("{0}")]
    Rows(String),
}

/// One board as the renderers see it: its identity, the budget its rows are
/// scored against, and either the rows or the fact of their absence.
#[derive(Debug, Clone)]
pub struct BoardRows {
    pub id: String,
    pub label: String,
    /// The budget a row is SCORED against (`budget_secs`), which is not always
    /// the wall the sweep armed. A row measured since 0.23 carries its own
    /// stamp and that stamp wins; this is the fallback for everything older.
    pub budget_secs: f64,
    /// `None` is the ABSENT case, and it is one state with two meanings. See
    /// [`RenderCtx::absent_cell`].
    pub rows: Option<Vec<RawRow>>,
}

/// Everything the three documents are rendered from.
///
/// Assembled once, read many times. The renderers take `&RenderCtx` and return
/// `String`, which is the whole of their contract with the world.
#[derive(Debug)]
pub struct RenderCtx {
    pub manifest: Manifest,
    /// In MANIFEST FILE ORDER, which is the Python `SWEEPS` dict's insertion
    /// order. Not incidental: `write_summary` sorts this list by percentage
    /// with a STABLE sort, so file order is the tie-break that decides which of
    /// two boards on the same percentage prints first.
    pub boards: Vec<BoardRows>,
    pub referee: Referee,
    pub archive: Ipc5Archive,
    pub bounds: BestKnownBounds,
    pub field: FieldBook,
    pub history: History,
    /// The workspace version, so the delta column never compares a release to
    /// ITSELF. `None` where `Cargo.toml` could not be read, which yields no
    /// predecessor and renders every row as a baseline.
    pub version: Option<String>,
    pub box_id: BoxId,
}

impl RenderCtx {
    /// Assemble from parts already in hand. Pure -- no filesystem, no clock.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        manifest: Manifest,
        boards: Vec<BoardRows>,
        referee: Referee,
        archive: Ipc5Archive,
        bounds: BestKnownBounds,
        field: FieldBook,
        history: History,
        version: Option<String>,
        box_id: BoxId,
    ) -> Self {
        Self {
            manifest,
            boards,
            referee,
            archive,
            bounds,
            field,
            history,
            version,
            box_id,
        }
    }

    /// Read a whole repository into a context: the manifest, every board's
    /// rows, and the five optional side-inputs.
    ///
    /// `root` is the repository root (the directory holding `Cargo.toml`,
    /// `README.md` and `benchmarks/`).
    ///
    /// THE PRESENCE GATE IS HERE. A board is `Some(rows)` only when its raw
    /// `.jsonl` and its `.md` scoreboard both exist. `ipc67.py` writes the
    /// `.md` at sweep end, so a lone JSONL is a sweep in flight -- and a sweep
    /// in flight rendered as a finished board publishes a partial measurement
    /// as a result.
    pub fn load(root: &Path, box_id: BoxId) -> Result<Self, CtxError> {
        let benchmarks = root.join("benchmarks");
        let manifest = Manifest::load(&benchmarks.join("manifest.toml"))?;

        let mut boards = Vec::with_capacity(manifest.boards.len());
        for b in &manifest.boards {
            let raw = benchmarks.join(&b.raw);
            let md = benchmarks.join(&b.md);
            // Both, not either. `exists()` answers false for an unreadable
            // path too, which is the same degradation Python's
            // `os.path.exists` performs.
            let rows = if raw.exists() && md.exists() {
                let text = std::fs::read_to_string(&raw)
                    .map_err(|e| CtxError::Rows(format!("{}: {e}", raw.display())))?;
                Some(crate::parse_rows(&text, &raw.display().to_string()).map_err(CtxError::Rows)?)
            } else {
                None
            };
            boards.push(BoardRows {
                id: b.id.clone(),
                label: b.label.clone(),
                budget_secs: b.budget_secs,
                rows,
            });
        }

        Ok(Self::new(
            manifest,
            boards,
            Referee::new(load_val_unavailable(
                &benchmarks.join("val-unavailable.json"),
            )),
            Ipc5Archive::open(&benchmarks.join("IPC5-results.tgz"))?,
            BestKnownBounds::load(&benchmarks.join(".ipc-corpus")),
            FieldBook::load(&benchmarks),
            History::load(&benchmarks.join("standings-history.json")),
            crate::history::current_version(root),
            box_id,
        ))
    }

    /// The board publishing under this label, present or not.
    pub fn board(&self, label: &str) -> Option<&BoardRows> {
        self.boards.iter().find(|b| b.label == label)
    }

    /// Python's `data.get(label)`: the rows and their budget, or nothing at all
    /// for an absent board. The two absences -- no such label, and a label
    /// whose sweep has not landed -- collapse to one answer here exactly as
    /// they do in the Python, because the renderer treats them identically.
    pub fn data(&self, label: &str) -> Option<(&[RawRow], f64)> {
        let b = self.board(label)?;
        Some((b.rows.as_deref()?, b.budget_secs))
    }

    /// Port of `split_rows` :866 -- one raw, sliced by each row's own `ipc`.
    ///
    /// `seq-sat`, `tempo-sat` and `seq-opt` are swept ONCE across both
    /// competitions and appear TWICE in the detail table, under IPC-6 and
    /// under IPC-7. An EMPTY slice is `None`, not an empty board: Python's
    /// `return (sub, budget) if sub else None` sends it down the absent branch,
    /// which is right -- a competition with no rows in a shared raw was not
    /// measured at zero, it was not measured.
    pub fn split(&self, label: &str, ipc: &str) -> Option<(Vec<RawRow>, f64)> {
        let (rows, budget) = self.data(label)?;
        let sub: Vec<RawRow> = rows
            .iter()
            .filter(|r| r.ipc.as_deref() == Some(ipc))
            .cloned()
            .collect();
        if sub.is_empty() {
            None
        } else {
            Some((sub, budget))
        }
    }

    /// Coverage and the failure-class histogram for a slice of rows.
    pub fn coverage_of(&self, rows: &[RawRow], budget: f64) -> Coverage {
        self.referee.coverage(rows, budget)
    }

    /// What an absent board's `entered` cell says -- and the two answers are
    /// not interchangeable.
    ///
    /// A board this box has produced before has a raw that has not landed yet:
    /// the sweep is in flight. A board this box has NEVER produced is a
    /// cloud-era board whose numbers were measured on other silicon and live in
    /// git history. Rendering both as "not swept" would claim we had never
    /// measured them at all, which is a false statement about the record.
    ///
    /// Python asks a frozen `AIR_REBASELINED` set; this asks the manifest's
    /// `rebaselined_on` for the box actually being rendered, which is the same
    /// answer on `m5-air` and the right one after the next hardware move.
    /// Every label in the manifest is currently rebaselined on `m5-air`, so the
    /// cloud-era arm is unreachable from live data -- and therefore has a
    /// dedicated test, because untested dead code re-arms silently.
    pub fn absent_cell(&self, label: &str) -> &'static str {
        if self.manifest.rebaselined_on(label, self.box_id.as_str()) {
            PENDING_TEXT
        } else {
            CLOUD_ERA_TEXT
        }
    }

    /// The proof-track mark, with its leading space -- `" ⚖️"` or nothing.
    ///
    /// Coverage on a proof track IS proof rate: 45% there is a categorically
    /// different claim from 45% on a satisficing board, and the mark is the
    /// only thing in the row that says so. Built from [`glyph::SCALES`], which
    /// is TWO codepoints; a bare U+2696 renders almost identically and diffs
    /// against every committed table.
    pub fn proof_mark(&self, label: &str) -> String {
        if self.manifest.is_proof_track(label) {
            format!(" {}", glyph::SCALES)
        } else {
            String::new()
        }
    }

    /// The release this table's deltas may be quoted against, if any.
    ///
    /// Same box, strictly lower version, highest such -- all three enforced by
    /// [`History::comparable_predecessor`], which is the only constructor for
    /// the returned type.
    pub fn predecessor(&self) -> Option<ComparablePredecessor<'_>> {
        // An unreadable workspace version parses to the same floor Python's
        // `except ValueError: return (0,)` produces, and nothing sorts below
        // it -- so a missing version yields no predecessor and every row reads
        // as a baseline, rather than silently comparing a release to itself.
        let cur = VersionKey::parse(self.version.as_deref().unwrap_or(""));
        self.history.comparable_predecessor(&self.box_id, &cur)
    }

    /// The `write_summary` :617-634 pass: split every board into live, pending
    /// and cloud-era, then sort the live ones.
    ///
    /// Shared by `STANDINGS.md` and the README block so the front page and the
    /// page behind it cannot disagree about which boards are in the headline.
    pub fn standings(&self) -> Standings<'_> {
        let mut live: Vec<LiveBoard<'_>> = Vec::new();
        let mut pending: Vec<&str> = Vec::new();
        let mut cloud: Vec<&str> = Vec::new();

        for b in &self.boards {
            let Some(rows) = b.rows.as_deref() else {
                // Two very different absences, and collapsing them would let a
                // half-promoted sweep read as "we never re-measured this".
                if self.manifest.rebaselined_on(&b.label, self.box_id.as_str()) {
                    pending.push(&b.label);
                } else {
                    cloud.push(&b.label);
                }
                continue;
            };
            let cov = self.coverage_of(rows, b.budget_secs);
            // `if not n: continue` -- a board with no rows has no share, and
            // taking one would divide by zero. It is also not "0%": it is
            // nothing to say.
            if cov.total == 0 {
                continue;
            }
            live.push(LiveBoard {
                label: &b.label,
                solved: cov.solved,
                total: cov.total,
                pct: crate::fmt::pct(cov.solved, cov.total),
                rows,
            });
        }

        // Python: `live.sort(key=lambda r: -r[3])` -- descending percentage,
        // STABLE, so boards that tie keep manifest order. `sort_by`, never
        // `sort_unstable_by`: at 92%/87%/86%/85%/82% today nothing ties, but a
        // tie reordered by an unstable sort is a silent diff in a published
        // table with no number wrong to point at.
        live.sort_by(|a, b| b.pct.total_cmp(&a.pct));

        let total_solved = live.iter().map(|r| r.solved).sum();
        let total_rows = live.iter().map(|r| r.total).sum();
        let proofs = live
            .iter()
            .filter(|r| self.manifest.is_proof_track(r.label))
            .map(|r| r.solved)
            .sum();

        Standings {
            live,
            pending,
            cloud,
            total_solved,
            total_rows,
            proofs,
        }
    }
}

/// One board in the at-a-glance view: coverage, its share, and the rows the
/// vs-field column needs to re-slice.
#[derive(Debug, Clone, Copy)]
pub struct LiveBoard<'c> {
    pub label: &'c str,
    pub solved: usize,
    pub total: usize,
    pub pct: f64,
    pub rows: &'c [RawRow],
}

/// The at-a-glance split of every board, and the three headline numbers.
#[derive(Debug, Clone)]
pub struct Standings<'c> {
    /// Descending by percentage, ties in manifest order.
    pub live: Vec<LiveBoard<'c>>,
    /// Absent, but this box has produced them before: a sweep in flight.
    /// In manifest order; the renderer sorts.
    pub pending: Vec<&'c str>,
    /// Absent and never re-baselined on this box. EXCLUDED from the headline
    /// total on purpose -- the old numbers are not comparable to the ones above
    /// them, and averaging the two would launder a hardware change into a
    /// result.
    pub cloud: Vec<&'c str>,
    pub total_solved: usize,
    pub total_rows: usize,
    /// Solved rows on proof tracks: certified optima, not plans.
    pub proofs: usize,
}

/// Read `benchmarks/val-unavailable.json` into the referee's exemption set.
///
/// Python's `_load_val_unavailable` :211: the KEYS of the `unavailable` object,
/// and a missing file is an empty set. Every other failure degrades the same
/// way, because the alternative -- refusing to render -- is worse than
/// rendering without an exemption that the boards beside this table also apply.
/// (When it is missing, `val: false` rows on domains VAL cannot ingest are
/// counted as failures: the table reads 46/240 where the board says 53. That is
/// visible. A crash on a clean clone is not more honest, only louder.)
fn load_val_unavailable(path: &Path) -> ValUnavailable {
    let Ok(src) = std::fs::read_to_string(path) else {
        return ValUnavailable::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&src) else {
        return ValUnavailable::default();
    };
    match v.get("unavailable").and_then(|u| u.as_object()) {
        Some(o) => ValUnavailable::new(o.keys().cloned()),
        None => ValUnavailable::default(),
    }
}

/// Contexts the sibling renderers' unit tests need, for the states live data
/// cannot reach.
///
/// A box that holds no raws at all is one of them: it is the clean-clone case,
/// and every branch that suppresses output rather than publishing an empty
/// claim (no headline, no bands, no README block) is only reachable from here.
#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    /// The preamble every hermetic manifest below needs and none of them is
    /// about.
    pub(crate) const PREAMBLE: &str = r#"
schema = 1
[corpus]
root = ".ipc-corpus"
domain_shared = "domain.pddl"
domain_per_instance = "domains/domain-{first}.pddl"
[defaults]
timeout_secs = 60
jobs = 2
threads = 1
mode = "auto"
mem_gb = 6.0
[track.t]
ipcs = ["ipc-2008"]
include = "x"
"#;

    /// A context over `manifest_src` in which EVERY board is absent, viewed
    /// from `box_`.
    ///
    /// Absent-board rendering is most of what needs proving and none of what
    /// live data can reach: on this repository every board is rebaselined on
    /// `m5-air` and most have landed, so the cloud-era arm and several
    /// in-flight rows are only reachable from a manifest built here.
    pub(crate) fn absent_ctx(manifest_src: &str, box_: &str) -> RenderCtx {
        let manifest = Manifest::parse(manifest_src, "<test>").expect("the manifest parses");
        let boards = manifest
            .boards
            .iter()
            .map(|b| BoardRows {
                id: b.id.clone(),
                label: b.label.clone(),
                budget_secs: b.budget_secs,
                rows: None,
            })
            .collect();
        RenderCtx::new(
            manifest,
            boards,
            Referee::default(),
            Ipc5Archive::default(),
            BestKnownBounds::default(),
            FieldBook::default(),
            History::default(),
            Some("0.25.0".to_string()),
            BoxId::new(box_),
        )
    }

    /// The smallest manifest that parses, with no boards at all -- so every
    /// band is empty, the headline is suppressed and the README block declines.
    pub(crate) fn empty_ctx() -> RenderCtx {
        absent_ctx(PREAMBLE, DEFAULT_BOX)
    }

    /// One `[[board]]` stanza, so a test can name the two fields it is about
    /// (the label and whether this box has ever swept it) and nothing else.
    pub(crate) fn board_stanza(id: &str, label: &str, rebaselined_on: &[&str]) -> String {
        let boxes = rebaselined_on
            .iter()
            .map(|b| format!("\"{b}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "\n[[board]]\nid = \"{id}\"\nraw = \"{id}.jsonl\"\nmd = \"{id}.md\"\n\
             label = \"{label}\"\ncompetition = \"ipc67\"\nbudget_secs = 60\n\
             track = \"t\"\nrebaselined_on = [{boxes}]\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::{absent_ctx, board_stanza, PREAMBLE};
    use super::*;

    /// A manifest with one board on each side of the rebaseline line, so both
    /// arms of `absent_cell` are exercised. The cloud-era arm is unreachable
    /// from live data today -- every board is rebaselined on `m5-air` -- and
    /// this is the only thing standing between it and untested dead code that
    /// re-arms at the next hardware move.
    fn two_sided() -> String {
        format!(
            "{PREAMBLE}{}{}",
            board_stanza("swept", "on this box", &["m5-air"]),
            board_stanza("ghost", "never re-baselined", &[]),
        )
    }

    fn ctx_from(manifest_src: &str, box_: &str) -> RenderCtx {
        absent_ctx(manifest_src, box_)
    }

    /// Defends the incident the two texts exist for: an absent board that this
    /// box HAS produced is a sweep in flight, and one it has never produced is
    /// a cloud-era board whose record is in git. Rendering both as "not swept"
    /// claims we never measured either.
    #[test]
    fn absent_cell_distinguishes_in_flight_from_cloud_era() {
        let ctx = ctx_from(&two_sided(), "m5-air");
        assert_eq!(ctx.absent_cell("on this box"), PENDING_TEXT);
        assert_eq!(ctx.absent_cell("never re-baselined"), CLOUD_ERA_TEXT);
    }

    /// The same manifest rendered for a DIFFERENT box: the board that is
    /// rebaselined on `m5-air` is a cloud-era ghost from anywhere else. This is
    /// the state the next hardware move puts the whole table into, and the
    /// reason `rebaselined_on` is a list of boxes rather than a boolean.
    #[test]
    fn a_different_box_sees_every_board_as_cloud_era() {
        let ctx = ctx_from(&two_sided(), "some-other-box");
        assert_eq!(ctx.absent_cell("on this box"), CLOUD_ERA_TEXT);
        assert_eq!(ctx.absent_cell("never re-baselined"), CLOUD_ERA_TEXT);
    }

    /// A label with no board at all answers "cloud-era", exactly as Python's
    /// `label in AIR_REBASELINED` answers false for a name it has never heard
    /// of. It is the conservative answer: it points at git history rather than
    /// promising a sweep that nothing is going to run.
    #[test]
    fn an_unknown_label_is_not_claimed_as_pending() {
        let ctx = ctx_from(&two_sided(), "m5-air");
        assert_eq!(ctx.absent_cell("no such board"), CLOUD_ERA_TEXT);
    }

    /// The pending/cloud split in `standings()` reads the same rule as
    /// `absent_cell`, so the detail table and the summary can never disagree
    /// about which kind of absence a board has.
    #[test]
    fn standings_splits_absences_the_same_way_absent_cell_does() {
        let ctx = ctx_from(&two_sided(), "m5-air");
        let s = ctx.standings();
        assert!(s.live.is_empty());
        assert_eq!(s.pending, vec!["on this box"]);
        assert_eq!(s.cloud, vec!["never re-baselined"]);
        // Cloud-era boards are excluded from the headline, not summed into it.
        assert_eq!(s.total_rows, 0);
    }

    /// The proof mark is TWO codepoints. A bare U+2696 renders almost
    /// identically in most fonts and diffs on every proof row of two published
    /// tables -- the definition of a silent diff.
    #[test]
    fn proof_mark_carries_the_variation_selector() {
        let src = two_sided().replace("id = \"swept\"", "id = \"swept\"\nproof_track = true");
        let ctx = ctx_from(&src, "m5-air");
        assert_eq!(
            ctx.proof_mark("on this box").as_bytes(),
            b" \xe2\x9a\x96\xef\xb8\x8f"
        );
        assert_eq!(ctx.proof_mark("never re-baselined"), "");
    }

    /// A missing `val-unavailable.json` is an empty exemption set, not a
    /// failure to render: the file is generated by `val-availability.py` and a
    /// clean clone may not carry it.
    #[test]
    fn a_missing_val_map_degrades_to_empty() {
        let v = load_val_unavailable(Path::new("/nonexistent/val-unavailable.json"));
        assert!(v.is_empty());
    }
}
