//! Rebuilding the database from the repo, and writing the repo back out.
//!
//! **The source of truth is the repo.** The board `.jsonl` files committed to
//! git are the durable record; this database is a fast, queryable cache and a
//! work queue, and it has to be reconstructible from those files plus the
//! manifest or it is a liability rather than an asset. That is the whole reason
//! `export` exists as a first-class operation and not a debugging aid: a cache
//! nobody can regenerate is a second source of truth.
//!
//! # A rebuilt run's timing is `unknown`, and it is never `clean`
//!
//! Only 4 of the 76 conditions files in this repo carry a per-sample timeline;
//! the other 72 carry a whole-board rollup and a one-word verdict. A rebuilt
//! run's cleanliness is therefore genuinely unknowable, and `unknown` is the
//! honest answer.
//!
//! It stays `unknown` even for the four boards that DO have a timeline. The
//! timeline is board-scoped evidence, and promoting it to a per-run verdict is
//! a judgment the rebuild is not entitled to make -- the live resume gate makes
//! that call, at resume time, and stamps `resumed_clean` on the row when it
//! does. That stamp is preserved here because it is the Python's own record of
//! a judgment that was actually made. Defaulting to `clean`, as
//! `crucible-spec.md` §8 does, would manufacture exactly the claim the
//! contention watcher exists to refuse -- and it would do it silently, on every
//! historical row at once.
//!
//! The timeline IS imported, into `sample`, because evidence is worth keeping
//! even when it does not support a verdict.
//!
//! # Attribution
//!
//! A row's board identity comes from the row's own stamps where it has them --
//! `budget`, `mode`, `jobs`, `threads` -- and from the manifest where it does
//! not. Boards swept before 0.25 carry no stamps at all, so the manifest is the
//! only thing that can say what they were measured under.
//!
//! A rebuilt board's `env` is empty, because the artifacts do not record it. A
//! live sweep of the same board therefore gets its OWN board row, which is
//! correct rather than unfortunate: a measurement whose environment is unknown
//! is not the same measurement as one whose environment is known.

use super::model::*;
use super::read::Reader;
use super::writer::WriterHandle;
use super::DbError;
use crucible_publish::manifest::{BoardSpec, Manifest};
use crucible_publish::raw::RawRow;
use crucible_publish::referee::ValUnavailable;
use std::path::{Path, PathBuf};

/// What one raw file contributed.
#[derive(Debug, Clone)]
pub struct RebuiltBoard {
    pub board_name: String,
    pub raw_path: PathBuf,
    pub board_id: i64,
    pub engine_id: i64,
    pub pass_id: i64,
    pub rows: usize,
    pub reused: usize,
    pub verdict: PassVerdict,
    /// Samples imported from the sibling conditions file. Zero for 72 of the
    /// 76 files in this repo, which is the reason a rebuilt run is `unknown`.
    pub samples: usize,
}

/// Read every board raw named by the manifest out of `dir` and load it.
///
/// A missing raw is SKIPPED, not an error: the manifest describes boards that
/// may not have been swept on this box, and `standings.py` renders that state
/// rather than failing on it. A raw that exists and does not parse IS an error
/// -- that is a corrupt record, not an absent one.
///
/// So is a raw that cannot be stored without losing a row: see
/// [`DbError::DuplicateInstance`]. That check runs before the offending board's
/// first write, so a refused board contributes nothing at all; boards already
/// loaded from earlier specs stay loaded, and re-running after the file is
/// dealt with is idempotent.
pub fn rebuild_from_artifacts(
    writer: &WriterHandle,
    manifest: &Manifest,
    dir: &Path,
    val_unavailable: Option<&ValUnavailable>,
) -> Result<Vec<RebuiltBoard>, DbError> {
    let mut out = Vec::new();
    for spec in &manifest.boards {
        let raw_path = dir.join(&spec.raw);
        let text = match std::fs::read_to_string(&raw_path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(DbError::Io {
                    path: raw_path.display().to_string(),
                    source,
                })
            }
        };
        let rows = crucible_publish::parse_rows(&text, &raw_path.display().to_string())
            .map_err(DbError::Parse)?;
        out.extend(load_board(
            writer,
            manifest,
            spec,
            dir,
            &raw_path,
            &rows,
            val_unavailable,
        )?);
    }
    writer.flush()?;
    Ok(out)
}

