//! The byte-for-byte gate of the whole port.
//!
//! Render all three published documents from the COMMITTED repository state and
//! compare them, byte for byte, against the artifacts sitting in git:
//! `benchmarks/ipc-standings.md`, `STANDINGS.md`, and the block between the
//! markers in `README.md`.
//!
//! This is the only test that can distinguish a PORT from a rewrite. The
//! Python's comment corpus records incident after incident where a subtle slip
//! -- a half-away-from-zero round, a `max()` that returned the last maximum
//! instead of the first, a dropped variation selector -- produced a wrong
//! published number. None of those show up as a crash, and most do not show up
//! in a unit test either: they show up as one character of one line of one
//! table. So the assertion is the table.
//!
//! **The raws are gitignored.** They are working data from multi-hour sweeps
//! and they are absent on a clean clone. A missing input SKIPS rather than
//! fails: a red test on a fresh checkout trains people to ignore red tests, and
//! this is the one test in the crate that must never be ignored. The skip
//! prints what it could not find.
//!
//! **Failures print ONE line, not two documents.** `assert_eq!` on a pair of
//! seventy-line strings prints a hundred and forty lines of escaped text with
//! the one that matters somewhere inside it. The comparison below finds the
//! first differing line, prints its number and both sides, and stops.

use std::path::{Path, PathBuf};

use crucible_publish::history::BoxId;
use crucible_publish::render::{detail, readme, summary, RenderCtx, DEFAULT_BOX};

/// The repository root: two levels up from this crate, then out of `crucible/`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("the repository root resolves")
}

/// Every input the three documents are rendered from. Absent inputs are the
/// clean-clone case and skip the whole battery.
fn inputs_present(root: &Path) -> Result<(), String> {
    let manifest = root.join("benchmarks/manifest.toml");
    if !manifest.exists() {
        return Err(format!("{} is absent", manifest.display()));
    }
    let src = std::fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
    let m = crucible_publish::manifest::Manifest::parse(&src, "manifest.toml")
        .map_err(|e| e.to_string())?;
    // At least one board must have landed. The committed artifacts were
    // rendered from a full sweep, so a partial set of raws would produce a
    // legitimately different table -- comparing it against the committed one
    // would fail for a reason that is not a port bug.
    let missing: Vec<String> = m
        .boards
        .iter()
        .filter(|b| {
            let raw = root.join("benchmarks").join(&b.raw);
            let md = root.join("benchmarks").join(&b.md);
            // The presence gate: a board counts only when BOTH exist. A board
            // absent from both is a sweep that has not run, which the committed
            // artifacts already render as absent -- so it is only the
            // half-present state, and boards the artifacts DID render, that
            // make a comparison meaningless.
            raw.exists() != md.exists()
        })
        .map(|b| b.id.clone())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "boards half-landed (raw or .md, not both): {}",
            missing.join(", ")
        ));
    }
    if !root.join("benchmarks/ipc5-prop.jsonl").exists() {
        return Err("benchmarks/ipc5-prop.jsonl is absent (raws are gitignored)".to_string());
    }
    Ok(())
}

/// Compare two documents and describe the FIRST line that differs.
///
/// Line-oriented on purpose: these are markdown tables, one row per line, and
/// the unit a human debugs is a row. The trailing-byte check is separate
/// because a file that differs only in its final newline has no differing line
/// at all -- and that trailing byte is in the committed artifact.
fn diff_report(what: &str, got: &str, want: &str) -> Option<String> {
    if got == want {
        return None;
    }
    let g: Vec<&str> = got.lines().collect();
    let w: Vec<&str> = want.lines().collect();
    for (i, (a, b)) in g.iter().zip(w.iter()).enumerate() {
        if a != b {
            return Some(format!(
                "{what}: first difference at line {}\n  rendered: {a:?}\n  committed: {b:?}",
                i + 1
            ));
        }
    }
    if g.len() != w.len() {
        let (side, extra) = if g.len() > w.len() {
            ("rendered has extra", g[w.len()])
        } else {
            ("committed has extra", w[g.len()])
        };
        return Some(format!(
            "{what}: {} lines ({} vs {}); first extra at line {}: {extra:?}",
            side,
            g.len(),
            w.len(),
            w.len().min(g.len()) + 1
        ));
    }
    // Same lines, different bytes: only the trailing newlines can differ.
    Some(format!(
        "{what}: every line matches but the trailing bytes differ \
         (rendered ends {:?}, committed ends {:?})",
        tail(got),
        tail(want)
    ))
}

