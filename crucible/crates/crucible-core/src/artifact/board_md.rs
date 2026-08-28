//! The `<board>.md` board summary -- the human-readable face of a sweep.
//!
//! Ported from `benchmarks/ipc67.py:main` (the `out = [...]` block at the end).
//! It is a small file and almost every line of it is load-bearing somewhere
//! else in the project:
//!
//! * **The last line is parsed.** The sweep drivers literally `tail -1` this
//!   file to get their one-line log entry, so `total coverage: **S/N**` has to
//!   be the final line, has to be spelled that way, and has to be preceded by
//!   the blank line the Python's `"\n" + ...` element puts there. A renderer
//!   that appended anything after it -- a timestamp, a trailing table, a
//!   courtesy newline of its own -- silently changes what every driver logs.
//!
//! * **Summed cost is NOT an IPC quality score and must never be presented as
//!   one.** The IPC formula is `reference_cost / your_cost` capped at 1, and
//!   this corpus carries no reference costs, so the column is the raw sum of
//!   plan costs and nothing more. A board that solves twenty hard instances
//!   badly outscores one that solves two well on this column; reading it as
//!   quality inverts the result. `--score-against PRIOR.jsonl` adds a real
//!   quality column, but it is SELF-relative -- the IPC formula against this
//!   project's own prior best -- which makes it regression tracking, not an
//!   official score, and the trailing line says so in those words.
//!
//! * **`val` is a tristate and `null` is not a verdict.** The column counts
//!   `true` as ok and `false` as fail; a `null` -- VAL unavailable, or the
//!   validator itself crashed -- is counted as NEITHER, so `18/18` means
//!   eighteen plans checked and eighteen accepted, not eighteen out of twenty
//!   rows. Folding `null` into either bucket is the 0.20, 0.21 and 0.23
//!   incidents, and the whole column collapses to `-` when no validator was
//!   found at all, because a board with no validator must not display a
//!   validation record.
//!
//! * **The STITCHED sentence appears if and only if rows were reused.** A
//!   stitched board says so -- the conditions honesty rule. Printing it with a
//!   zero count would mark every ordinary board as stitched; omitting it on a
//!   stitched one hides that some rows were measured in an earlier pass.
//!
//! * **Solve time is summed over SOLVED rows only**, so it is "time spent
//!   succeeding", not "time spent". A board that times out on eighteen of
//!   twenty instances shows a small number here, and that is the intended
//!   reading -- adding the timeouts in would make every bad board look busy.
//!
//! Cost per row is `metric` where the domain has one and `length` otherwise,
//! which is why a numeric board's column is huge next to a STRIPS board's: they
//! are different currencies and are never compared across tracks.

use crucible_publish::fmt::{fmt_f, glyph};
use crucible_publish::raw::{Instance, RawRow};
use std::collections::HashMap;

/// The run parameters that only the invocation knows -- none of them are
/// recoverable from the rows, so the caller supplies them.
#[derive(Debug, Clone, Default)]
pub struct BoardHeader {
    /// `--track`, printed verbatim into the title (`seq-sat-2014`, `seq-opt`,
    /// `numeric-2026`, ...).
    pub track: String,
    pub timeout_s: i64,
    pub jobs: u32,
    /// `--mode` passthrough. `None` (and the empty string) render as `auto`,
    /// which is what `MODE or 'auto'` does.
    pub mode: Option<String>,
    /// Whether a validator was found. Drives both the header sentence and
    /// whether the `val` column shows counts or a dash.
    pub val: bool,
    /// Rows reused from a prior pass's clean windows. The STITCHED sentence
    /// appears if and only if this is non-zero.
    pub reused_total: usize,
    /// `--resume-raw`. Only its basename is printed.
    pub resume_raw: Option<String>,
}

/// One table row: a variant's whole result.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantSummary {
    /// `None` prints as `None`, which is what an f-string does with it. A row
    /// with no `ipc` key is a malformed raw, and it should look malformed.
    pub ipc: Option<String>,
    pub variant: String,
    pub solved: usize,
    pub total: usize,
    /// `metric` where present, else `length`, summed over SOLVED rows.
    pub cost_sum: f64,
    /// Wall-clock summed over SOLVED rows.
    pub time_sum: f64,
    pub val_ok: usize,
    pub val_fail: usize,
    /// Self-relative quality, present only under `--score-against`.
    pub quality: Option<f64>,
}