fn load_board(
    writer: &WriterHandle,
    manifest: &Manifest,
    spec: &BoardSpec,
    dir: &Path,
    raw_path: &Path,
    rows: &[RawRow],
    vu: Option<&ValUnavailable>,
) -> Result<Vec<RebuiltBoard>, DbError> {
    let stem = spec.raw.strip_suffix(".jsonl").unwrap_or(&spec.raw);
    let cond_path = dir.join(format!("{stem}.conditions.json"));
    let done_path = dir.join(format!("{stem}.done"));
    let cond = Conditions::load(&cond_path);
    let facts = board_facts(spec, manifest, rows.first());

    // Group by the identity each row declares. In practice this is one group
    // per file; a file that spans two identities gets one pass row each, which
    // is the honest answer rather than a merge.
    let mut groups: Vec<(BoardKey, EngineKey, Vec<&RawRow>)> = Vec::new();
    for r in rows {
        let bk = board_key(manifest, spec, r);
        let ek = EngineKey {
            blake3: None,
            ver: r.ver.clone(),
        };
        // position-then-index rather than `iter_mut().find()`: the borrow of
        // `groups` in the scrutinee would still be live in the `None` arm that
        // pushes to it.
        match groups.iter().position(|(b, e, _)| *b == bk && *e == ek) {
            Some(i) => groups[i].2.push(r),
            None => groups.push((bk, ek, vec![r])),
        }
    }
    if groups.is_empty() {
        // An empty raw is still a pass that happened; recording it is what
        // stops the next resume from treating the board as never swept.
        groups.push((
            board_key_from_manifest(manifest, spec),
            EngineKey::default(),
            Vec::new(),
        ));
    }

    // Refuse BEFORE writing anything. A row is addressed by (board, instance,
    // engine, attempt) and the import upserts, so a raw carrying two rows for
    // one instance loads as one row and exports SHORTER than it arrived --
    // silently, with the board's denominator quietly reduced. Two committed
    // raws do exactly this; see `DbError::DuplicateInstance`.
    for (_, _, group) in &groups {
        duplicate_instance(raw_path, group)?;
    }

    let engine_facts = EngineFacts {
        rebuilt: true,
        ..EngineFacts::default()
    };

    let mut out = Vec::new();
    for (i, (bk, ek, group)) in groups.into_iter().enumerate() {
        let (board_id, engine_id) =
            writer.resolve(bk.clone(), facts.clone(), ek.clone(), engine_facts.clone())?;

        for r in &group {
            writer.run(RunRecord {
                board: bk.clone(),
                board_facts: facts.clone(),
                engine: ek.clone(),
                engine_facts: engine_facts.clone(),
                attempt: 1,
                state: RunState::Done,
                // Never `clean`. See the module header.
                banked: false,
                verdict: None,
                timing: TimingQuality::Unknown,
                val_reason: val_reason_for(r, vu),
                row: (**r).clone(),
                measured: Measured::default(),
            })?;
        }

        let reused = group.iter().filter(|r| r.resumed_clean).count();
        // The conditions file describes the whole raw, so it attaches to the
        // first identity only: importing its timeline once per group would
        // double every sample in it.
        let first = i == 0;
        let pass_id = writer.board_pass(BoardPassRec {
            board: bk,
            board_facts: facts.clone(),
            engine: ek,
            engine_facts: engine_facts.clone(),
            started_at: cond.as_ref().and_then(|c| c.started.clone()),
            ended_at: cond.as_ref().and_then(|c| c.ended.clone()),
            verdict: cond.as_ref().map_or(PassVerdict::Unknown, |c| c.verdict),
            ran: group.len() as i64,
            reused: reused as i64,
            done_marker: done_path.exists().then(|| done_path.display().to_string()),
            raw_path: Some(raw_path.display().to_string()),
            conditions_path: (first && cond.is_some()).then(|| cond_path.display().to_string()),
            // The EFFECTIVE padding, not the raw key: NULL here means "no
            // conditions file", and never "a file that did not say".
            sample_interval: if first {
                cond.as_ref().map(|c| c.interval)
            } else {
                None
            },
            source_path: Some(raw_path.display().to_string()),
        })?;

        let mut samples = 0usize;
        if first {
            if let Some(c) = &cond {
                for (at, idle, comp) in &c.timeline {
                    writer.sample(SampleRec {
                        at: *at,
                        idle_pct: *idle,
                        competitors_total: *comp,
                        pass_id: Some(pass_id),
                        ..SampleRec::default()
                    });
                    samples += 1;
                }
            }
        }

        out.push(RebuiltBoard {
            board_name: spec.id.clone(),
            raw_path: raw_path.to_path_buf(),
            board_id,
            engine_id,
            pass_id,
            rows: group.len(),
            reused,
            verdict: cond.as_ref().map_or(PassVerdict::Unknown, |c| c.verdict),
            samples,
        });
    }
    writer.flush()?;
    Ok(out)
}

