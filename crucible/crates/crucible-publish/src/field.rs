//! The "vs field" column: where a board's coverage RATE would place us among
//! the competition's actual entrants. Ported from `benchmarks/standings.py`
//! (`load_field` :450, the IPC-2023n CSV parser :460-497, `_ord` :500,
//! `_placement` :506, `field_cell` :541).
//!
//! The whole column exists to be *approximately right on the record* instead of
//! precisely stale on a page: before 0.25 the placements lived in
//! `docs/ipc-rankings.md` and were hand-refreshed, which means they were
//! correct on the day a cut shipped and quietly wrong on every day after. A
//! cell is a rough placement under standing caveats -- official budgets are
//! ~30x ours, and coverage is not IPC's quality-weighted score -- never a
//! claimed result. That is exactly why the ways it can be *silently* wrong
//! matter more here than the ways it can be loud:
//!
//! * **The summary rows.** The official IPC-2023 numeric CSVs put per-group
//!   ("SNP"/"LNP") and "Total" summary rows underneath the domain rows,
//!   distinguished only by an EMPTY group tag in column 1. Summing them
//!   alongside the domain rows roughly TRIPLES every entrant's count and the
//!   instance denominator with it, which reads as a plausible table and ranks
//!   us against a field that never existed. The guard is the group tag.
//!
//! * **The official Total wins, even where it disagrees.** `opt.csv`'s
//!   ENHSP BLIND Total reads 48 where that file's own SNP and LNP rows sum to
//!   51. 48 is the number the competition published, so 48 is what we place
//!   against -- and the disagreement is warned about rather than buried, so
//!   nobody rediscovers it as a bug in this parser five cycles from now.
//!
//! * **Lenient cells are counted.** Python swallows per-cell parse failures
//!   (`except (ValueError, IndexError): pass`) because the official files carry
//!   ragged trailing commas. The leniency is right; silence about it is not. A
//!   column read as zeroes throughout is not a weak entrant, it is a mis-rank
//!   presented as data, so every skipped cell is counted and named.
//!
//! * **The first maximum leads.** Python's `max(ents, key=...)` returns the
//!   FIRST maximum; Rust's `max_by_key` returns the LAST. On a two-entrant tie
//!   that is a different planner's name printed in a published table, so the
//!   leader is folded by hand with a strict `>`.
//!
//! * **A rank floor is conditional.** A sparse entrant list makes a strict rank
//!   optimistic, so a cohort that KNOWS more entrants sit ahead than it lists
//!   carries a `rank_floor` and the cell says >= instead of pretending. The
//!   floor holds only while the entrant that justifies it is still ahead of us
//!   (`rank_floor_if_behind` names it): a future cut that passes that mark must
//!   not inherit a stale pessimism any more than an old cut should inherit a
//!   stale optimism.
//!
//! Everything here is a pure function of bytes. Missing inputs degrade: the
//! vendored CSVs are gitignored and simply absent on a clean clone, and a board
//! with no cohort renders an em-dash rather than a guess.

use crate::raw::RawRow;
use crate::referee::Referee;
use std::collections::BTreeMap;
use std::path::Path;

/// Table furniture. These belong beside the rest of the document's non-ASCII
/// glyphs in `fmt`, but that module is being written in the same wave and its
/// shape is not settled; they collapse into `fmt::glyph` at integration.
const EM_DASH: &str = "\u{2014}";
/// The rank-floor marker: ">=", one glyph, U+2265.
const GE: &str = "\u{2265}";
/// The separator between a split cohort's two competitions.
const SPLIT_SEP: &str = " \u{b7} ";

/// The two cohorts parsed live from the vendored official CSVs rather than
/// transcribed into `field-results.json`. Order is Python's tuple order, which
/// is observable only through the warning stream.
const CSV_COHORTS: [(&str, &str); 2] =
    [("2023 numeric", "sat.csv"), ("2023 numeric-opt", "opt.csv")];

/// One planner's line in an official field: a name and a coverage fraction.
///
/// Denominators genuinely differ between entrants in the official record (the
/// 2008 seq-sat field ran three different instance counts), which is why the
/// only currency this module compares in is the RATE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entrant {
    pub name: String,
    pub solved: usize,
    pub of: usize,
}

impl Entrant {
    /// Python: `(e[1] / e[2]) if e[2] else 0.0`. A zero denominator is not an
    /// error, it is an entrant we hold no usable number for, and it must never
    /// win a leader comparison by dividing by nothing.
    fn rate(&self) -> f64 {
        if self.of == 0 {
            0.0
        } else {
            self.solved as f64 / self.of as f64
        }
    }
}

/// One competition's field, as `benchmarks/field-results.json` holds it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cohort {
    pub entrants: Vec<Entrant>,
    /// Total entrants in the official field. `null` in the JSON where the field
    /// size is genuinely unknown; Python's `cohort.get("field_size") or known`
    /// is a FALSY test, so an explicit `0` falls back to the located count
    /// exactly as `null` does, and `placement` reproduces both.
    pub field_size: Option<usize>,
    pub note: Option<String>,
    pub confidence: Option<String>,
    /// A board merging two competitions splits by the rows' own `ipc` field.
    /// Sorted, because Python renders `sorted(cohort["splits"].items())` and
    /// the keys are ASCII, where codepoint order and byte order agree.
    ///
    /// `Option`, not a bare map, because Python branches on `"splits" in
    /// cohort` -- a PRESENCE test, not an emptiness one. A cohort carrying an
    /// empty `splits` object alongside entrants takes the splits path in
    /// Python and renders the em-dash; reading emptiness instead would take the
    /// entrant path and print a placement. Both are plausible tables and only
    /// one is what the Python publishes, which is the whole reason the
    /// distinction is carried in the type rather than in a comment.
    pub splits: Option<BTreeMap<String, Cohort>>,
    /// The pessimism floor. `i64` rather than `usize` because a nonsense
    /// negative in the data should simply never bite, the way Python's `>`
    /// comparison lets it never bite, instead of wrapping into a huge rank.
    pub rank_floor: Option<i64>,
    /// The entrant whose position justifies `rank_floor`. The floor lapses the
    /// moment we pass them.
    pub rank_floor_if_behind: Option<String>,
}

