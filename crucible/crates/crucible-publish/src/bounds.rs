//! Best-known cost bounds for the modern boards, ported from
//! `benchmarks/standings.py` (`load_bounds` :957-979, read by
//! `bounds_quality` :981-1002 and the `MODERN_Q` table :1009-1016).
//!
//! This module answers exactly one question -- "what is the best cost anyone is
//! known to have achieved on this instance?" -- and every wrong answer it can
//! give has already been published once by this project, or was one careless
//! line away from being.
//!
//! **The 2023 key is track-scoped.** `agl/`, `sat/` and `opt/` carry DIFFERENT
//! instance sets under the SAME domain names, so `folding/p07` means three
//! unrelated problems. Until 0.25 only the agile half of that corpus was
//! vendored and a bare `"2023"` year key was harmless; the cycle that vendored
//! the real satisficing and optimal halves made it a silent cross-track join,
//! where an agile bound would referee a satisficing plan. The key grew a track
//! (`"2023-agl"`, `"2023-sat"`, `"2023-opt"`) in the same change, and nothing
//! in this file may ever mint a bare `"2023"`.
//!
//! **The 2023 file gives `[lo, hi]` and we take `hi`.** `lo` is a lower bound
//! -- a proof that nothing cheaper exists -- and it is `0` wherever nobody
//! proved anything. In the vendored file as ported, 159 of 417 entries have
//! `lo != hi` and every one of them has `lo == 0`: all of `labyrinth`,
//! `quantum-layout`, `recharging-robots` and `rubiks-cube`. A port that took
//! the first number would score four entire domains against a reference of
//! zero, and `bounds_quality`'s `min(ref/ours, 1.0)` would drag their mean
//! quality to 0.00 without erroring once.
//!
//! **The 2018 file is a flat list with several entries per instance, and we
//! take the minimum.** In the vendored file every `sat/` instance appears
//! exactly twice, ascending -- a lower and an upper bound flattened into two
//! rows -- so the minimum is the LOWER of the two. That is the opposite
//! convention from 2023's `hi`, and it is deliberately preserved here: it can
//! only depress our ratio and cost us wins, it is what every published 2018
//! quality column was computed under (`benchmarks/ipc-standings.md`: "0W/1T/28L,
//! mean quality 0.77"), and harmonising the two conventions would move a
//! published number. That is a cycle decision with a recorded before/after, not
//! a tidy-up to slip into a port.
//!
//! **The 2018 pattern reads `sat/` only, and the `opt/` half of that same file
//! is dropped on purpose.** 0.25 vendored the 2018 optimal corpus, but its
//! bounds stayed unreadable here, because the 2018 year key is bare `"2018"` --
//! feeding `opt/` costs into it would recreate precisely the cross-track join
//! the 2023 key was scoped to prevent. The `2018 seq-opt` board renders the
//! proof-rate note instead of a quality column, so nothing is lost today; a
//! future optimal quality column must scope the 2018 key FIRST and only then
//! widen this pattern.
//!
//! Both inputs live under `benchmarks/.ipc-corpus/`, which is **gitignored**.
//! On a clean clone they are simply absent, and every bounds-scored board
//! degrades to coverage-only. That is a required behaviour, not an error path:
//! `load` never fails on a missing corpus.
//!
//! `benchmarks/crucible-differential.py` diffs `classify` and `coverage` only,
//! so nothing here is covered by the 42,356-row harness. The fixtures at the
//! bottom of this file are the whole of this transform's defence.

use std::collections::BTreeMap;
use std::path::Path;

/// `(year_key, domain, instance)`. The year key is `"2018"` or one of the
/// track-scoped `"2023-{agl,sat,opt}"` -- never a bare `"2023"`.
pub type BoundKey = (String, String, u64);

/// The 2018 year key. Bare, because that file's `opt/` half is not read; see
/// the module header before changing either fact.
pub const YEAR_2018: &str = "2018";

/// The three halves of the 2023 dataset, each its own instance set.
pub const TRACKS_2023: [&str; 3] = ["agl", "sat", "opt"];

/// Best-known cost per instance, joined by [`BoundKey`].
///
/// A `BTreeMap` rather than a hash map: lookup is the only thing callers do, so
/// iteration order is not observable today, and an ordered map keeps it
/// unobservable if someone later dumps the table into an audit record.
#[derive(Debug, Clone, Default)]
pub struct BestKnownBounds {
    by_key: BTreeMap<BoundKey, f64>,
    problems: Vec<String>,
}