/// Refuse a group of rows that cannot be stored without losing one.
///
/// The key is the one `run` is addressed by -- (variant, instance) inside a
/// board and engine -- and the instance half is the LABEL, not its first digit
/// group, which is the whole reason `instance.label` is text. The check counts
/// rather than merely detecting, because the number is what tells a reader
/// whether they are looking at the known 0.19-era label collapse (six rows per
/// key on `line-exchange-snp-numeric-2026`) or at something new.
fn duplicate_instance(raw_path: &Path, group: &[&RawRow]) -> Result<(), DbError> {
    let mut seen: std::collections::BTreeMap<(Option<&str>, &str, String), usize> =
        std::collections::BTreeMap::new();
    for r in group {
        let key = (
            r.ipc.as_deref(),
            r.variant.as_str(),
            InstanceKey::of(&r.instance).label,
        );
        *seen.entry(key).or_insert(0) += 1;
    }
    if let Some(((_, variant, instance), count)) = seen.into_iter().find(|(_, n)| *n > 1) {
        return Err(DbError::DuplicateInstance {
            path: raw_path.display().to_string(),
            variant: variant.to_string(),
            instance,
            count,
        });
    }
    Ok(())
}

/// Render one board back to the bytes `ipc67.py` would have written.
///
/// The order is canonical -- see [`Reader::export_rows`] -- so a crucible raw
/// is diffable against a Python one even though the scheduler reordered
/// execution.
pub fn export(reader: &Reader, board_id: i64, engine_id: i64) -> Result<String, DbError> {
    let rows = reader.export_rows(board_id, engine_id)?;
    let mut out = String::new();
    for r in &rows {
        crucible_publish::write_row(r, &mut out);
        out.push('\n');
    }
    Ok(out)
}

/// Write one board to a path. Separate from [`export`] so a caller can diff
/// without touching the filesystem.
pub fn export_to(
    reader: &Reader,
    board_id: i64,
    engine_id: i64,
    path: &Path,
) -> Result<(), DbError> {
    let text = export(reader, board_id, engine_id)?;
    std::fs::write(path, text).map_err(|source| DbError::Io {
        path: path.display().to_string(),
        source,
    })
}

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