/// Per-instance costs from a PRIOR run's raw JSONL, for `--score-against`.
#[derive(Debug, Clone, Default)]
pub struct Reference {
    costs: HashMap<(String, InstKey), f64>,
}

/// An instance label as a hash key.
///
/// Mirrors `raw::Instance` rather than flattening it to a string: the int `3`
/// and the string `"3"` are different dict keys in Python, and collapsing them
/// is the same class of mistake that put `ipc2026-numeric`'s 320 rows under 288
/// keys and broke this very join.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum InstKey {
    Num(u64),
    Parts(String),
}

impl InstKey {
    fn of(i: &Instance) -> InstKey {
        match i {
            Instance::Num(n) => InstKey::Num(*n),
            Instance::Parts(s) => InstKey::Parts(s.clone()),
        }
    }
}

impl Reference {
    /// `ipc67.py:load_reference`.
    ///
    /// Keyed on (variant, instance) with NO ipc component -- that is the
    /// Python's key, and it is why a prior run of a different competition year
    /// contributes nothing rather than mis-joining. A row only enters if it was
    /// solved AND its cost is truthy: a zero cost is not a reference, it is a
    /// missing measurement, and dividing by it later would be worse than having
    /// no reference at all.
    pub fn from_rows(rows: &[RawRow]) -> Reference {
        let mut costs = HashMap::new();
        for r in rows {
            if !r.solved {
                continue;
            }
            if let Some(c) = row_cost(r) {
                if c != 0.0 {
                    // A later row wins, exactly as reassigning a dict key does.
                    costs.insert((r.variant.clone(), InstKey::of(&r.instance)), c);
                }
            }
        }
        Reference { costs }
    }

    fn get(&self, variant: &str, instance: &Instance) -> Option<f64> {
        self.costs
            .get(&(variant.to_string(), InstKey::of(instance)))
            .copied()
    }

    pub fn len(&self) -> usize {
        self.costs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.costs.is_empty()
    }
}

/// `metric` where the domain has one, `length` otherwise. `None` when neither
/// is recorded -- which is not the same as zero, and the callers distinguish.
fn row_cost(r: &RawRow) -> Option<f64> {
    match r.metric {
        Some(m) => Some(m),
        None => r.length.map(|l| l as f64),
    }
}

/// Fold a board's rows into one table row per variant.
///
/// Rows are grouped in CONTIGUOUS runs, not by first appearance anywhere in the
/// file. `ipc67.py` writes a whole variant's rows in one go after its pool
/// finishes, so a run is exactly a variant; grouping by first appearance would
/// merge two blocks that the Python would have reported as two rows.
pub fn summarize_variants(rows: &[RawRow], reference: Option<&Reference>) -> Vec<VariantSummary> {
    let mut out: Vec<VariantSummary> = Vec::new();
    let mut start = 0usize;
    while start < rows.len() {
        let key = (rows[start].ipc.clone(), rows[start].variant.clone());
        let mut end = start + 1;
        while end < rows.len() && (rows[end].ipc.clone(), rows[end].variant.clone()) == key {
            end += 1;
        }
        out.push(fold(&rows[start..end], reference));
        start = end;
    }
    out
}

fn fold(recs: &[RawRow], reference: Option<&Reference>) -> VariantSummary {
    let head = &recs[0];
    let mut solved = 0usize;
    let mut val_ok = 0usize;
    let mut val_fail = 0usize;
    // Float addition is not associative: both sums walk the rows in file order
    // because the Python walks `recs` in pool order, which is file order.
    let mut cost_sum = 0.0f64;
    let mut time_sum = 0.0f64;
    for r in recs {
        if r.solved {
            solved += 1;
            // `r["length"] or 0`: a missing or zero length contributes nothing,
            // but a `metric` of 0.0 is a measurement and is added as one.
            cost_sum += match r.metric {
                Some(m) => m,
                None => r.length.unwrap_or(0) as f64,
            };
            time_sum += r
                .time
                .as_ref()
                .and_then(serde_json::Number::as_f64)
                .unwrap_or(0.0);
        }
        match r.val {
            Some(true) => val_ok += 1,
            Some(false) => val_fail += 1,
            // `null` is UNAVAILABLE, not a verdict. It counts for neither side.
            None => {}
        }
    }

    let quality = reference.map(|reference| {
        let mut q = 0.0f64;
        for r in recs {
            let cost = row_cost(r);
            let rc = reference.get(&r.variant, &r.instance);
            match (r.solved, cost, rc) {
                // The IPC formula, capped at 1: we are never rewarded for
                // beating the reference, only penalised for losing to it.
                (true, Some(c), Some(rc)) if c != 0.0 && rc != 0.0 => q += 1.0f64.min(rc / c),
                // Solved something the reference never did. Credited in full --
                // and note this fires even when OUR cost is missing, because the
                // Python's `elif` tests only `rc is None`.
                (true, _, None) => q += 1.0,
                _ => {}
            }
        }
        q
    });

    VariantSummary {
        ipc: head.ipc.clone(),
        variant: head.variant.clone(),
        solved,
        total: recs.len(),
        cost_sum,
        time_sum,
        val_ok,
        val_fail,
        quality,
    }
}