impl Cohort {
    /// Rank `s`/`n` among this cohort's entrants by coverage rate.
    ///
    /// `None` where there is nothing to rank against -- no entrants, or an
    /// empty board -- which the caller renders as an em-dash. Python:
    /// `if not ents or not n: return None`.
    pub fn placement(&self, s: usize, n: usize) -> Option<String> {
        if self.entrants.is_empty() || n == 0 {
            return None;
        }
        let ours = s as f64 / n as f64;

        // No accumulation anywhere in this function: every comparison is one
        // IEEE division against another, so there is no summation order to
        // preserve and no result that depends on it.
        let ahead = self
            .entrants
            .iter()
            .filter(|e| e.of != 0 && e.solved as f64 / e.of as f64 > ours)
            .count();

        let known = self.entrants.len();
        let fs = self.field_size.filter(|f| *f != 0).unwrap_or(known);
        // The field plus us -- the ipc-rankings.md convention, so a cell reads
        // "3rd of 9" against an 8-planner field.
        let total = fs + 1;
        // '~' marks a field with UNLOCATED entrants: the rank is a floor on our
        // own ignorance, and says so by being approximate rather than by being
        // silently generous.
        let mut approx = if fs > known { "~" } else { "" };

        // THE FIRST MAXIMUM. Python's `max(iter, key=...)` keeps the earliest
        // element on a tie and Rust's `max_by_key` keeps the last, so a tie at
        // the top of a field would print a different planner's name in a
        // published table. Folded by hand with a strict `>`.
        let mut lead = &self.entrants[0];
        let mut lead_rate = lead.rate();
        for e in &self.entrants[1..] {
            let r = e.rate();
            if r > lead_rate {
                lead = e;
                lead_rate = r;
            }
        }

        let mut rank = ahead as i64 + 1;
        let mut floor = self.rank_floor.unwrap_or(1);
        if let Some(justif) = &self.rank_floor_if_behind {
            // First match, as Python's `next((e for e in ents if ...), None)`.
            let je = self.entrants.iter().find(|e| &e.name == justif);
            let still_behind_them =
                je.is_some_and(|e| e.of != 0 && e.solved as f64 / e.of as f64 > ours);
            if !still_behind_them {
                floor = 1;
            }
        }
        if floor > rank {
            rank = floor;
            approx = GE;
        }

        Some(format!(
            "{approx}{rank}{suffix} of {total} by rate (leader {name} {solved}/{of})",
            suffix = ordinal_suffix(rank),
            name = lead.name,
            solved = lead.solved,
            of = lead.of,
        ))
    }
}

/// Every cohort we hold, plus everything that looked wrong on the way in.
#[derive(Debug, Clone, Default)]
pub struct FieldBook {
    cohorts: BTreeMap<String, Cohort>,
    warnings: Vec<String>,
}

impl FieldBook {
    /// Read `field-results.json` and the vendored official CSVs under
    /// `benchmarks/`.
    ///
    /// Missing files degrade to an empty book, matching Python's
    /// `if os.path.exists(...)` guards -- the `.ipc-corpus` CSVs in particular
    /// are gitignored and absent on every clean clone, which is normal and not
    /// worth a warning. The rest of the standings still publish; the vs-field
    /// column just says it holds no data.
    pub fn load(benchmarks_dir: &Path) -> Self {
        let mut book = FieldBook::default();
        book.load_json(&benchmarks_dir.join("field-results.json"));
        for (label, fname) in CSV_COHORTS {
            let path = benchmarks_dir
                .join(".ipc-corpus")
                .join("ipc-2023n")
                .join("results")
                .join(fname);
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let rel = format!(".ipc-corpus/ipc-2023n/results/{fname}");
            if let Some(c) = parse_ipc2023n_csv(&decode_utf8_sig(&bytes), &rel, &mut book.warnings)
            {
                // Python's `out[label] = {...}`: a CSV cohort REPLACES a JSON
                // one of the same label. None exists today -- field-results.json
                // says so in its own `_meta` -- and this keeps the live parse
                // authoritative if one ever appears.
                book.cohorts.insert(label.to_string(), c);
            }
        }
        book.check_splits_shape();
        book
    }

    /// The rendered "vs field" cell for one board.
    ///
    /// `solved`/`total` are the board's own coverage; `rows` and `referee` are
    /// needed only by split cohorts, which re-slice the board by each row's
    /// `ipc` field and recount coverage per competition. Always non-empty:
    /// where there is nothing to say it says so with an em-dash, exactly as
    /// Python's `field_cell` does.
    pub fn cell(
        &self,
        label: &str,
        rows: &[RawRow],
        referee: &Referee,
        solved: usize,
        total: usize,
    ) -> String {
        let Some(cohort) = self.cohorts.get(label) else {
            return EM_DASH.to_string();
        };
        // PRESENCE, not emptiness: Python's `if "splits" in cohort`. A cohort
        // that carries the key at all takes this path, even where the object is
        // empty and the cell therefore renders as the em-dash.
        if let Some(splits) = &cohort.splits {
            let mut parts: Vec<String> = Vec::new();
            for (ipc, sub) in splits {
                let rs: Vec<&RawRow> = rows
                    .iter()
                    .filter(|r| r.ipc.as_deref() == Some(ipc.as_str()))
                    .collect();
                let ss = rs.iter().filter(|r| referee.is_solved(r)).count();
                if let Some(p) = sub.placement(ss, rs.len()) {
                    // Python's `ipc[-4:]`: the competition year, sliced off the
                    // corpus directory name. Sliced by CHARACTER, and short of
                    // four it is the whole key.
                    parts.push(format!("{}: {p}", tail4(ipc)));
                }
            }
            return if parts.is_empty() {
                EM_DASH.to_string()
            } else {
                parts.join(SPLIT_SEP)
            };
        }
        cohort
            .placement(solved, total)
            .unwrap_or_else(|| EM_DASH.to_string())
    }

