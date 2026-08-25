// Included into every test binary, so any helper a given binary does not use
// reads as dead code there. The repo's one precedent for this attribute is
// crates/ferroplan-bevy/src/palette.rs.
#![allow(dead_code)]

//! Shared fixture loading. Paths are relative to this crate's manifest dir, the
//! way `crates/ferroplan/tests/fluent_fold.rs` reaches `benchmarks/bench/`.

use crucible_publish::{parse_rows, RawRow, ValUnavailable};

pub const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
pub const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");

pub fn incident(name: &str) -> Vec<RawRow> {
    let p = format!("{FIXTURES}/incidents/{name}.jsonl");
    let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    parse_rows(&src, &p).unwrap_or_else(|e| panic!("{e}"))
}

/// The real, committed `benchmarks/val-unavailable.json`.
pub fn real_val_map() -> ValUnavailable {
    let p = format!("{REPO}/benchmarks/val-unavailable.json");
    let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"));
    let doc: serde_json::Value = serde_json::from_str(&src).unwrap();
    ValUnavailable::new(
        doc["unavailable"]
            .as_object()
            .expect("val-unavailable.json has an `unavailable` object")
            .keys()
            .cloned(),
    )
}