impl BestKnownBounds {
    /// Load both bounds files from a corpus root (`benchmarks/.ipc-corpus`).
    ///
    /// A missing file contributes nothing and is not a problem -- Python guards
    /// each read with `os.path.exists` for the same reason. A file that exists
    /// but cannot be read or parsed also degrades to nothing, but is recorded
    /// in [`problems`](Self::problems), because there Python would have raised
    /// and stopped the regeneration outright. **A caller that publishes a
    /// number off these bounds must check `problems()` and refuse**: silently
    /// losing one instance's reference changes a mean quality without changing
    /// anything visible.
    pub fn load(corpus_root: &Path) -> Self {
        let p23 = corpus_root.join("ipc-2023").join("bounds.json");
        let p18 = corpus_root.join("ipc-2018").join("cost_bounds.json");
        let mut out = Self::default();
        // 2023 first, then 2018 -- the Python's order. The two year-key spaces
        // are disjoint by construction, so the order cannot decide a value; it
        // is kept anyway so any future overlap fails the same way in both
        // implementations rather than differently.
        if let Some(text) = out.slurp(&p23, "ipc-2023/bounds.json") {
            out.absorb_2023(&text, "ipc-2023/bounds.json");
        }
        if let Some(text) = out.slurp(&p18, "ipc-2018/cost_bounds.json") {
            out.absorb_2018(&text, "ipc-2018/cost_bounds.json");
        }
        out
    }

    /// The same loader over file CONTENTS, so the fixtures below (and any
    /// caller holding the bytes already) need no filesystem. `None` is an
    /// absent file.
    pub fn from_sources(bounds_2023: Option<&str>, cost_bounds_2018: Option<&str>) -> Self {
        let mut out = Self::default();
        if let Some(text) = bounds_2023 {
            out.absorb_2023(text, "ipc-2023/bounds.json");
        }
        if let Some(text) = cost_bounds_2018 {
            out.absorb_2018(text, "ipc-2018/cost_bounds.json");
        }
        out
    }

    /// True when no bound was loaded at all -- the clean-clone case, in which
    /// every bounds-scored board must render coverage-only.
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn get(&self, k: &BoundKey) -> Option<f64> {
        self.by_key.get(k).copied()
    }

    /// The best known cost for one instance -- [`get`](Self::get) without
    /// making the caller build the tuple. It still allocates the two strings; a
    /// bounds lookup happens once per solved row, so this is not a place that
    /// needs borrowed keys.
    pub fn best(&self, year_key: &str, domain: &str, instance: u64) -> Option<f64> {
        self.get(&(year_key.to_string(), domain.to_string(), instance))
    }