/// The identity a row declares, falling back to the manifest for every stamp it
/// does not carry. Boards swept before 0.25 carry none of them.
fn board_key(m: &Manifest, spec: &BoardSpec, r: &RawRow) -> BoardKey {
    let mut k = board_key_from_manifest(m, spec);
    // `r.get("budget") or budget` -- Python's `or`, so a stamp of ZERO falls
    // back to the manifest exactly as a missing one does. This is the same rule
    // `Referee::budget_for` pins, and it has to be the same rule: the board's
    // `budget_secs` is what a timeout is denominated in, and a board keyed at
    // 0 s classifies every row it holds as a timeout.
    if r.present.budget {
        if let Some(b) = r.budget.filter(|b| *b != 0.0) {
            k.budget_secs = b;
        }
    }
    if r.present.stamps {
        // `MODE or "auto"` -- and `or` fires on the EMPTY STRING too, not just
        // on None. The resume gate normalises both sides before comparing, so a
        // row stamped "auto", one stamped "" and one whose mode was unset are
        // one board, not three.
        k.mode = match r.mode.as_deref() {
            Some("") | None => "auto".to_string(),
            Some(mode) => mode.to_string(),
        };
        if let Some(j) = r.jobs {
            k.jobs = j;
        }
        // `str(r.get("threads"))` is the gate's currency, and `str(None)` is
        // the literal "None" -- which matches no manifest board, so such a row
        // never gets reused. Keeping that currency here means a row that
        // carried no thread count gets an identity of its own instead of being
        // quietly filed under the manifest's, which would credit it with a
        // setting it never recorded.
        k.threads = r
            .threads
            .as_ref()
            .map_or_else(|| "None".to_string(), py_str);
    }
    k
}

/// The identity a board has BEFORE any row exists for it: what the manifest
/// says it sweeps under. A live sweep starts from this and fills in the one
/// thing the manifest alone cannot canonicalise for it, the environment.
pub fn board_key_from_manifest(m: &Manifest, spec: &BoardSpec) -> BoardKey {
    let threads = spec.threads.unwrap_or(m.defaults.threads);
    BoardKey {
        name: spec.id.clone(),
        // The ARMED wall, which is `timeout_secs` when a tier move is in
        // flight and the manifest's scored budget only when it is not.
        budget_secs: spec.timeout_secs.unwrap_or(m.defaults.timeout_secs) as f64,
        mode: spec.mode.clone().unwrap_or_else(|| m.defaults.mode.clone()),
        jobs: spec.jobs.unwrap_or(m.defaults.jobs),
        threads: threads.to_string(),
        // Empty because the artifacts do not record it. See the module header.
        env: "{}".to_string(),
        args: serde_json::to_string(&spec.extra_args).unwrap_or_else(|_| "[]".to_string()),
    }
}

/// The reporting-only columns, from the spec and -- when one is in hand -- the
/// first row's own `threads` token.
pub fn board_facts(spec: &BoardSpec, m: &Manifest, first: Option<&RawRow>) -> BoardFacts {
    let threads_json = match first.and_then(|r| r.present.stamps.then_some(r)) {
        Some(r) => r
            .threads
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
            .unwrap_or_else(|| "null".into()),
        // `ipc67.py` passes the CLI argument through unconverted, so an
        // unstamped board's threads would have been written as a JSON string.
        None => format!("\"{}\"", spec.threads.unwrap_or(m.defaults.threads)),
    };
    BoardFacts {
        label: Some(spec.label.clone()),
        competition: Some(spec.competition.clone()),
        proof_track: spec.proof_track,
        threads_json,
    }
}