/// The last few characters of a document, for the trailing-byte report. Taken
/// by CHARACTER so a multi-byte tail cannot panic the reporter -- a diagnostic
/// that crashes while describing a diff is worse than no diagnostic.
fn tail(s: &str) -> String {
    let n = s.chars().count().saturating_sub(8);
    s.chars().skip(n).collect()
}

/// The three documents, rendered once and checked together, so a run reports
/// every one that moved rather than only the first.
#[test]
fn renders_the_committed_standings_byte_for_byte() {
    let root = repo_root();
    if let Err(why) = inputs_present(&root) {
        eprintln!("SKIP: live raws absent -- {why}");
        return;
    }

    // The committed artifacts were all measured on `m5-air`; the Python takes
    // the box from `$FERROPLAN_BOX` and defaults to the same name.
    let ctx = RenderCtx::load(&root, BoxId::new(DEFAULT_BOX)).expect("the context loads");

    let mut problems: Vec<String> = Vec::new();

    let want_detail = std::fs::read_to_string(root.join("benchmarks/ipc-standings.md"))
        .expect("benchmarks/ipc-standings.md is committed");
    if let Some(p) = diff_report(
        "benchmarks/ipc-standings.md",
        &detail::render(&ctx),
        &want_detail,
    ) {
        problems.push(p);
    }

    let want_summary =
        std::fs::read_to_string(root.join("STANDINGS.md")).expect("STANDINGS.md is committed");
    if let Some(p) = diff_report("STANDINGS.md", &summary::render(&ctx), &want_summary) {
        problems.push(p);
    }

    let readme_text =
        std::fs::read_to_string(root.join("README.md")).expect("README.md is committed");
    let block = readme::block(&ctx).expect("boards are live, so a block renders");
    let patched = readme::patch(&readme_text, &block).expect("README.md carries both markers");
    if let Some(p) = diff_report("README.md", &patched, &readme_text) {
        problems.push(p);
    }

    assert!(problems.is_empty(), "\n{}\n", problems.join("\n\n"));
}

/// The README splice must be IDEMPOTENT: patching an already-patched README
/// with the same block reproduces it exactly. If it were not, every
/// regeneration would grow or shave the file by a newline and the front page
/// would churn on every release with no number changing.
#[test]
fn patching_an_already_patched_readme_is_a_fixed_point() {
    let root = repo_root();
    if let Err(why) = inputs_present(&root) {
        eprintln!("SKIP: live raws absent -- {why}");
        return;
    }
    let ctx = RenderCtx::load(&root, BoxId::new(DEFAULT_BOX)).expect("the context loads");
    let readme_text =
        std::fs::read_to_string(root.join("README.md")).expect("README.md is committed");
    let block = readme::block(&ctx).expect("a block renders");
    let once = readme::patch(&readme_text, &block).expect("markers present");
    let twice = readme::patch(&once, &block).expect("markers survive the first splice");
    assert_eq!(once, twice, "the splice is not a fixed point");
}

/// The two summaries must agree about the headline. `STANDINGS.md` and the
/// README block are generated from one `standings()` pass for exactly this
/// reason -- a front page that disagrees with the page behind it is the
/// hand-maintained-numbers failure the generator exists to end.
#[test]
fn the_front_page_and_the_summary_quote_the_same_totals() {
    let root = repo_root();
    if let Err(why) = inputs_present(&root) {
        eprintln!("SKIP: live raws absent -- {why}");
        return;
    }
    let ctx = RenderCtx::load(&root, BoxId::new(DEFAULT_BOX)).expect("the context loads");
    let st = ctx.standings();
    let totals = format!(
        "({}/{})",
        crucible_publish::fmt::thousands(st.total_solved as u64),
        crucible_publish::fmt::thousands(st.total_rows as u64)
    );
    let summary_doc = summary::render(&ctx);
    let block = readme::block(&ctx).expect("a block renders");
    assert!(summary_doc.contains(&totals), "STANDINGS.md lost {totals}");
    assert!(block.contains(&totals), "the README block lost {totals}");
    assert!(
        block.contains(&format!("across {} IPC boards", st.live.len())),
        "the README block disagrees on the board count"
    );
}