    /// Inputs that existed but could not be read as expected. Empty on a clean
    /// clone, where the files are simply absent.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    fn slurp(&mut self, path: &Path, label: &str) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            // The clean-clone case, and the only absence that is silent.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                self.problems.push(format!("{label}: unreadable ({e})"));
                None
            }
        }
    }

    /// `{path: [lo, hi]}` -- take `hi`, key by track.
    fn absorb_2023(&mut self, text: &str, label: &str) {
        let parsed: OrderedObject = match serde_json::from_str(text) {
            Ok(o) => o,
            Err(e) => {
                self.problems
                    .push(format!("{label}: not an object of path -> [lo, hi] ({e})"));
                return;
            }
        };
        for (path, value) in parsed.0 {
            // Shape check first, where Python's `for path, (_, hi) in ...`
            // destructure sits: a value that is not a pair is a complaint even
            // on a path this pattern would have skipped anyway.
            let Some(hi) = pair(&value).map(|(_lo, hi)| hi) else {
                self.problems.push(format!(
                    "{label}: {path:?}: expected [lo, hi], found {value}"
                ));
                continue;
            };
            // A null upper bound means nobody has a plan for this instance. Not
            // an error; it simply mints no key, and the row scores coverage-only.
            if hi.is_null() {
                continue;
            }
            let Some((track, domain, instance)) = split_bound_path(&path, &TRACKS_2023) else {
                continue;
            };
            // Python is `float(hi)`, which is WIDER than `as_f64`: it also
            // reads a quoted number (`"10"`) and a bool (`True` -> 1.0), and
            // it yields `inf` for a token that overflows f64 -- where
            // `as_f64` returns nothing for the first two and, under the
            // workspace's `arbitrary_precision` feature, filters the third out
            // as non-finite. Every value in both vendored files is a bare int
            // (417 and 180, checked), so the gap is unreachable today. Where
            // it is reachable this arm refuses LOUDLY -- a recorded problem
            // the publisher must check -- rather than guessing at a coercion
            // whose `float()` fidelity (whitespace, underscores, "infinity",
            // Unicode digits) could not be defended. It is the ONE arm here
            // that complains where Python would NOT have raised.
            let Some(hi) = hi.as_f64() else {
                self.problems
                    .push(format!("{label}: {path:?}: non-numeric upper bound {hi}"));
                continue;
            };
            // Last writer wins, exactly as assignment into a Python dict does.
            // Two distinct paths can only collide here through the trailing
            // slack in the pattern (see `split_bound_path`), so this decides
            // nothing in the vendored file -- but it decides it identically.
            self.by_key
                .insert((format!("2023-{track}"), domain.to_string(), instance), hi);
        }
    }

    /// `[[path, cost], ...]` -- several entries per instance, keep the minimum.
    fn absorb_2018(&mut self, text: &str, label: &str) {
        let parsed: Vec<serde_json::Value> = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                self.problems
                    .push(format!("{label}: not a list of [path, cost] ({e})"));
                return;
            }
        };
        for entry in parsed {
            let Some((path, cost)) = pair(&entry) else {
                self.problems
                    .push(format!("{label}: expected [path, cost], found {entry}"));
                continue;
            };
            let Some(path) = path.as_str() else {
                // Python reaches `re.match` with a non-string here and dies of
                // a TypeError; the entry is unusable either way.
                self.problems
                    .push(format!("{label}: non-string path {path}"));
                continue;
            };
            if cost.is_null() {
                continue;
            }
            // `sat/` only -- the `opt/` half of this same file is dropped; see
            // the module header for why widening this is not a one-line change.
            let Some((_track, domain, instance)) = split_bound_path(path, &["sat"]) else {
                continue;
            };
            // Wider in Python for the same reasons as the 2023 upper bound
            // above; the note there is the whole argument.
            let Some(cost) = cost.as_f64() else {
                self.problems
                    .push(format!("{label}: {path:?}: non-numeric cost {cost}"));
                continue;
            };
            let key = (YEAR_2018.to_string(), domain.to_string(), instance);
            match self.by_key.entry(key) {
                std::collections::btree_map::Entry::Occupied(mut e) => {
                    // Python is `min(best.get(k, inf), float(cost))`, and
                    // `min` keeps its FIRST argument unless the second is
                    // strictly smaller. Written as a strict `<` so a tie -- or
                    // a NaN, which compares false against everything -- leaves
                    // the incumbent in place, as it does there.
                    if cost < *e.get() {
                        e.insert(cost);
                    }
                }
                std::collections::btree_map::Entry::Vacant(v) => {
                    v.insert(cost);
                }
            }
        }
    }
}

/// A JSON value that is an array of exactly two elements, as `(first, second)`.
///
/// Python destructures both files' entries into a 2-tuple, which raises on any
/// other length; this is the same test, minus the raising.
fn pair(v: &serde_json::Value) -> Option<(&serde_json::Value, &serde_json::Value)> {
    match v.as_array() {
        Some(a) if a.len() == 2 => Some((&a[0], &a[1])),
        _ => None,
    }
}