    /// Everything that looked wrong while loading. Never fatal, always named:
    /// the failure mode this column has is a plausible number, not a crash.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn cohort(&self, label: &str) -> Option<&Cohort> {
        self.cohorts.get(label)
    }

    /// The half of the splits check that needs a board to check against: a key
    /// whose shape is fine but which matches no row's `ipc`, so its side of the
    /// cell silently renders as nothing.
    ///
    /// Separate from `warnings` because `cell` is `&self` and pure; the renderer
    /// folds these into its own warning stream as it walks the boards.
    pub fn unmatched_splits(&self, label: &str, rows: &[RawRow]) -> Vec<String> {
        let Some(c) = self.cohorts.get(label) else {
            return Vec::new();
        };
        c.splits
            .iter()
            .flat_map(|m| m.keys())
            .filter(|k| !rows.iter().any(|r| r.ipc.as_deref() == Some(k.as_str())))
            .map(|k| {
                format!(
                    "benchmarks/field-results.json: cohort \"{label}\" splits key \"{k}\" \
                     matched no row on that board -- that competition's half of the cell \
                     renders as nothing"
                )
            })
            .collect()
    }

    fn load_json(&mut self, path: &Path) {
        // Missing (or unreadable) is an empty book, per Python's
        // `if os.path.exists(p)`.
        let Ok(src) = std::fs::read_to_string(path) else {
            return;
        };
        let doc: serde_json::Value = match serde_json::from_str(&src) {
            Ok(d) => d,
            Err(e) => {
                // Python raises here and takes the whole regeneration with it.
                // Publication costs one column instead, loudly.
                self.warnings.push(format!(
                    "benchmarks/field-results.json: {e} -- the vs-field column \
                     renders as {EM_DASH} for every board"
                ));
                return;
            }
        };
        let Some(obj) = doc.get("cohorts").and_then(|x| x.as_object()) else {
            return;
        };
        for (label, v) in obj {
            let c = cohort_from_json(label, v, &mut self.warnings);
            self.cohorts.insert(label.clone(), c);
        }
    }

    /// A splits key is an ipc corpus directory name (`ipc-2008`) and is joined
    /// against each row's `ipc` field. A typo -- a stray space, a year with a
    /// letter in it, a bare "2008" -- matches nothing and renders an empty cell
    /// that looks exactly like "we hold no field data", which is a different
    /// and much more forgivable claim.
    fn check_splits_shape(&mut self) {
        let mut found = Vec::new();
        for (label, c) in &self.cohorts {
            for key in c.splits.iter().flat_map(|m| m.keys()) {
                if !is_ipc_dir_name(key) {
                    found.push(format!(
                        "benchmarks/field-results.json: cohort \"{label}\" splits key \
                         \"{key}\" is not an ipc corpus directory name (\"ipc-2008\"), \
                         so it can match no row's `ipc` field and renders as nothing"
                    ));
                }
            }
        }
        self.warnings.extend(found);
    }
}

fn is_ipc_dir_name(k: &str) -> bool {
    k.strip_prefix("ipc-")
        .is_some_and(|y| y.len() == 4 && y.bytes().all(|b| b.is_ascii_digit()))
}

/// Python's `s[-4:]`, which slices by codepoint and returns the whole string
/// when it is shorter than four.
fn tail4(s: &str) -> &str {
    match s.char_indices().rev().nth(3) {
        Some((i, _)) => &s[i..],
        None => s,
    }
}

/// Python's `_ord`. `rem_euclid` rather than `%` so a negative rank from a
/// nonsense `rank_floor` lands on the same suffix Python would give it, instead
/// of on Rust's sign-of-the-dividend remainder.
fn ordinal_suffix(n: i64) -> &'static str {
    if (10..=20).contains(&n.rem_euclid(100)) {
        return "th";
    }
    match n.rem_euclid(10) {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

fn cohort_from_json(label: &str, v: &serde_json::Value, warnings: &mut Vec<String>) -> Cohort {
    let mut c = Cohort::default();
    if let Some(a) = v.get("entrants").and_then(|x| x.as_array()) {
        for (i, e) in a.iter().enumerate() {
            match entrant_from_json(e) {
                Some(x) => c.entrants.push(x),
                // Python unpacks `for _, es, eo in ents` and raises on anything
                // that is not a triple. Dropping the entrant keeps the rest of
                // the table publishable, but a dropped entrant is one fewer
                // planner counted ahead of us -- i.e. a rank that flatters.
                None => warnings.push(format!(
                    "benchmarks/field-results.json: cohort \"{label}\" entrant #{i} is not \
                     [name, solved, of] and was dropped -- every cell this cohort renders \
                     is ranked against a field one entrant short"
                )),
            }
        }
    }
    c.field_size = v
        .get("field_size")
        .and_then(|x| x.as_u64())
        .map(|x| x as usize);
    c.note = v.get("note").and_then(|x| x.as_str()).map(String::from);
    c.confidence = v
        .get("confidence")
        .and_then(|x| x.as_str())
        .map(String::from);
    c.rank_floor = v.get("rank_floor").and_then(|x| x.as_i64());
    c.rank_floor_if_behind = v
        .get("rank_floor_if_behind")
        .and_then(|x| x.as_str())
        .map(String::from);
    // The key's PRESENCE is what Python branches on, so it is what is
    // recorded. A `splits` that is not an object is where Python raises; the
    // cell degrades to the em-dash instead, which reads as "no field data
    // held" and so has to say out loud that it is a malformed cohort.
    if let Some(sv) = v.get("splits") {
        let mut m = BTreeMap::new();
        match sv.as_object() {
            Some(o) => {
                for (k, x) in o {
                    let sub = cohort_from_json(&format!("{label}/{k}"), x, warnings);
                    m.insert(k.clone(), sub);
                }
            }
            None => warnings.push(format!(
                "benchmarks/field-results.json: cohort \"{label}\" has a `splits` that is \
                 not an object -- every cell it renders is the {EM_DASH} that otherwise \
                 means no field data is held"
            )),
        }
        c.splits = Some(m);
    }
    c
}

fn entrant_from_json(v: &serde_json::Value) -> Option<Entrant> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    let name = a[0].as_str()?.to_string();
    let solved = usize::try_from(a[1].as_i64()?).ok()?;
    let of = usize::try_from(a[2].as_i64()?).ok()?;
    Some(Entrant { name, solved, of })
}

/// Python opens the official CSVs with `encoding="utf-8-sig"`: one leading BOM
/// is consumed, and it must be, or the first header cell is `\u{feff}Optimal`
/// and every column shifts in the reader's mind.
///
/// Decoding is lossy where Python would raise. A mangled entrant NAME is
/// visible in the published cell; an aborted regeneration is not visible
/// anywhere, and these files are third-party vendored data.
fn decode_utf8_sig(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    match s.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => s.into_owned(),
    }
}

