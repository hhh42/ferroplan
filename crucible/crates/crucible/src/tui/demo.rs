//! A synthetic sweep for `crucible tui --demo` and `--dump`: the layout gets
//! looked at without spending three days of CPU to see it, and a dumped
//! frame can be reviewed in a transcript or checked in CI.

use super::app::*;
use std::time::Duration;

/// A snapshot `t` seconds into an imaginary sweep.
pub fn snapshot(t: f64) -> Snapshot {
    let specs: [(&str, usize, u64, u32); 8] = [
        ("ipc5-prop", 450, 60, 1),
        ("ipc67-results", 580, 60, 1),
        ("ipc67-temporal", 630, 60, 1),
        ("ipc2014-agile", 280, 60, 1),
        ("ipc2018-sat", 240, 60, 1),
        ("ipc2023-agile-300s", 140, 300, 1),
        ("ipc7-mco-t4", 280, 60, 4),
        ("ipc-opt-2008-11", 550, 60, 1),
    ];
    let progress = ((t * 12.0) as usize).max(1);
    let mut cursor = 0usize;
    let mut boards = Vec::new();
    let mut running_slot: Option<SlotRun> = None;
    for (id, n, budget, threads) in specs {
        let mut cells = Vec::with_capacity(n);
        for i in 0..n {
            let global = cursor + i;
            let prev_solved = (i * 7 + 3) % 10 < 7;
            let prev_secs = if prev_solved {
                Some(0.4 + (i * 37 % 500) as f64 / 10.0)
            } else {
                None
            };
            let mut c = InstanceCell {
                variant: format!("{}-domain{}", id.split('-').next().unwrap_or(id), i / 20),
                label: (i % 20 + 1).to_string(),
                prev_solved: Some(prev_solved),
                prev_secs,
                ..Default::default()
            };
            if global < progress {
                let this_solved = (i * 7 + 3) % 10 < 7 || i % 53 == 0;
                c.this_solved = Some(this_solved && i % 97 != 0);
                c.this_secs =
                    Some(prev_secs.unwrap_or(budget as f64) * if i % 3 == 0 { 0.9 } else { 1.05 });
                c.rho = Some(0.97 + (i % 4) as f64 * 0.007);
                c.attempt = 1;
                c.cell = if c.this_solved == Some(true) {
                    if i % 11 == 0 {
                        Cell::SolvedDirty
                    } else {
                        Cell::SolvedClean
                    }
                } else if i % 13 == 0 {
                    c.rho = Some(0.61);
                    c.verdict = Some("starved".into());
                    Cell::Owed
                } else {
                    c.verdict = Some("rho".into());
                    Cell::TimeoutBanked
                };
                if c.verdict.is_none() {
                    c.verdict = Some("solved".into());
                }
            } else if global == progress {
                c.cell = Cell::Running;
                running_slot = Some(SlotRun {
                    board: id.into(),
                    variant: c.variant.clone(),
                    instance: c.label.clone(),
                    effective: Duration::from_millis(((t * 1000.0) as u64) % (budget * 1000)),
                    suspended: (t as u64 / 8) % 3 == 2,
                    budget: Duration::from_secs(budget),
                    rho: Some(0.99),
                    rss_mb: Some(412.0),
                    last_stderr: Some("[ladder] rung 2 novelty-light, 14k pops".into()),
                });
            }
            cells.push(c);
        }
        cursor += n;
        boards.push(BoardRow {
            id: id.into(),
            label: id.into(),
            budget_secs: budget,
            threads,
            cells,
        });
    }
    let level = match (t as u64 / 8) % 3 {
        0 => (Level::Full, None),
        1 => (Level::Polite, Some("Foreign(34.0)".to_string())),
        _ => (
            Level::Suspended,
            Some("Game { name: \"Timberborn\", cpu: 240.0 }".to_string()),
        ),
    };
    let mut s = Snapshot {
        engine_ver: "ff 0.27.0".into(),
        engine_hash: "03a17198744b".into(),
        level: LevelState {
            level: level.0,
            reason: level.1,
        },
        uptime: Duration::from_secs_f64(t) + Duration::from_secs(4 * 3600 + 720),
        quiet_in: Some(Duration::from_secs(9660)),
        sweep: SweepProgress {
            delta_vs: "0.26.0".into(),
            eta: Some(Duration::from_secs(51_720)),
            ..Default::default()
        },
        boards,
        slots: vec![Slot {
            index: 0,
            what: running_slot,
        }],
        p_cores: 4,
        log: vec![
            LogLine {
                at: "14:02:11".into(),
                kind: LogKind::Warn,
                text: "throttle full -> polite (Foreign(34.0))".into(),
            },
            LogLine {
                at: "14:03:40".into(),
                kind: LogKind::Good,
                text: "ipc5-prop verdicts: solved 380, rho 68, starved 2".into(),
            },
            LogLine {
                at: "14:04:02".into(),
                kind: LogKind::Regression,
                text: "REGRESSION ipc67-results/transport-strips/14 -- solved on 0.26.0, unsolved and banked here".into(),
            },
        ],
        toasts: vec![],
        throughput: (0..30).map(|i| 20.0 + 6.0 * ((i as f64 + t) / 3.0).sin()).collect(),
        canary: Some(1.0 + 0.2 * ((t / 5.0).sin().max(0.0))),
        competitors: vec![("Brave Browser Helper".into(), 34.0), ("WindowServer".into(), 9.0)],
        ..Default::default()
    };
    s.tally();
    s
}

/// A synthetic instance detail, for the dumped instance and timeline views.
pub fn detail() -> InstanceDetail {
    let start = 1_788_400_000.0;
    let points: Vec<TimelinePoint> = (0..90)
        .map(|k| TimelinePoint {
            at: start + k as f64 * 20.0,
            foreign: Some(if (30..45).contains(&k) {
                180.0
            } else {
                8.0 + (k % 5) as f64
            }),
            canary: Some(if (30..48).contains(&k) { 1.31 } else { 1.02 }),
            swap_mb: Some(6_600.0 + (k as f64) * 40.0),
            mem_pressure: Some(if (35..40).contains(&k) { 4 } else { 2 }),
        })
        .collect();
    InstanceDetail {
        attempts: vec![
            AttemptRow {
                attempt: 1,
                solved: false,
                secs: Some(60.0),
                wall_ms: Some(61_020),
                cpu_ms: Some(38_400),
                suspended_ms: Some(0),
                peak_rss: Some(812 << 20),
                timing: "dirty".into(),
                verdict: Some("starved".into()),
                started_at: Some(start + 600.0),
                finished_at: Some(start + 661.0),
            },
            AttemptRow {
                attempt: 2,
                solved: false,
                secs: Some(60.0),
                wall_ms: Some(60_250),
                cpu_ms: Some(59_800),
                suspended_ms: Some(0),
                peak_rss: Some(790 << 20),
                timing: "clean".into(),
                verdict: Some("rho".into()),
                started_at: Some(start + 1_300.0),
                finished_at: Some(start + 1_360.0),
            },
        ],
        competitors: vec![
            ("Brave Browser Helper".into(), 92.0),
            ("WindowServer".into(), 21.0),
        ],
        canary_max: Some(1.31),
        swap_growth_mb: Some(340.0),
        timeline: Timeline {
            points,
            windows: vec![(start + 620.0, Some(start + 900.0), "suspended".into())],
            runs: vec![
                (start + 600.0, start + 661.0, false),
                (start + 1_300.0, start + 1_360.0, true),
            ],
        },
    }
}