/// Split `"<track>/<domain>/p<NN>.pddl"` into its three parts.
///
/// This hand-rolls Python's `re.match(r"(agl|sat|opt)/([\w-]+)/p(\d+)\.pddl")`
/// (and the 2018 file's `sat/`-only twin), and matches it including its edges:
///
/// * `re.match` anchors only at the START, so trailing slack after `.pddl` is
///   accepted there and is accepted here. Nothing in the vendored files has
///   any, and keeping the slack means a corpus that grows a suffix cannot make
///   the two implementations disagree about which instances have bounds.
/// * `[\w-]` cannot match `/`, so a deeper path (`sat/a/b/p01.pddl`) matches
///   nothing -- backtracking cannot rescue it, since a shorter domain must
///   still be followed by `/p`.
/// * The domain class is Python's Unicode `\w` plus `-`. Every vendored domain
///   is ASCII; using `is_alphanumeric` rather than `is_ascii_alphanumeric`
///   means a future non-ASCII domain is taken here exactly as Python takes it,
///   instead of quietly losing its bound.
/// * The digit run needs no backtracking: the character after the MAXIMAL run
///   is the only one that can be the `.`, so a greedy scan and a backtracking
///   regex accept the same strings -- over ASCII.
/// * The digit class is the ONE place this is NARROWER than the regex, and it
///   goes the opposite way from the domain class above, so it is written down
///   rather than left to be rediscovered. Python's `\d` also matches Unicode
///   decimal digits, and `int` reads them: `p١٢.pddl` matches there and keys
///   instance 12, while `is_ascii_digit` declines it. Rust ships no digit
///   VALUE for non-ASCII `Nd`, so closing this would mean carrying a Unicode
///   table to key an instance the runner cannot emit -- its own instance ids
///   are parsed from these same ASCII filenames, and every vendored path in
///   both files is ASCII (checked).
///
/// Leading zeros are dropped by the parse (`p07` -> `7`), which is what makes
/// this key join the runner's integer `instance`.
fn split_bound_path<'a>(path: &'a str, tracks: &[&str]) -> Option<(&'a str, &'a str, u64)> {
    let (track, rest) = path.split_once('/')?;
    if !tracks.contains(&track) {
        return None;
    }
    let (domain, file) = rest.split_once('/')?;
    if domain.is_empty()
        || !domain
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let file = file.strip_prefix('p')?;
    let end = file
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(file.len());
    let (digits, tail) = file.split_at(end);
    if digits.is_empty() || !tail.starts_with(".pddl") {
        return None;
    }
    // An instance number too wide for u64 is dropped rather than wrapped.
    // Python's ints are unbounded, but a `p` number that big is not an
    // instance, and the runner could not have produced a row to join it to.
    let instance = digits.parse::<u64>().ok()?;
    Some((track, domain, instance))
}

/// A JSON object read in DOCUMENT order.
///
/// `serde_json`'s own `Map` is a `BTreeMap` here (no `preserve_order` feature),
/// which would sort the paths -- and Python dicts iterate in insertion order.
/// Insertion order is observable through the last-writer-wins assignment in
/// `absorb_2023`, so the difference is only invisible while no two paths derive
/// the same key. That is not a property to lean on: reading the object in file
/// order costs twenty lines and removes the question.
struct OrderedObject(Vec<(String, serde_json::Value)>);