/// Python's `int(s)` on a CSV cell: surrounding whitespace stripped, an
/// optional sign, then digits. `None` where Python would raise `ValueError`,
/// which the caller swallows and counts.
///
/// Two Python acceptances are deliberately not reproduced -- underscore
/// grouping (`"1_0"` is 10 to `int()`) and non-ASCII decimal digits. Neither
/// has ever appeared in an official results CSV, and treating them as
/// unparseable is the conservative direction: it warns rather than inventing a
/// number.
fn py_int(s: &str) -> Option<i64> {
    let t = s.trim();
    let (neg, digits) = match t.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: i64 = digits.parse().ok()?;
    Some(if neg { -v } else { v })
}

/// Entrant column `i` of a CSV record, i.e. Python's `int(r[2 + i])` inside its
/// `try`. Both failure modes -- a short row (`IndexError`) and an unparseable
/// cell (`ValueError`) -- collapse to `None`, exactly as the `except` clause
/// collapses them.
fn cell_int(rec: &[String], i: usize) -> Option<i64> {
    rec.get(2 + i).and_then(|s| py_int(s))
}

/// Parse one official IPC-2023 numeric results CSV into a cohort.
///
/// The file's shape: a header naming the entrants from column 2 on, one row per
/// domain tagged SNP or LNP in column 0, then summary rows whose column-0 tag is
/// EMPTY -- a "Total" row and one per group. Every trap in this function is in
/// those last three rows.
fn parse_ipc2023n_csv(text: &str, path: &str, warnings: &mut Vec<String>) -> Option<Cohort> {
    let rows = read_csv(text);
    let Some(header) = rows.first() else {
        // Python indexes `rows[0]` and raises IndexError on an empty file.
        warnings.push(format!(
            "{path}: no rows -- the file is empty or truncated, and its cohort is \
             not loaded"
        ));
        return None;
    };

    let head: Vec<&str> = header.iter().skip(2).map(|c| c.trim()).collect();
    // Python's `[n.strip() for n in rows[0][2:] if n.strip()]` compacts EVERY
    // empty header cell, not just the trailing one the files actually carry.
    // A gap in the middle would leave the name list one short of the data
    // columns and shift every entrant after it onto its neighbour's numbers.
    if let Some(last) = head.iter().rposition(|c| !c.is_empty()) {
        let gaps = head[..last].iter().filter(|c| c.is_empty()).count();
        if gaps > 0 {
            warnings.push(format!(
                "{path}: {gaps} unnamed entrant column(s) before the last named one -- \
                 the name list compacts but the DATA columns do not, so entrants after \
                 the gap are read from a neighbour's column"
            ));
        }
    }
    let names: Vec<String> = head
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| (*c).to_string())
        .collect();

    let mut tot = vec![0i64; names.len()];
    let mut group_tot = vec![0i64; names.len()];
    let mut group_rows = 0usize;
    let mut doms = 0usize;
    let mut skipped = 0usize;
    let mut total_row: Option<&Vec<String>> = None;

    for r in rows.iter().skip(1) {
        if r.len() < 3 {
            continue;
        }
        // THE GUARD. Domain rows carry a group tag (SNP/LNP) in column 0; the
        // trailing summary rows leave it empty. Summing the summary rows in
        // alongside the domain rows triples every count AND the domain
        // denominator, producing a table that looks entirely reasonable.
        if r[0].trim().is_empty() {
            if r[1].trim() == "Total" {
                // Last "Total" row wins, as a repeated Python assignment does.
                total_row = Some(r);
            } else {
                // Not summed into anything published -- kept only to check the
                // official file against itself below.
                group_rows += 1;
                for (i, g) in group_tot.iter_mut().enumerate() {
                    if let Some(v) = cell_int(r, i) {
                        *g += v;
                    }
                }
            }
            continue;
        }
        doms += 1;
        for (i, t) in tot.iter_mut().enumerate() {
            match cell_int(r, i) {
                Some(v) => *t += v,
                // The leniency exists for the files' ragged trailing commas and
                // is kept; the silence is not.
                None => skipped += 1,
            }
        }
    }

    if skipped > 0 {
        warnings.push(format!(
            "{path}: {skipped} entrant cell(s) unreadable and skipped -- a column read \
             as zeroes throughout is a mis-rank presented as data, not a weak entrant"
        ));
    }

    let official: Vec<Option<i64>> = match total_row {
        Some(tr) => (0..names.len()).map(|i| cell_int(tr, i)).collect(),
        None => vec![None; names.len()],
    };

    // Both disagreements are recorded BEFORE the override, because after it
    // there is nothing left to see.
    let mut vs_rows: Vec<String> = Vec::new();
    let mut vs_groups: Vec<String> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let Some(off) = official[i] else { continue };
        if off != tot[i] {
            vs_rows.push(format!("{name} (domain rows {}, Total {off})", tot[i]));
        }
        if group_rows > 0 && off != group_tot[i] {
            vs_groups.push(format!("{name} (Total {off}, group rows {})", group_tot[i]));
        }
    }
    if !vs_rows.is_empty() {
        warnings.push(format!(
            "{path}: the official Total row disagrees with this file's own domain rows \
             for {} entrant(s); the OFFICIAL Total is kept verbatim -- {}",
            vs_rows.len(),
            vs_rows.join("; ")
        ));
    }
    if !vs_groups.is_empty() {
        warnings.push(format!(
            "{path}: the official summary rows disagree among themselves for {} \
             entrant(s); the OFFICIAL Total is kept verbatim -- {}",
            vs_groups.len(),
            vs_groups.join("; ")
        ));
    }

    // The official Total row OVERRIDES our sum, per column and leniently: a
    // Total cell we cannot read leaves that column's computed sum standing.
    for (i, t) in tot.iter_mut().enumerate() {
        if let Some(off) = official[i] {
            *t = off;
        }
    }

    // `of = doms * 20`: the official numeric tracks run 20 instances per domain
    // and the CSV carries no denominator of its own. Counting the summary rows
    // as domains is the other half of the tripling.
    let of = doms.saturating_mul(20);
    let entrants = names
        .iter()
        .zip(tot.iter())
        .map(|(n, t)| Entrant {
            name: n.clone(),
            // A negative count cannot occur in an official file; clamping keeps
            // a corrupt cell from wrapping into an enormous rank.
            solved: usize::try_from(*t).unwrap_or(0),
            of,
        })
        .collect();

    Some(Cohort {
        entrants,
        field_size: Some(names.len()),
        note: Some("official per-domain CSV (ipc-2023n/results), parsed live".to_string()),
        confidence: Some("high".to_string()),
        // Python's CSV cohort dict carries no `splits` key at all.
        splits: None,
        rank_floor: None,
        rank_floor_if_behind: None,
    })
}