/// Python's `str()` on the `threads` value: a JSON string yields its contents,
/// anything else its literal form. `str(2)` and `str("2")` are both `"2"`,
/// which is exactly why the resume gate compares in this currency.
fn py_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Why validation was unavailable, as far as an artifact can say.
///
/// Only one reason is inferable from a raw: a domain VAL cannot INGEST, which
/// `benchmarks/val-unavailable.json` names. Everything else stays NULL, which
/// is honest -- it means "no reason recorded" and never "valid".
fn val_reason_for(r: &RawRow, vu: Option<&ValUnavailable>) -> Option<ValReason> {
    if r.val.is_none() && vu.is_some_and(|v| v.contains(r)) {
        Some(ValReason::Ingest)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// The sibling conditions file
// ---------------------------------------------------------------------------

/// What `contention.py` wrote beside a board raw.
struct Conditions {
    started: Option<String>,
    ended: Option<String>,
    verdict: PassVerdict,
    /// The EFFECTIVE padding, never the raw key. Python is
    /// `float(cond.get("interval") or 20)`, and `or` is falsy: an absent key,
    /// a null and a recorded ZERO all mean twenty seconds. 72 of the 76
    /// conditions files in this repo have no `interval` at all, so this is the
    /// normal case rather than the corner one -- and storing a zero here would
    /// hand the resume gate a window with no padding, which consults fewer
    /// samples than the Python does and can read a contended window as clean.
    interval: f64,
    /// `[epoch_ts, idle_pct, competitors_total_pcpu]`, present on 4 of the 76
    /// files in this repo.
    timeline: Vec<(f64, Option<f64>, Option<f64>)>,
}

impl Conditions {
    /// A missing or unreadable conditions file degrades to `None`, the way
    /// `load_resume` degrades on the same input. An absent watcher is a known
    /// state of the world, not a corrupt record.
    fn load(path: &Path) -> Option<Conditions> {
        let text = std::fs::read_to_string(path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&text).ok()?;
        let obj = v.as_object()?;
        let string = |k: &str| obj.get(k).and_then(|x| x.as_str()).map(str::to_string);
        let mut timeline = Vec::new();
        if let Some(arr) = obj.get("timeline").and_then(|x| x.as_array()) {
            for t in arr {
                // `load_resume` requires len >= 3 and a non-null timestamp, and
                // discards anything else rather than guessing at it.
                let Some(row) = t.as_array() else { continue };
                if row.len() < 3 {
                    continue;
                }
                if row[0].is_null() {
                    continue;
                }
                // A timestamp that is present but not a number is a THIRD
                // thing. Python reaches the comparison and dies with a
                // TypeError, taking the pass with it; dropping the sample here
                // would be the one direction that is unsafe, because the
                // sample dropped might be the dirty one that would have
                // refused a reuse. Refuse the whole file instead --
                // `sched::resume::Conditions::parse` makes the same call, and
                // two readers of one document that disagree is how this
                // project loses a number.
                let at = row[0].as_f64()?;
                timeline.push((at, row[1].as_f64(), row[2].as_f64()));
            }
        }
        Some(Conditions {
            started: string("started"),
            ended: string("ended"),
            verdict: obj
                .get("verdict")
                .and_then(|x| x.as_str())
                .map_or(PassVerdict::Unknown, PassVerdict::parse),
            // `float(cond.get("interval") or 20)` -- zero is falsy in Python
            // and must fall back here too. The constant is imported from the
            // resume gate rather than re-typed: a second `20.0` is the drift.
            interval: match obj.get("interval").and_then(|x| x.as_f64()) {
                Some(i) if i != 0.0 => i,
                _ => crate::sched::resume::DEFAULT_INTERVAL_SECS,
            },
            timeline,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rule that cannot be relaxed: a conditions file with no `verdict` --
    /// or with one this build does not know -- must not read as clean.
    #[test]
    fn a_conditions_file_without_a_verdict_is_unknown() {
        let dir = std::env::temp_dir().join(format!("crucible-cond-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("x.conditions.json");
        std::fs::write(&p, r#"{"samples": 3}"#).unwrap();
        let c = Conditions::load(&p).expect("parsed");
        assert_eq!(c.verdict, PassVerdict::Unknown);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A timeline entry that `load_resume` would discard must be discarded
    /// here too, or the database ends up holding samples the Python never
    /// counted and the two gates disagree.
    #[test]
    fn short_and_null_timeline_rows_are_dropped() {
        let dir = std::env::temp_dir().join(format!("crucible-tl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("y.conditions.json");
        std::fs::write(
            &p,
            r#"{"verdict":"clean","interval":20.0,
                "timeline":[[1.0,80.0,13.5],[2.0,70.0],[null,1.0,2.0],[3.0,null,null]]}"#,
        )
        .unwrap();
        let c = Conditions::load(&p).expect("parsed");
        assert_eq!(c.timeline.len(), 2);
        assert_eq!(c.timeline[0], (1.0, Some(80.0), Some(13.5)));
        // A sample whose competitor total could not be attributed is KEPT and
        // is dirty; it is only a NULL TIMESTAMP that makes a row unusable.
        assert_eq!(c.timeline[1], (3.0, None, None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Python's `str()`, which is the currency the resume gate compares
    /// threads in: `str(2)` and `str("2")` are the same string.
    #[test]
    fn threads_compare_as_python_strings() {
        assert_eq!(py_str(&serde_json::json!("2")), "2");
        assert_eq!(py_str(&serde_json::json!(2)), "2");
    }

    /// `float(cond.get("interval") or 20)`. The `or` is the whole test: 72 of
    /// the 76 conditions files in this repo have no `interval` key, and a
    /// recorded ZERO is falsy in Python too. Storing either as "no padding"
    /// narrows the resume gate's window, which consults fewer samples than the
    /// Python does and can read a contended window as clean.
    #[test]
    fn an_absent_or_zero_interval_is_twenty_seconds() {
        let dir = std::env::temp_dir().join(format!("crucible-iv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("i.conditions.json");
        let interval_of = |body: &str| {
            std::fs::write(&p, body).unwrap();
            Conditions::load(&p).expect("parsed").interval
        };
        let default = crate::sched::resume::DEFAULT_INTERVAL_SECS;
        assert_eq!(interval_of(r#"{"verdict":"clean"}"#), default, "absent");
        assert_eq!(interval_of(r#"{"interval":null}"#), default, "null");
        assert_eq!(interval_of(r#"{"interval":0}"#), default, "zero is falsy");
        assert_eq!(interval_of(r#"{"interval":5.0}"#), 5.0, "a real one stands");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A raw that carries two rows for one instance cannot be cached, and must
    /// say so rather than load short.
    ///
    /// `benchmarks/air/ipc2026-numeric.jsonl` and its 0.19 sibling are 320-row
    /// boards with 288 distinct keys, written before `ipc67.py` learned to keep
    /// every digit group of a multipart filename. Upserted into a table keyed
    /// on (board, instance, engine, attempt) they load as 288 rows and export
    /// as 288 lines -- a board 32 instances light, with nothing in the output
    /// saying so.
    #[test]
    fn a_raw_with_two_rows_for_one_instance_is_refused() {
        let one = |t: f64| {
            format!(
                "{{\"ipc\": \"ipc-2026n\", \"variant\": \"line-exchange-snp-numeric-2026\", \
                 \"instance\": 3, \"solved\": false, \"time\": {t}, \"metric\": null, \
                 \"length\": null, \"val\": null, \"notes\": null}}"
            )
        };
        let text = format!("{}\n{}\n", one(28.39), one(27.97));
        let rows = crucible_publish::parse_rows(&text, "x.jsonl").expect("parsed");
        let refs: Vec<&RawRow> = rows.iter().collect();
        match duplicate_instance(Path::new("x.jsonl"), &refs) {
            Err(DbError::DuplicateInstance {
                variant,
                instance,
                count,
                ..
            }) => {
                assert_eq!(variant, "line-exchange-snp-numeric-2026");
                assert_eq!(instance, "3");
                assert_eq!(count, 2);
            }
            other => panic!("a duplicated instance was accepted: {other:?}"),
        }

        // ...and a board whose instances are distinct is untouched, including
        // one whose labels differ only past the FIRST digit group -- which is
        // the whole reason the label is stored as text.
        let ok = "{\"ipc\": \"ipc-2026n\", \"variant\": \"v\", \"instance\": \"3_10\", \
                  \"solved\": false, \"time\": null, \"metric\": null, \"length\": null, \
                  \"val\": null, \"notes\": null}\n\
                  {\"ipc\": \"ipc-2026n\", \"variant\": \"v\", \"instance\": \"3_11\", \
                  \"solved\": false, \"time\": null, \"metric\": null, \"length\": null, \
                  \"val\": null, \"notes\": null}\n";
        let rows = crucible_publish::parse_rows(ok, "y.jsonl").expect("parsed");
        let refs: Vec<&RawRow> = rows.iter().collect();
        assert!(duplicate_instance(Path::new("y.jsonl"), &refs).is_ok());
    }
}