/// `os.path.basename` on a POSIX path.
fn basename(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

/// The whole `<board>.md`, including its trailing newline.
///
/// `score_against`, when set, is both the switch for the sixth column and the
/// path printed in the trailing line -- the Python gates on `reference is not
/// None`, and `reference` exists exactly when `--score-against` was given.
pub fn render(
    header: &BoardHeader,
    summary: &[VariantSummary],
    score_against: Option<&str>,
) -> String {
    // `reference = load_reference(SCORE_AGAINST) if SCORE_AGAINST else None` --
    // the switch is Python TRUTHINESS, not `is not None`, so an EMPTY path is no
    // reference at all and the board keeps its five columns. `is_some()` alone
    // would widen the table and print a `**0.00**` line quoting a path of `""`,
    // which the Python cannot produce. Same rule as `MODE or 'auto'` below.
    let score_against = score_against.filter(|s| !s.is_empty());
    let scored = score_against.is_some();
    let mode = match header.mode.as_deref() {
        Some(m) if !m.is_empty() => m,
        // `MODE or 'auto'`: the empty string is falsy in Python too.
        _ => "auto",
    };

    let mut second = format!(
        "timeout {}s/instance, jobs {}, mode {}.",
        header.timeout_s, header.jobs, mode
    );
    second.push_str(if header.val {
        " Plans externally validated with VAL."
    } else {
        " VAL not available."
    });
    if header.reused_total > 0 {
        second.push_str(&format!(
            " STITCHED: {} row(s) reused from a prior pass's clean windows ({}).",
            header.reused_total,
            header.resume_raw.as_deref().map(basename).unwrap_or("")
        ));
    }
    second.push('\n');

    // Built as the Python's list of lines and joined once, because the blank
    // lines in this document come from elements that carry their own `\n` -- not
    // from the join -- and that is easy to get subtly wrong when appending.
    let mut out: Vec<String> = vec![
        format!("# IPC-2008/2011 {} full-corpus results\n", header.track),
        second,
        format!(
            "| variant | coverage | summed cost | solve time | val |{}",
            if scored { " quality |" } else { "" }
        ),
        format!("|---|---|---|---|---|{}", if scored { "---|" } else { "" }),
    ];

    for v in summary {
        let vtag = if header.val {
            format!("{}/{}", v.val_ok, v.val_ok + v.val_fail)
        } else {
            "-".to_string()
        };
        let mut row = format!(
            "| {}/{} | {}/{} | {} | {}s | {} |",
            v.ipc.as_deref().unwrap_or("None"),
            v.variant,
            v.solved,
            v.total,
            fmt_f(v.cost_sum, 0),
            fmt_f(v.time_sum, 1),
            vtag
        );
        if scored {
            // Unreachable by construction -- `summarize_variants` fills
            // `quality` for every row whenever a reference is supplied -- but a
            // dash beats inventing a 0.00 that would land in the total.
            row.push_str(&match v.quality {
                Some(q) => format!(" {} |", fmt_f(q, 2)),
                None => " - |".to_string(),
            });
        }
        out.push(row);
    }

    let total: usize = summary.iter().map(|v| v.solved).sum();
    let n: usize = summary.iter().map(|v| v.total).sum();
    out.push(format!("\ntotal coverage: **{total}/{n}**"));

    if let Some(prior) = score_against {
        // NOT `.sum()`. Rust's `Sum` for floats folds from `-0.0`, so an empty
        // board renders `**-0.00**` where Python's `sum(...)` -- which starts at
        // the int `0` -- renders `**0.00**`. The fold from `+0.0` is Python's
        // start value exactly: `0 + x` promotes to `0.0 + x`, which is `x` for
        // every float and `+0.0` for `-0.0`, same as here.
        let qt: f64 = summary
            .iter()
            .filter_map(|v| v.quality)
            .fold(0.0, |a, b| a + b);
        out.push(format!(
            "\nself-relative quality vs `{}`: **{}** (IPC formula against our \
             own prior best {} regression tracking, NOT an official IPC score)",
            prior,
            fmt_f(qt, 2),
            glyph::EM_DASH
        ));
    }

    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_publish::parse_rows;

    /// A committed board and the run parameters it was produced with. The
    /// parameters are not in the raw, so they are written down here from the
    /// board's own header line -- which is the only place they survive.
    struct Board {
        name: &'static str,
        md: &'static str,
        jsonl: &'static str,
        track: &'static str,
        timeout_s: i64,
        jobs: u32,
        mode: Option<&'static str>,
    }

    const BOARDS: &[Board] = &[
        Board {
            name: "ipc2014-sat",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc2014-sat.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc2014-sat.jsonl"),
            track: "seq-sat-2014",
            timeout_s: 60,
            jobs: 2,
            mode: None,
        },
        Board {
            name: "ipc-opt-2008-11",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc-opt-2008-11.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc-opt-2008-11.jsonl"),
            track: "seq-opt",
            timeout_s: 60,
            jobs: 2,
            mode: Some("optimal"),
        },
        Board {
            name: "ipc2014-tempo",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc2014-tempo.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc2014-tempo.jsonl"),
            track: "tempo-sat-2014",
            timeout_s: 30,
            jobs: 2,
            mode: None,
        },
        Board {
            name: "ipc2023-numeric",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc2023-numeric.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc2023-numeric.jsonl"),
            track: "numeric-2023",
            timeout_s: 60,
            jobs: 2,
            mode: None,
        },
        Board {
            name: "ipc2023-agile-300s",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc2023-agile-300s.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc2023-agile-300s.jsonl"),
            track: "agile-2023",
            timeout_s: 300,
            jobs: 2,
            mode: None,
        },
        Board {
            name: "ipc2026-numeric",
            md: include_str!("../../../../../benchmarks/air-0.21.0/ipc2026-numeric.md"),
            jsonl: include_str!("../../../../../benchmarks/air-0.21.0/ipc2026-numeric.jsonl"),
            track: "numeric-2026",
            timeout_s: 60,
            jobs: 2,
            mode: None,
        },
    ];

    impl Board {
        fn header(&self) -> BoardHeader {
            BoardHeader {
                track: self.track.to_string(),
                timeout_s: self.timeout_s,
                jobs: self.jobs,
                mode: self.mode.map(str::to_string),
                val: true,
                reused_total: 0,
                resume_raw: None,
            }
        }
        fn rows(&self) -> Vec<RawRow> {
            parse_rows(self.jsonl, self.name).unwrap()
        }
    }

    /// The golden test: six committed boards, regenerated from their own raws
    /// and compared byte for byte. Between them they cover a non-default mode,
    /// a 30 s and a 300 s budget, float `metric` costs, integer `length` costs,
    /// zero-coverage variants and a board whose costs run to five figures.
    #[test]
    fn committed_boards_regenerate_byte_for_byte() {
        for b in BOARDS {
            let rows = b.rows();
            let summary = summarize_variants(&rows, None);
            assert_eq!(
                render(&b.header(), &summary, None),
                b.md,
                "{} differs",
                b.name
            );
        }
    }

    /// The line the sweep drivers `tail -1`. Its shape is an interface.
    #[test]
    fn the_last_line_is_the_coverage_total() {
        for b in BOARDS {
            let out = render(&b.header(), &summarize_variants(&b.rows(), None), None);
            let last = out.lines().next_back().unwrap();
            assert!(last.starts_with("total coverage: **"), "{}: {last}", b.name);
            assert!(last.ends_with("**"), "{}: {last}", b.name);
            assert!(out.ends_with("**\n"), "{}: no trailing newline", b.name);
            // And it is preceded by a blank line, as the Python's leading "\n"
            // element puts there.
            let lines: Vec<&str> = out.lines().collect();
            assert_eq!(lines[lines.len() - 2], "", "{}: no blank line", b.name);
        }
    }

    fn row(json: &str) -> RawRow {
        parse_rows(json, "synthetic").unwrap().pop().unwrap()
    }

    /// `null` is UNAVAILABLE, not a verdict. Four solved plans -- two accepted,
    /// one rejected, one the validator never managed to judge -- read as `2/3`.
    /// Counting the null as a pass gives `3/4`; counting it as a failure gives
    /// `2/4`. Both are claims about a plan nobody checked.
    #[test]
    fn a_null_val_counts_for_neither_side() {
        let src = "\
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 1, \"solved\": true, \"val\": true, \"length\": 1}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 2, \"solved\": true, \"val\": true, \"length\": 1}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 3, \"solved\": true, \"val\": null, \"length\": 1}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 4, \"solved\": true, \"val\": false, \"length\": 1}
";
        let s = summarize_variants(&parse_rows(src, "s").unwrap(), None);
        assert_eq!((s[0].val_ok, s[0].val_fail), (2, 1));
        let h = BoardHeader {
            track: "t".into(),
            val: true,
            ..Default::default()
        };
        let out = render(&h, &s, None);
        assert!(out.contains("| x/v | 4/4 | 4 | 0.0s | 2/3 |\n"), "{out}");
    }

    /// With no validator the whole column is a dash: a board that could not
    /// check a single plan must not display a validation record.
    #[test]
    fn no_validator_dashes_the_column_and_says_so() {
        let b = &BOARDS[0];
        let mut h = b.header();
        h.val = false;
        let out = render(&h, &summarize_variants(&b.rows(), None), None);
        assert!(out.contains("mode auto. VAL not available.\n"));
        assert!(!out.contains("Plans externally validated"));
        for line in out.lines().filter(|l| l.starts_with("| ipc-")) {
            assert!(line.ends_with("| - |"), "{line}");
        }
    }

    /// The STITCHED sentence appears if and only if rows were reused, and it
    /// names the source by BASENAME -- a full path would leak the operator's
    /// directory layout into a published file.
    #[test]
    fn stitched_is_announced_only_when_rows_were_reused() {
        let b = &BOARDS[0];
        let summary = summarize_variants(&b.rows(), None);

        let mut h = b.header();
        h.resume_raw = Some("/Users/x/ferroplan/benchmarks/prior/ipc2014-sat.jsonl".into());
        assert!(
            !render(&h, &summary, None).contains("STITCHED"),
            "a zero count must not mark the board stitched"
        );

        h.reused_total = 37;
        let out = render(&h, &summary, None);
        assert!(out.contains(
            " STITCHED: 37 row(s) reused from a prior pass's clean windows \
             (ipc2014-sat.jsonl).\n"
        ));
        assert!(!out.contains("/Users/x/"));
    }

    /// Solve time is time spent SUCCEEDING. The 59 seconds burnt on a timeout
    /// are not in the column, which is what makes the number comparable across
    /// boards with different coverage.
    #[test]
    fn solve_time_and_cost_cover_solved_rows_only() {
        let src = "\
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 1, \"solved\": true, \"time\": 1.5, \"length\": 10}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 2, \"solved\": false, \"time\": 59.9, \"length\": null}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 3, \"solved\": true, \"time\": 0.25, \"metric\": 7.5, \"length\": 99}
";
        let s = summarize_variants(&parse_rows(src, "s").unwrap(), None);
        assert_eq!(s[0].solved, 2);
        assert_eq!(s[0].time_sum, 1.75);
        // metric wins over length where both are present.
        assert_eq!(s[0].cost_sum, 17.5);
        let h = BoardHeader {
            track: "t".into(),
            val: true,
            ..Default::default()
        };
        assert!(
            render(&h, &s, None).contains("| 2/3 | 18 | 1.8s |"),
            "half-to-even"
        );
    }

    /// Both roundings in the row are Python's, which is half-to-EVEN: a cost
    /// summing to 17.5 renders `18` and one summing to 18.5 renders `18`, where
    /// `f64::round` would give 19. Same rule on the one-decimal solve time.
    #[test]
    fn the_row_uses_pythons_half_to_even_rounding() {
        assert_eq!(fmt_f(17.5, 0), "18");
        assert_eq!(fmt_f(18.5, 0), "18");
        assert_eq!(fmt_f(1.25, 1), "1.2");
    }

    /// A board scored against itself: every solved row matches its own
    /// reference cost exactly, so `min(1, rc/cost)` is 1.0 and the quality
    /// equals the coverage. That pins the sixth column, the widened separator
    /// row and the trailing self-relative line on real data.
    #[test]
    fn score_against_adds_the_sixth_column_and_the_disclaimer() {
        let b = &BOARDS[0];
        let rows = b.rows();
        let reference = Reference::from_rows(&rows);
        let summary = summarize_variants(&rows, Some(&reference));
        let out = render(&b.header(), &summary, Some("benchmarks/prior.jsonl"));

        assert!(out.contains("| variant | coverage | summed cost | solve time | val | quality |"));
        assert!(out.contains("|---|---|---|---|---|---|"));
        for v in &summary {
            assert_eq!(v.quality, Some(v.solved as f64), "{}", v.variant);
        }
        let total: usize = summary.iter().map(|v| v.solved).sum();
        assert!(out.contains(&format!("total coverage: **{total}/280**")));
        assert!(out.contains(&format!(
            "\nself-relative quality vs `benchmarks/prior.jsonl`: **{total}.00** \
             (IPC formula against our own prior best \u{2014} regression tracking, \
             NOT an official IPC score)\n"
        )));
        // The disclaimer's dash is U+2014, not an ASCII hyphen.
        assert!(!out.contains("prior best - regression"));
        // Unscored boards gain neither the column nor the line.
        let plain = render(&b.header(), &summarize_variants(&rows, None), None);
        assert!(!plain.contains("quality"));
    }

    /// The quality arithmetic, one branch at a time. Hand-computed from
    /// `ipc67.py:main`: 0.5 (we cost twice the reference), 1.0 (capped -- we beat
    /// it and are not rewarded), 1.0 (solved something the reference never did),
    /// and nothing at all for the unsolved row.
    #[test]
    fn quality_caps_at_one_and_credits_unseen_instances() {
        let prior = "\
{\"variant\": \"v\", \"instance\": 1, \"solved\": true, \"length\": 10}
{\"variant\": \"v\", \"instance\": 2, \"solved\": true, \"length\": 10}
{\"variant\": \"v\", \"instance\": 4, \"solved\": true, \"length\": 10}
";
        let now = "\
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 1, \"solved\": true, \"length\": 20}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 2, \"solved\": true, \"length\": 5}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 3, \"solved\": true, \"length\": 5}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 4, \"solved\": false, \"length\": null}
";
        let reference = Reference::from_rows(&parse_rows(prior, "prior").unwrap());
        assert_eq!(reference.len(), 3);
        let s = summarize_variants(&parse_rows(now, "now").unwrap(), Some(&reference));
        assert_eq!(s[0].quality, Some(2.5));
    }

    /// A zero cost is a missing measurement, not a reference: admitting it would
    /// put a division by zero one line further down.
    #[test]
    fn a_zero_cost_never_becomes_a_reference() {
        let prior = "\
{\"variant\": \"v\", \"instance\": 1, \"solved\": true, \"length\": 0}
{\"variant\": \"v\", \"instance\": 2, \"solved\": false, \"length\": 10}
";
        let reference = Reference::from_rows(&parse_rows(prior, "prior").unwrap());
        assert!(reference.is_empty());
        // Both rows are then "solved something the reference never did".
        let now = "\
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 1, \"solved\": true, \"length\": 4}
{\"ipc\": \"x\", \"variant\": \"v\", \"instance\": 2, \"solved\": true, \"length\": 4}
";
        let s = summarize_variants(&parse_rows(now, "now").unwrap(), Some(&reference));
        assert_eq!(s[0].quality, Some(2.0));
    }

    /// The reference key carries the instance's TYPE. A multipart label like
    /// `"3_10_50_10"` must not join to the integer instance 3.
    #[test]
    fn a_multipart_instance_does_not_join_to_an_integer_one() {
        let prior = "{\"variant\": \"v\", \"instance\": 3, \"solved\": true, \"length\": 10}\n";
        let reference = Reference::from_rows(&parse_rows(prior, "prior").unwrap());
        let r = row("{\"variant\": \"v\", \"instance\": \"3_10_50_10\", \"solved\": true, \"length\": 40}\n");
        assert_eq!(reference.get(&r.variant, &r.instance), None);
        let n = row("{\"variant\": \"v\", \"instance\": 3, \"solved\": true, \"length\": 40}\n");
        assert_eq!(reference.get(&n.variant, &n.instance), Some(10.0));
    }

    /// Two blocks of the same variant stay two table rows: rows are grouped in
    /// contiguous runs because that is how the runner writes them, and merging
    /// them would report a coverage the Python never printed.
    #[test]
    fn variants_are_grouped_in_contiguous_runs() {
        let src = "\
{\"ipc\": \"x\", \"variant\": \"a\", \"instance\": 1, \"solved\": true, \"length\": 1}
{\"ipc\": \"x\", \"variant\": \"b\", \"instance\": 1, \"solved\": true, \"length\": 1}
{\"ipc\": \"x\", \"variant\": \"a\", \"instance\": 2, \"solved\": false}
";
        let s = summarize_variants(&parse_rows(src, "s").unwrap(), None);
        assert_eq!(s.len(), 3);
        assert_eq!(s[0].total, 1);
        assert_eq!(s[2].variant, "a");
        assert_eq!(s[2].solved, 0);
    }

    /// An empty raw still renders a whole document -- `0/0`, no table rows. A
    /// board that produced nothing must still leave a readable record, and the
    /// driver's `tail -1` must still find its line.
    #[test]
    fn an_empty_board_still_writes_a_readable_record() {
        let h = BoardHeader {
            track: "seq-sat".into(),
            timeout_s: 60,
            jobs: 2,
            mode: None,
            val: false,
            reused_total: 0,
            resume_raw: None,
        };
        let out = render(&h, &[], None);
        assert_eq!(
            out,
            "# IPC-2008/2011 seq-sat full-corpus results\n\
             \n\
             timeout 60s/instance, jobs 2, mode auto. VAL not available.\n\
             \n\
             | variant | coverage | summed cost | solve time | val |\n\
             |---|---|---|---|---|\n\
             \n\
             total coverage: **0/0**\n"
        );
    }

    /// An empty `--mode` is falsy in Python and renders as `auto`, same as no
    /// mode at all. A port that printed `mode .` would diff on any driver that
    /// passes the flag through unset.
    #[test]
    fn an_empty_mode_renders_as_auto() {
        let h = BoardHeader {
            track: "t".into(),
            mode: Some(String::new()),
            ..Default::default()
        };
        assert!(render(&h, &[], None).contains("mode auto."));
    }

    /// A scored board with nothing to score sums NOTHING, and Python's `sum()`
    /// of nothing is the int `0`, which formats `0.00`. Rust's `Iterator::sum`
    /// for floats folds from `-0.0` instead, so the naive spelling publishes
    /// `**-0.00**` -- a negative quality, which the formula cannot produce, in
    /// the one line whose whole job is to be quotable.
    #[test]
    fn an_empty_scored_board_reports_a_positive_zero() {
        let h = BoardHeader {
            track: "seq-sat".into(),
            val: true,
            ..Default::default()
        };
        let out = render(&h, &[], Some("benchmarks/prior.jsonl"));
        assert!(out.contains("**0.00**"), "{out}");
        assert!(!out.contains("-0.00"), "{out}");
        // Same when every row's quality is absent rather than the board empty.
        let none_scored = [VariantSummary {
            ipc: Some("x".into()),
            variant: "v".into(),
            solved: 0,
            total: 1,
            cost_sum: 0.0,
            time_sum: 0.0,
            val_ok: 0,
            val_fail: 0,
            quality: None,
        }];
        let out = render(&h, &none_scored, Some("p.jsonl"));
        assert!(out.contains("**0.00**"), "{out}");
        assert!(!out.contains("-0.00"), "{out}");
    }

    /// `reference = load_reference(SCORE_AGAINST) if SCORE_AGAINST else None`
    /// gates on TRUTHINESS, so an empty `--score-against` is no reference at
    /// all. An `is_some()` port widens the table and prints a self-relative
    /// line quoting a path of `""` -- a quality claim against nothing.
    #[test]
    fn an_empty_score_against_path_scores_nothing() {
        let b = &BOARDS[0];
        let rows = b.rows();
        let reference = Reference::from_rows(&rows);
        let summary = summarize_variants(&rows, Some(&reference));
        let out = render(&b.header(), &summary, Some(""));
        assert!(!out.contains("quality"), "{out}");
        assert!(!out.contains("self-relative"), "{out}");
        assert_eq!(
            out,
            render(&b.header(), &summarize_variants(&rows, None), None)
        );
    }
}