/// A minimal RFC 4180 reader, standing in for Python's `csv.reader`.
///
/// There is no `csv` crate here by design (the crate stays dependency-thin), and
/// these files need very little: comma-separated fields, quotes only where a
/// field begins with one, `""` for a literal quote inside. The newline handling
/// is Python's, not the CSV spec's -- the file is opened in text mode, so
/// universal newlines have already turned `\r\n` and a lone `\r` into `\n`
/// before the reader sees a byte. A wholly blank line is `[]` to `csv.reader`,
/// not `[""]`, and a trailing newline does not produce a final empty record.
fn read_csv(src: &str) -> Vec<Vec<String>> {
    let src = src.replace("\r\n", "\n").replace('\r', "\n");
    let mut recs: Vec<Vec<String>> = Vec::new();
    let mut rec: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut field_start = true;
    let mut rec_chars = 0usize;
    let mut it = src.chars().peekable();
    while let Some(c) = it.next() {
        if in_quotes {
            rec_chars += 1;
            if c == '"' {
                if it.peek() == Some(&'"') {
                    it.next();
                    rec_chars += 1;
                    cur.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                cur.push(c);
            }
            continue;
        }
        if c == '\n' {
            if rec_chars == 0 {
                recs.push(Vec::new());
            } else {
                rec.push(std::mem::take(&mut cur));
                recs.push(std::mem::take(&mut rec));
            }
            field_start = true;
            rec_chars = 0;
            continue;
        }
        rec_chars += 1;
        match c {
            '"' if field_start => {
                in_quotes = true;
                field_start = false;
            }
            ',' => {
                rec.push(std::mem::take(&mut cur));
                field_start = true;
            }
            _ => {
                cur.push(c);
                field_start = false;
            }
        }
    }
    // A final record with no terminating newline.
    if rec_chars > 0 {
        rec.push(cur);
        recs.push(rec);
    }
    recs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(name: &str, solved: usize, of: usize) -> Entrant {
        Entrant {
            name: name.to_string(),
            solved,
            of,
        }
    }

    fn book(label: &str, c: Cohort) -> FieldBook {
        let mut b = FieldBook::default();
        b.cohorts.insert(label.to_string(), c);
        b
    }

    /// Built through serde so a column another agent adds to `RawRow` in this
    /// wave cannot break these tests.
    fn row(ipc: &str, solved: bool) -> RawRow {
        serde_json::from_str(&format!(
            r#"{{"ipc":"{ipc}","variant":"x","instance":1,"solved":{solved},"val":true}}"#
        ))
        .expect("fixture row parses")
    }

    // ---------------------------------------------------------------- cells
    // The four tests below reproduce cells from the committed STANDINGS.md
    // verbatim. Those strings ARE the specification; anything that changes one
    // of them is a published number moving.

    /// STANDINGS.md, `2014 seq-sat` 149/280: the rank floor bites, so the cell
    /// reads ">=8th" and not the strict-rank "3rd" a three-entrant list would
    /// otherwise flatter us into.
    #[test]
    fn real_cell_2014_seq_sat_rank_floor_bites() {
        let c = Cohort {
            entrants: vec![
                ent("IBaCoP2", 198, 280),
                ent("located band floor", 163, 280),
                ent("quality-tier floor", 125, 280),
            ],
            field_size: Some(20),
            rank_floor: Some(8),
            rank_floor_if_behind: Some("located band floor".to_string()),
            ..Cohort::default()
        };
        let b = book("2014 seq-sat", c);
        assert_eq!(
            b.cell("2014 seq-sat", &[], &Referee::default(), 149, 280),
            "\u{2265}8th of 21 by rate (leader IBaCoP2 198/280)"
        );
    }

    /// STANDINGS.md, `2018 seq-sat` 82/240. A 24-entrant field whose winners
    /// hold no located raw counts: the floor is justified by the field MEDIAN,
    /// and the leader printed is the field MEAN, which is the higher rate and
    /// comes first in the list.
    #[test]
    fn real_cell_2018_seq_sat_floor_justified_by_the_median() {
        let c = Cohort {
            entrants: vec![
                ent("field mean", 94, 240),
                ent("field median", 91, 240),
                ent("fs-sim", 70, 240),
                ent("fs-blind", 60, 240),
                ent("freelunch-madagascar", 23, 240),
                ent("alien", 15, 240),
                ent("Symple-1/2", 14, 240),
            ],
            field_size: Some(24),
            rank_floor: Some(13),
            rank_floor_if_behind: Some("field median".to_string()),
            ..Cohort::default()
        };
        let b = book("2018 seq-sat", c);
        assert_eq!(
            b.cell("2018 seq-sat", &[], &Referee::default(), 82, 240),
            "\u{2265}13th of 25 by rate (leader field mean 94/240)"
        );
    }

    /// STANDINGS.md, `net-benefit` 248/270: a fully located field, so no '~',
    /// and we lead it -- 1st of 4 while the leader named is the best ENTRANT.
    /// The leader is the field's leader, never us.
    #[test]
    fn real_cell_net_benefit_fully_located_field() {
        let c = Cohort {
            entrants: vec![
                ent("Gamer", 81, 210),
                ent("Mips-XXL", 59, 210),
                ent("HSP*P", 51, 210),
            ],
            field_size: Some(3),
            ..Cohort::default()
        };
        let b = book("net-benefit", c);
        assert_eq!(
            b.cell("net-benefit", &[], &Referee::default(), 248, 270),
            "1st of 4 by rate (leader Gamer 81/210)"
        );
    }

    /// STANDINGS.md, `seq-sat` 504/580: one sweep covering two competitions,
    /// re-sliced by each row's own `ipc` field and rendered as two placements
    /// joined by the middle dot. The board's own 504/580 is NOT used by either
    /// half -- each competition recounts its own rows.
    #[test]
    fn real_cell_seq_sat_splits_render_both_competitions() {
        let mut splits = BTreeMap::new();
        splits.insert(
            "ipc-2008".to_string(),
            Cohort {
                entrants: vec![
                    ent("LAMA", 281, 300),
                    ent("FF(h_sa)", 225, 270),
                    ent("Plan-A", 37, 180),
                ],
                field_size: Some(10),
                ..Cohort::default()
            },
        );
        splits.insert(
            "ipc-2011".to_string(),
            Cohort {
                entrants: vec![
                    ent("LAMA-2011", 250, 280),
                    ent("FDSS-2", 233, 280),
                    ent("PROBE", 233, 280),
                    ent("FDSS-1", 232, 280),
                    ent("FD-AUTOTUNE-1", 223, 280),
                    ent("ROAMER", 213, 280),
                    ent("ACOPLAN", 20, 280),
                ],
                field_size: Some(27),
                ..Cohort::default()
            },
        );
        let b = book(
            "seq-sat",
            Cohort {
                splits: Some(splits),
                ..Cohort::default()
            },
        );
        // 2008: 300 rows, 285 solved (95%) beats every located entrant.
        // 2011: 280 rows, 219 solved (78%) sits behind five of seven.
        let mut rows: Vec<RawRow> = Vec::new();
        for i in 0..300 {
            rows.push(row("ipc-2008", i < 285));
        }
        for i in 0..280 {
            rows.push(row("ipc-2011", i < 219));
        }
        assert_eq!(
            b.cell("seq-sat", &rows, &Referee::default(), 504, 580),
            "2008: ~1st of 11 by rate (leader LAMA 281/300) \u{b7} \
             2011: ~6th of 28 by rate (leader LAMA-2011 250/280)"
        );
    }

    /// STANDINGS.md, `2023 numeric` 251/400 against the eight columns of the
    /// official sat.csv, with the Totals that file publishes (the four ENHSP /
    /// NLM-CutPlan figures are quoted in docs/ipc-rankings.md). A cohort born
    /// from a CSV knows its whole field, so there is no '~'.
    #[test]
    fn real_cell_2023_numeric_from_the_official_totals() {
        let c = Cohort {
            entrants: vec![
                ent("ENHSP hmrp", 191, 400),
                ent("ENHSP hmrp+ha", 267, 400),
                ent("ENHSP hmrp+ha+ht", 264, 400),
                ent("NLM-CutPlan Sat", 136, 400),
                ent("NLM-CutPlan OC Sat", 88, 400),
                ent("NLM-CutPlan Sat2", 88, 400),
                ent("OMTPlan (Sequential)", 63, 400),
                ent("OMTPlan (Parallel)", 100, 400),
            ],
            field_size: Some(8),
            ..Cohort::default()
        };
        let b = book("2023 numeric", c);
        assert_eq!(
            b.cell("2023 numeric", &[], &Referee::default(), 251, 400),
            "3rd of 9 by rate (leader ENHSP hmrp+ha 267/400)"
        );
    }

    // ------------------------------------------------------------ CSV traps

    /// The shape of the official files, compressed: a BOM, a trailing comma on
    /// every line, four domain rows under two group tags, then the three
    /// summary rows whose group tag is empty. ALPHA reproduces the real
    /// opt.csv incident -- Total 48 where the group rows sum to 51. BETA's
    /// Total (11) disagrees with its own domain rows (10). GAMMA carries an
    /// unreadable cell.
    const CSV: &str = "\u{feff}Optimal,,ALPHA,BETA,GAMMA,\n\
         SNP,d1,10,1,x,\n\
         SNP,d2,20,2,3,\n\
         LNP,d3,9,3,4,\n\
         LNP,d4,9,4,5,\n\
         ,Total,48,11,12,\n\
         ,SNP,30,3,3,\n\
         ,LNP,21,8,9,\n";

    /// THE SUMMARY-ROW TRAP. Four domain rows means `of = 80`; counting the
    /// three summary rows as domains would make it 140 and roughly triple every
    /// entrant's count. The guard is the empty group tag in column 0.
    #[test]
    fn csv_summary_rows_are_not_domains() {
        let mut w = Vec::new();
        let c = parse_ipc2023n_csv(CSV, "t.csv", &mut w).expect("cohort");
        assert_eq!(c.entrants.len(), 3);
        assert_eq!(c.field_size, Some(3));
        assert_eq!(c.entrants[0].of, 80, "4 domain rows * 20, not 7 * 20");
        assert_eq!(c.entrants[0].solved, 48);
    }

    /// THE OFFICIAL TOTAL WINS. BETA's domain rows sum to 10 and its Total
    /// reads 11; 11 is the published number and is what we place against. The
    /// disagreement is warned about, not buried.
    #[test]
    fn csv_official_total_overrides_the_computed_sum_and_says_so() {
        let mut w = Vec::new();
        let c = parse_ipc2023n_csv(CSV, "t.csv", &mut w).expect("cohort");
        assert_eq!(c.entrants[1].name, "BETA");
        assert_eq!(c.entrants[1].solved, 11, "the official Total, not our 10");
        assert!(
            w.iter()
                .any(|x| x.contains("domain rows 10, Total 11") && x.contains("kept verbatim")),
            "{w:?}"
        );
    }

    /// The real opt.csv discrepancy, in miniature: ENHSP BLIND's Total reads 48
    /// where that file's own SNP and LNP rows sum to 51. The official Total is
    /// preserved AND the disagreement is put on the record, so nobody
    /// rediscovers it later as a bug in this parser.
    #[test]
    fn csv_official_summary_rows_disagreeing_among_themselves_is_recorded() {
        let mut w = Vec::new();
        let c = parse_ipc2023n_csv(CSV, "t.csv", &mut w).expect("cohort");
        assert_eq!(c.entrants[0].solved, 48);
        assert!(
            w.iter()
                .any(|x| x.contains("ALPHA (Total 48, group rows 51)")),
            "{w:?}"
        );
    }

    /// The leniency is Python's -- the official files carry ragged trailing
    /// commas -- but a silently-zero column is a mis-rank presented as data, so
    /// the skipped cells are counted and named once per file.
    #[test]
    fn csv_unreadable_cells_are_skipped_leniently_but_counted() {
        let mut w = Vec::new();
        let c = parse_ipc2023n_csv(CSV, "t.csv", &mut w).expect("cohort");
        // GAMMA: "x" skipped, 3 + 4 + 5 = 12 summed, and the Total agrees.
        assert_eq!(c.entrants[2].solved, 12);
        assert!(
            w.iter().any(|x| x.contains("1 entrant cell(s) unreadable")),
            "{w:?}"
        );
    }

    /// utf-8-sig. Without the strip the first header cell is "\u{feff}Optimal",
    /// which is only cosmetic here -- but the same BOM on a file whose first
    /// two columns were entrants would rename one of them.
    #[test]
    fn csv_leading_bom_is_consumed() {
        assert_eq!(decode_utf8_sig(b"\xef\xbb\xbfOptimal,,A,"), "Optimal,,A,");
        let recs = read_csv(&decode_utf8_sig(CSV.as_bytes()));
        assert_eq!(recs[0][0], "Optimal");
    }

    /// A gap in the header names compacts the name list but not the data
    /// columns, so every entrant after the gap would be scored from a
    /// neighbour's numbers. Python does this silently.
    #[test]
    fn csv_unnamed_interior_column_warns() {
        let src = "H,,A,,B,\nSNP,d1,1,2,3,\n";
        let mut w = Vec::new();
        let c = parse_ipc2023n_csv(src, "t.csv", &mut w).expect("cohort");
        assert_eq!(c.entrants.len(), 2);
        assert!(
            w.iter().any(|x| x.contains("unnamed entrant column")),
            "{w:?}"
        );
    }

    /// csv.reader gives `[]` for a blank line and no final record for a
    /// trailing newline; a quoted field keeps its comma.
    #[test]
    fn csv_reader_matches_python_on_blank_lines_and_quotes() {
        let recs = read_csv("a,b\n\n\"x,y\",z\n");
        assert_eq!(recs.len(), 3);
        assert_eq!(recs[0], vec!["a".to_string(), "b".to_string()]);
        assert!(recs[1].is_empty(), "a blank line is [], not [\"\"]");
        assert_eq!(recs[2], vec!["x,y".to_string(), "z".to_string()]);
    }

    // ------------------------------------------------------------ placement

    /// THE FIRST MAXIMUM. Python's `max(ents, key=...)` keeps the earliest
    /// element on a tie; `max_by_key` would print "Bee" here, in a published
    /// table, forever.
    #[test]
    fn leader_on_an_exact_tie_is_the_first_maximum() {
        let c = Cohort {
            entrants: vec![ent("Aye", 10, 20), ent("Bee", 10, 20), ent("Cee", 5, 20)],
            field_size: Some(3),
            ..Cohort::default()
        };
        assert_eq!(
            c.placement(6, 20).unwrap(),
            "3rd of 4 by rate (leader Aye 10/20)"
        );
    }

    /// The floor is CONDITIONAL. Same cohort as the real 2014 seq-sat cell, but
    /// with a coverage that has passed the entrant justifying the floor: the
    /// pessimism lapses rather than being inherited by a better cut.
    #[test]
    fn rank_floor_lapses_once_its_justifying_entrant_is_passed() {
        let c = Cohort {
            entrants: vec![
                ent("IBaCoP2", 198, 280),
                ent("located band floor", 163, 280),
                ent("quality-tier floor", 125, 280),
            ],
            field_size: Some(20),
            rank_floor: Some(8),
            rank_floor_if_behind: Some("located band floor".to_string()),
            ..Cohort::default()
        };
        // One instance clear of the band floor: the pessimism lapses.
        assert_eq!(
            c.placement(164, 280).unwrap(),
            "~2nd of 21 by rate (leader IBaCoP2 198/280)"
        );
        // LEVEL with it also lapses -- the Python condition is a strict `>`,
        // so a cut that has merely matched the mark is no longer "behind" it.
        assert_eq!(
            c.placement(163, 280).unwrap(),
            "~2nd of 21 by rate (leader IBaCoP2 198/280)"
        );
        // One instance short, and the floor bites as it does in STANDINGS.md.
        assert_eq!(
            c.placement(162, 280).unwrap(),
            "\u{2265}8th of 21 by rate (leader IBaCoP2 198/280)"
        );
    }

    /// A floor with no justifying entrant named holds unconditionally -- Python
    /// only clears it when `rank_floor_if_behind` is present and its entrant is
    /// not ahead.
    #[test]
    fn unjustified_rank_floor_holds() {
        let c = Cohort {
            entrants: vec![ent("Solo", 1, 280)],
            field_size: Some(20),
            rank_floor: Some(9),
            ..Cohort::default()
        };
        assert_eq!(
            c.placement(279, 280).unwrap(),
            "\u{2265}9th of 21 by rate (leader Solo 1/280)"
        );
    }

    /// `field_size: null` (the qualitative-preferences cohort) falls back to
    /// the located count, so the field is not marked approximate -- Python's
    /// `or` makes null and 0 behave identically here.
    #[test]
    fn null_field_size_falls_back_to_the_located_count() {
        let c = Cohort {
            entrants: vec![ent("SGPlan5", 100, 100), ent("HPlan-P", 70, 100)],
            field_size: None,
            ..Cohort::default()
        };
        assert_eq!(
            c.placement(50, 100).unwrap(),
            "3rd of 3 by rate (leader SGPlan5 100/100)"
        );
        let zero = Cohort {
            field_size: Some(0),
            ..c.clone()
        };
        assert_eq!(zero.placement(50, 100), c.placement(50, 100));
    }

    /// An entrant we hold no denominator for can neither rank ahead of us nor
    /// lead the field by dividing by nothing.
    #[test]
    fn zero_denominator_entrant_neither_ranks_nor_leads() {
        let c = Cohort {
            entrants: vec![ent("Unknown", 999, 0), ent("Real", 1, 100)],
            field_size: Some(2),
            ..Cohort::default()
        };
        assert_eq!(
            c.placement(0, 100).unwrap(),
            "2nd of 3 by rate (leader Real 1/100)"
        );
    }

    /// Teens take "th" whatever their last digit says -- 11th, 12th, 13th, and
    /// 21st again after them.
    #[test]
    fn ordinal_suffixes_follow_python() {
        let got: Vec<&str> = [1, 2, 3, 4, 11, 12, 13, 14, 20, 21, 22, 23, 101, 111]
            .into_iter()
            .map(ordinal_suffix)
            .collect();
        assert_eq!(
            got,
            ["st", "nd", "rd", "th", "th", "th", "th", "th", "th", "st", "nd", "rd", "st", "th"]
        );
    }

    // ------------------------------------------------------- absent / typos

    /// No cohort, an empty cohort and an empty board all render the em-dash:
    /// "we hold no per-entrant field data", never a rank invented from nothing.
    #[test]
    fn no_data_renders_the_em_dash() {
        let empty = FieldBook::default();
        assert_eq!(
            empty.cell("anything", &[], &Referee::default(), 1, 2),
            "\u{2014}"
        );
        let b = book("bare", Cohort::default());
        assert_eq!(b.cell("bare", &[], &Referee::default(), 1, 2), "\u{2014}");
        let c = book(
            "solo",
            Cohort {
                entrants: vec![ent("SGPlan5", 1, 2)],
                ..Cohort::default()
            },
        );
        // n == 0: nothing measured, nothing to place.
        assert_eq!(c.cell("solo", &[], &Referee::default(), 0, 0), "\u{2014}");
    }

    /// A missing benchmarks directory is an empty book, not a failure -- the
    /// vendored CSVs are gitignored and absent on every clean clone, and that
    /// silence is deliberate.
    #[test]
    fn missing_inputs_load_to_an_empty_book_without_warnings() {
        let b = FieldBook::load(Path::new("/nonexistent/benchmarks"));
        assert!(b.warnings().is_empty(), "{:?}", b.warnings());
        assert_eq!(
            b.cell("2023 numeric", &[], &Referee::default(), 1, 2),
            "\u{2014}"
        );
    }

    /// A splits key that is not an ipc corpus directory name can never join to
    /// a row and renders an empty cell that reads exactly like "no field data
    /// held" -- a different and far more forgivable claim.
    #[test]
    fn splits_key_that_is_not_a_corpus_dir_name_warns() {
        assert!(is_ipc_dir_name("ipc-2008"));
        assert!(!is_ipc_dir_name("ipc-08"));
        assert!(!is_ipc_dir_name("2008"));
        assert!(!is_ipc_dir_name("ipc-2008 "));

        let mut splits = BTreeMap::new();
        splits.insert("ipc-201l".to_string(), Cohort::default());
        let mut b = book(
            "seq-sat",
            Cohort {
                splits: Some(splits),
                ..Cohort::default()
            },
        );
        b.check_splits_shape();
        assert!(
            b.warnings()[0].contains("ipc-201l") && b.warnings()[0].contains("seq-sat"),
            "{:?}",
            b.warnings()
        );
    }

    /// A well-formed key that matches no row on the board it renders: the typo
    /// class the shape check cannot see, reported against the rows.
    #[test]
    fn unmatched_splits_names_the_key_that_matched_nothing() {
        let mut splits = BTreeMap::new();
        splits.insert("ipc-2008".to_string(), Cohort::default());
        splits.insert("ipc-2099".to_string(), Cohort::default());
        let b = book(
            "seq-sat",
            Cohort {
                splits: Some(splits),
                ..Cohort::default()
            },
        );
        let rows = vec![row("ipc-2008", true)];
        let w = b.unmatched_splits("seq-sat", &rows);
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("ipc-2099"), "{w:?}");
    }

    /// The competition year is the last four CHARACTERS of the key, and a key
    /// shorter than four is itself.
    #[test]
    fn split_prefix_is_the_last_four_chars() {
        assert_eq!(tail4("ipc-2008"), "2008");
        assert_eq!(tail4("ipc"), "ipc");
        assert_eq!(tail4(""), "");
    }

    /// The entrant triple is a triple. Python raises unpacking anything else;
    /// dropping it keeps the table publishable but flatters the rank, so it is
    /// named.
    #[test]
    fn malformed_entrant_is_dropped_with_a_warning() {
        let v: serde_json::Value = serde_json::json!({
            "entrants": [["Good", 1, 2], ["Bad", 1], ["Worse", 1, 2, 3], "nope"],
            "field_size": 4
        });
        let mut w = Vec::new();
        let c = cohort_from_json("t", &v, &mut w);
        assert_eq!(c.entrants, vec![ent("Good", 1, 2)]);
        assert_eq!(w.len(), 3, "{w:?}");
    }

    /// PRESENCE, not emptiness. Python's `field_cell` branches on
    /// `"splits" in cohort`, so a cohort carrying the key -- even as an empty
    /// object -- renders the em-dash and never reaches its own entrants. A port
    /// that asked whether the map was EMPTY would print a placement here, which
    /// is a published number invented from a cohort that declared itself split.
    #[test]
    fn an_empty_splits_object_takes_the_splits_path_and_renders_the_em_dash() {
        let v: serde_json::Value = serde_json::json!({
            "splits": {},
            "entrants": [["SGPlan5", 5, 10]],
            "field_size": 1
        });
        let mut w = Vec::new();
        let c = cohort_from_json("t", &v, &mut w);
        assert_eq!(c.splits, Some(BTreeMap::new()));
        let b = book("L", c);
        assert_eq!(b.cell("L", &[], &Referee::default(), 9, 10), "\u{2014}");

        // ... while a cohort with NO `splits` key at all places its entrants.
        let v: serde_json::Value = serde_json::json!({
            "entrants": [["SGPlan5", 5, 10]],
            "field_size": 1
        });
        let c = cohort_from_json("t", &v, &mut w);
        assert_eq!(c.splits, None);
        let b = book("L", c);
        assert_eq!(
            b.cell("L", &[], &Referee::default(), 9, 10),
            "1st of 2 by rate (leader SGPlan5 5/10)"
        );
        assert!(w.is_empty(), "{w:?}");
    }

    /// A `splits` that is not an object is where Python raises. The cell
    /// degrades to the em-dash -- which reads exactly like "no field data is
    /// held" -- so the malformed cohort has to be named on the way past.
    #[test]
    fn a_splits_that_is_not_an_object_degrades_but_is_named() {
        let v: serde_json::Value = serde_json::json!({"splits": null});
        let mut w = Vec::new();
        let c = cohort_from_json("bad", &v, &mut w);
        assert_eq!(c.splits, Some(BTreeMap::new()));
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(
            w[0].contains("bad") && w[0].contains("not an object"),
            "{w:?}"
        );
        let b = book("bad", c);
        assert_eq!(b.cell("bad", &[], &Referee::default(), 1, 2), "\u{2014}");
    }
}