impl<'de> serde::Deserialize<'de> for OrderedObject {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = OrderedObject;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON object of path -> [lo, hi]")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut m: A,
            ) -> Result<OrderedObject, A::Error> {
                let mut out = Vec::with_capacity(m.size_hint().unwrap_or(0));
                while let Some(kv) = m.next_entry::<String, serde_json::Value>()? {
                    out.push(kv);
                }
                Ok(OrderedObject(out))
            }
        }
        d.deserialize_map(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(year: &str, domain: &str, instance: u64) -> BoundKey {
        (year.to_string(), domain.to_string(), instance)
    }

    /// The 2023 file gives `[lo, hi]` and the reference is `hi`. `lo` is 0
    /// wherever nothing was proven -- 159 of 417 vendored entries, all of
    /// labyrinth / quantum-layout / recharging-robots / rubiks-cube -- so
    /// taking the first number scores four domains against zero.
    #[test]
    fn takes_the_upper_bound_never_the_lower() {
        let b = BestKnownBounds::from_sources(
            Some(r#"{"agl/labyrinth/p01.pddl": [0, 10], "agl/folding/p01.pddl": [10, 10]}"#),
            None,
        );
        assert_eq!(b.get(&key("2023-agl", "labyrinth", 1)), Some(10.0));
        assert_eq!(b.get(&key("2023-agl", "folding", 1)), Some(10.0));
        assert!(b.problems().is_empty());
    }

    /// Python guards these two reads with `hi is not None` and `cost is not
    /// None` -- an identity test, NOT truthiness. Zero is a real best-known
    /// cost (an instance whose goals already hold costs nothing to reach), and
    /// it is the value every convenient Rust spelling swallows: a
    /// `filter(|v| *v > 0.0)`, an `unwrap_or_default` read back as "absent", a
    /// `!= 0.0` guard borrowed from the `if ours` check that genuinely IS
    /// falsy over in `bounds_quality`. Swallowing it does not error -- it
    /// silently drops that instance's reference and moves a published mean.
    /// Both files, because the two guards are written separately; and both
    /// orders of the 2018 minimum, because a zero incumbent is exactly what a
    /// truthiness bug would let the next entry overwrite.
    #[test]
    fn a_bound_of_zero_is_a_bound_not_a_missing_one() {
        let b = BestKnownBounds::from_sources(
            Some(r#"{"agl/folding/p01.pddl": [0, 0]}"#),
            Some(
                r#"[["sat/snake/p01.pddl", 0],
                    ["sat/snake/p02.pddl", 0], ["sat/snake/p02.pddl", 5],
                    ["sat/snake/p03.pddl", 5], ["sat/snake/p03.pddl", 0]]"#,
            ),
        );
        assert_eq!(b.get(&key("2023-agl", "folding", 1)), Some(0.0));
        assert_eq!(b.get(&key("2018", "snake", 1)), Some(0.0));
        assert_eq!(b.get(&key("2018", "snake", 2)), Some(0.0));
        assert_eq!(b.get(&key("2018", "snake", 3)), Some(0.0));
        assert_eq!(b.len(), 4);
        assert!(b.problems().is_empty(), "zero is not a complaint");
    }

    /// The 0.25 incident: agl/, sat/ and opt/ carry different instances under
    /// the same domain names, so the year key is track-scoped and a bare
    /// "2023" key must not exist at all.
    #[test]
    fn the_2023_key_is_track_scoped() {
        let b = BestKnownBounds::from_sources(
            Some(
                r#"{"agl/folding/p07.pddl": [1, 11],
                    "sat/folding/p07.pddl": [2, 22],
                    "opt/folding/p07.pddl": [3, 33]}"#,
            ),
            None,
        );
        assert_eq!(b.get(&key("2023-agl", "folding", 7)), Some(11.0));
        assert_eq!(b.get(&key("2023-sat", "folding", 7)), Some(22.0));
        assert_eq!(b.get(&key("2023-opt", "folding", 7)), Some(33.0));
        assert_eq!(b.get(&key("2023", "folding", 7)), None);
        assert_eq!(b.len(), 3);
    }

    /// The 2018 file lists an instance more than once and the best known is the
    /// MINIMUM -- and which entry came first in the file must not decide it.
    #[test]
    fn the_2018_duplicates_collapse_to_their_minimum() {
        let ascending = BestKnownBounds::from_sources(
            None,
            Some(r#"[["sat/caldera/p01.pddl", 11], ["sat/caldera/p01.pddl", 13]]"#),
        );
        let descending = BestKnownBounds::from_sources(
            None,
            Some(r#"[["sat/caldera/p01.pddl", 13], ["sat/caldera/p01.pddl", 11]]"#),
        );
        assert_eq!(ascending.get(&key("2018", "caldera", 1)), Some(11.0));
        assert_eq!(descending.get(&key("2018", "caldera", 1)), Some(11.0));
    }

    /// The `opt/` half of the 2018 file is dropped: the 2018 year key is bare,
    /// so admitting optimal costs would make one key mean two tracks -- the
    /// exact join the 2023 key was scoped to prevent.
    #[test]
    fn the_2018_optimal_half_is_dropped() {
        let b = BestKnownBounds::from_sources(
            None,
            Some(
                r#"[["opt/termes/p02.pddl", 40],
                    ["opt/termes/p02.pddl", 44],
                    ["sat/termes/p02.pddl", 90]]"#,
            ),
        );
        assert_eq!(b.get(&key("2018", "termes", 2)), Some(90.0));
        assert_eq!(b.len(), 1);
        assert!(
            b.problems().is_empty(),
            "a skipped track is not a complaint"
        );
    }

    /// The corpus is gitignored, so on a clean clone BOTH files are absent and
    /// every bounds-scored board must degrade to coverage-only. Absence is a
    /// supported state, never an error.
    #[test]
    fn a_missing_corpus_loads_empty_and_quiet() {
        let b = BestKnownBounds::load(Path::new("/nonexistent/crucible/benchmarks/.ipc-corpus"));
        assert!(b.is_empty());
        assert_eq!(b.len(), 0);
        assert!(b.problems().is_empty());
        assert_eq!(b.get(&key("2018", "caldera", 1)), None);
    }

    /// Half a corpus is a real state too (2018 fetched, 2023 not), and the half
    /// that is present must still score.
    #[test]
    fn one_file_present_is_half_a_table_not_none() {
        let b = BestKnownBounds::from_sources(None, Some(r#"[["sat/snake/p05.pddl", 7]]"#));
        assert!(!b.is_empty());
        assert_eq!(b.get(&key("2018", "snake", 5)), Some(7.0));
        assert_eq!(b.get(&key("2023-agl", "snake", 5)), None);
    }

    /// `p01` keys as instance 1, because the runner records instance numbers as
    /// integers without leading zeros; a string key would join nothing.
    #[test]
    fn leading_zeros_are_dropped_from_the_instance() {
        let b = BestKnownBounds::from_sources(Some(r#"{"sat/spider/p07.pddl": [4, 9]}"#), None);
        assert_eq!(b.get(&key("2023-sat", "spider", 7)), Some(9.0));
    }

    /// Paths the pattern does not take are skipped in silence, exactly as the
    /// regex skips them: a deeper path, a filename that is not `p<digits>`, and
    /// an unknown track prefix.
    #[test]
    fn unmatched_paths_are_skipped_silently() {
        let b = BestKnownBounds::from_sources(
            Some(
                r#"{"agl/nested/dir/p01.pddl": [1, 1],
                    "agl/folding/domain.pddl": [1, 1],
                    "agl/folding/p0x.pddl": [1, 1],
                    "agl//p01.pddl": [1, 1],
                    "tempo/folding/p01.pddl": [1, 1]}"#,
            ),
            None,
        );
        assert!(b.is_empty());
        assert!(b.problems().is_empty());
    }

    /// `re.match` anchors only at the start, so a suffix after `.pddl` still
    /// matches in Python. Preserved so a corpus that ever grows one cannot make
    /// the two implementations disagree about which instances have a bound.
    #[test]
    fn a_suffix_after_pddl_still_matches_as_in_python() {
        let b =
            BestKnownBounds::from_sources(Some(r#"{"agl/folding/p01.pddl.bak": [3, 5]}"#), None);
        assert_eq!(b.get(&key("2023-agl", "folding", 1)), Some(5.0));
    }

    /// Insertion order decides a collision, as assignment into a Python dict
    /// does -- not the sorted order a `serde_json::Map` would have imposed.
    #[test]
    fn document_order_decides_a_2023_collision() {
        let b = BestKnownBounds::from_sources(
            Some(r#"{"agl/folding/p01.pddl.z": [0, 1], "agl/folding/p01.pddl": [0, 2]}"#),
            None,
        );
        assert_eq!(b.get(&key("2023-agl", "folding", 1)), Some(2.0));
        let reversed = BestKnownBounds::from_sources(
            Some(r#"{"agl/folding/p01.pddl": [0, 2], "agl/folding/p01.pddl.z": [0, 1]}"#),
            None,
        );
        assert_eq!(reversed.get(&key("2023-agl", "folding", 1)), Some(1.0));
    }

    /// A null bound means nobody has a plan for that instance: no key, and no
    /// complaint. A value of the wrong SHAPE is different -- Python raises
    /// there, so it must at least be recorded here.
    #[test]
    fn null_bounds_are_quiet_but_malformed_ones_are_recorded() {
        let b = BestKnownBounds::from_sources(
            Some(r#"{"agl/folding/p01.pddl": [0, null], "agl/folding/p02.pddl": [1, 2, 3]}"#),
            Some(r#"[["sat/snake/p01.pddl", null], ["sat/snake/p02.pddl"]]"#),
        );
        assert!(b.is_empty());
        assert_eq!(
            b.problems().len(),
            2,
            "one per malformed entry, none for the nulls"
        );
        assert!(b.problems().iter().all(|p| p.contains("expected")));
    }

    /// A file that exists but is not JSON loses only its own half, and says so
    /// -- Python would have stopped the whole regeneration instead.
    #[test]
    fn unparseable_input_degrades_but_complains() {
        let b =
            BestKnownBounds::from_sources(Some("not json"), Some(r#"[["sat/snake/p05.pddl", 7]]"#));
        assert_eq!(b.get(&key("2018", "snake", 5)), Some(7.0));
        assert_eq!(b.problems().len(), 1);
    }
}
