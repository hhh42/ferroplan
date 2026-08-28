//! Two implementations of one rule, held against each other.
//!
//! The resume gate exists twice on purpose: `sched::resume::judge` reads a
//! board's conditions FILE (the artifact side, what `ipc67.py` does), and
//! `db::Reader::window_gate` reads the box-wide `sample` table (the database
//! side, what a live crucible sweep does). Two implementations of one rule is
//! the shape of half the incidents in this repo's comment corpus -- one drifts
//! from the other unobserved -- so this file walks every real timeline fixture
//! on the box, feeds it to both, and refuses any window on which they differ.
//! Same idea as `the_two_conditions_readers_agree`, one layer down.

use crucible_core::db::{Cleanliness, Db, SampleRec};
use crucible_core::sched::resume::{judge, Conditions, Reject, RunParams, ENGINE_KEY};
use crucible_publish::raw::{Instance, Present, RawRow};
use std::path::PathBuf;

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn scratch(tag: &str) -> Scratch {
    let d = std::env::temp_dir().join(format!(
        "crucible-gate-{}-{tag}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&d).unwrap();
    Scratch(d)
}

fn fixtures() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/conditions");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("timeline-") && n.ends_with(".json"))
        })
        .collect();
    out.sort();
    out
}

fn params() -> RunParams {
    RunParams {
        engine: "0123abcd".into(),
        ver: Some("ff 0.26.0".into()),
        budget_secs: 60.0,
        mode: None,
        jobs: 2,
        threads: "1".into(),
    }
}

/// A row that passes every identity check, so the only thing `judge` can
/// refuse it on is the timeline.
fn row(start: f64, end: f64) -> RawRow {
    let mut extra = serde_json::Map::new();
    extra.insert(
        ENGINE_KEY.into(),
        serde_json::Value::String(params().engine),
    );
    RawRow {
        ipc: Some("ipc-2014".into()),
        variant: "toy".into(),
        instance: Instance::Num(1),
        solved: false,
        time: None,
        metric: None,
        length: None,
        val: None,
        notes: None,
        budget: Some(60.0),
        ver: Some("ff 0.26.0".into()),
        mode: Some("auto".into()),
        jobs: Some(2),
        threads: Some(serde_json::Value::String("1".into())),
        start_ts: Some(start),
        end_ts: Some(end),
        makespan: None,
        resumed_clean: false,
        extra,
        present: Present::current(false),
    }
}

/// The artifact gate's answer in the database gate's vocabulary.
fn expected(v: Result<RawRow, Reject>) -> Cleanliness {
    match v {
        Ok(_) => Cleanliness::Clean,
        Err(Reject::Contended { .. }) | Err(Reject::UnknownLoad { .. }) => Cleanliness::Dirty,
        Err(Reject::WindowOutsideSpan { .. }) | Err(Reject::NoOverlappingSamples { .. }) => {
            Cleanliness::Uncovered
        }
        Err(other) => panic!("the row failed an identity check, not the timeline: {other:?}"),
    }
}

#[test]
fn the_sql_gate_and_the_artifact_gate_agree_on_every_real_timeline() {
    let files = fixtures();
    assert!(
        files.len() >= 4,
        "the four rescued timeline fixtures are the corpus: {files:?}"
    );
    let mut seen = std::collections::BTreeSet::new();
    let mut checked = 0usize;

    for f in &files {
        let cond = Conditions::load(f).unwrap_or_else(|| panic!("{}: unreadable", f.display()));
        assert!(cond.has_timeline(), "{}: no timeline", f.display());
        let iv = cond.interval;
        let first = cond.timeline[0].at;
        let last = cond.timeline[cond.timeline.len() - 1].at;

        // The same timeline, as the live watcher would have written it:
        // box-wide, no pass.
        let dir = scratch(f.file_stem().unwrap().to_str().unwrap());
        let db = Db::open(&dir.0).expect("open");
        for s in &cond.timeline {
            db.writer().sample(SampleRec {
                at: s.at,
                idle_pct: s.idle_pct,
                competitors_total: s.competitors_total,
                ..SampleRec::default()
            });
        }
        db.writer().flush().expect("flush");
        let r = db.reader().expect("reader");

        // Windows that between them reach every verdict either gate can give:
        // one sitting on every sample, one between every pair of samples, and
        // the edges -- before, after, straddling, and the whole span.
        let mut windows: Vec<(f64, f64)> = Vec::new();
        for s in &cond.timeline {
            windows.push((s.at - 0.5, s.at + 0.5));
        }
        for w in cond.timeline.windows(2) {
            if w[1].at - w[0].at > 2.0 {
                windows.push((w[0].at + 1.0, w[1].at - 1.0));
            }
        }
        windows.push((first - 3.0 * iv, first - 2.0 * iv));
        windows.push((last + 2.0 * iv, last + 3.0 * iv));
        windows.push((first - 2.0 * iv, first + iv));
        windows.push((last - iv, last + 2.0 * iv));
        windows.push((first, last));

        for (s, e) in windows {
            let want = expected(judge(&row(s, e), &cond, &params()));
            let got = r.window_gate(s, e, iv, None).expect("gate");
            assert_eq!(
                got,
                want,
                "{}: window [{s}, {e}] -- the database gate says {got:?}, the artifact gate \
                 says {want:?}",
                f.display()
            );
            seen.insert(format!("{got:?}"));
            checked += 1;
        }
    }
    // Not vacuous: every verdict was exercised somewhere in the corpus.
    for v in ["Clean", "Dirty", "Uncovered"] {
        assert!(
            seen.contains(v),
            "no window reached {v} across {checked} checks"
        );
    }
}
