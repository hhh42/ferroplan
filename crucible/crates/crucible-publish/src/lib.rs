//! The publication half of `crucible`: everything that turns per-board rows
//! into the numbers this project publishes.
//!
//! Deliberately **pure**. No database, no terminal, no platform calls, no
//! clock. The golden battery is the only thing that proves this is a *port* of
//! `benchmarks/standings.py` rather than a rewrite of it, so it has to run on
//! any box, in seconds, against committed fixtures.
//!
//! The Python it ports is kept, not retired -- `docs/roadmap-0.26.md` records
//! why: it is the only independent implementation of this taxonomy that exists,
//! and every incident in its comment corpus is a case of one implementation
//! drifting from another unobserved.

pub mod archive;
pub mod bounds;
pub mod class;
pub mod compare;
pub mod field;
pub mod fmt;
pub mod history;
pub mod manifest;
pub mod promote;
pub mod pyjson;
pub mod quality;
pub mod raw;
pub mod referee;
pub mod render;
pub mod snapshot;

pub use class::{Class, Coverage};
pub use raw::{write_row, Instance, Notes, Present, RawRow};
pub use referee::{Referee, ValUnavailable, TIMEOUT_FRAC};

/// Read a `.jsonl` board raw, naming the file and line on a bad row.
///
/// Truncated tail lines are SKIPPED, not fatal: a pass killed mid-write leaves
/// one, and `ipc67.py:566-569` skips it too. Everything else is a parse error
/// with a location, where Python would raise a bare `KeyError`.
pub fn parse_rows(src: &str, path: &str) -> Result<Vec<RawRow>, String> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();
    let last = lines.len().saturating_sub(1);
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => {
                let present = v.as_object().map(Present::of).unwrap_or_default();
                match serde_json::from_value::<RawRow>(v) {
                    Ok(mut r) => {
                        r.present = present;
                        out.push(r);
                    }
                    Err(e) => return Err(format!("{path}:{}: {e}", i + 1)),
                }
            }
            Err(e) => {
                // Only the final line may be a truncated write: a pass killed
                // mid-write leaves one, and `ipc67.py:566-569` skips it too.
                if i == last {
                    continue;
                }
                return Err(format!("{path}:{}: {e}", i + 1));
            }
        }
    }
    Ok(out)
}
