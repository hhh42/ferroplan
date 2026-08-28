//! The vendored IPC-5 official results archive -- the FIELD every 2006 board is
//! scored against. Ported from `benchmarks/standings.py` (`arch_track` :188,
//! `archive_lengths` :296, `archive_makespans` :325).
//!
//! Three of this reader's rules look like distrust of the data and are in fact
//! scar tissue:
//!
//! * Plan LENGTHS are COUNTED from the action lines, never read from the
//!   `; NrActions` header, because whole planners in this archive ship that
//!   header present and empty. A header-trusting reader scores a board against
//!   blanks.
//! * MAKESPANS are RECOMPUTED from the timed steps as `max(t + duration)`,
//!   never read from `; MakeSpan` -- which is empty on exactly sgplan, the
//!   planner that DOMINATES the temporal tracks. Trusting that header loses the
//!   best-of-field on Time and MetricTime wholesale and quietly scores
//!   ferroplan against the runners-up. (`yochanps/storage/Time/p01.soln` shows
//!   the other half of the argument: its header says 3.02 where the steps say
//!   3.03. One instrument for all planners, or none.)
//! * A zero is not a measurement. Both maps insert only a NON-ZERO value, so a
//!   "no plan found" stub cannot become a best-of-field of 0 and drag every
//!   quality ratio on its instance to nothing.
//!
//! `arch_track` is the join, and its silence is deliberate. Any variant suffix
//! outside the six it knows returns no track at all, which is WHY
//! `storage-time-constraints` and
//! `trucks-time-constraints-timed-initial-literals` never reach a field and
//! render coverage-only. The archive does hold their tracks (`TimeConstraints`,
//! `MetricTimeConstraints`) and this reader ingests them; nothing of ours ever
//! asks for those keys, so they are inert BY CONSTRUCTION rather than removed
//! by a filter. A filter would have to be maintained, and a filter someone
//! forgets to maintain is how a coverage-only board silently acquires a quality
//! number it never measured.
//!
//! DEVIATION, DECLARED -- ONE PASS, NOT TWO. The Python opens the tarball twice
//! and walks it twice, once per dict. This walks it once and fills both. The
//! two are behaviour-identical because the passes never disagree about a
//! member: lengths take EVERY track, makespans take only members whose track
//! contains `"Time"`, and neither decision reads the other's result. The proof
//! is in the tests, which pin both maps' exact key and entry counts (1,094 /
//! 2,436 and 360 / 576) against the numbers the two-pass Python produces, plus
//! the total action-line count over all 2,469 members (206,897).
//!
//! DEVIATION, DECLARED -- TWO PYTHON CRASHES BECOME WARNINGS. The Python does
//! `re.search(...).group(1)` on a basename and `t.extractfile(m).read()` on a
//! member; a name without a `pNN.soln` number raises `AttributeError` and a
//! non-regular member raises the same on `None`. Neither occurs in the
//! committed archive (every one of the 2,469 `.soln` members is a regular file
//! named `pNN.soln`), and a publication tool must not abort a whole standings
//! regeneration over one unreadable member -- so both skip and record a
//! warning. `ArchiveWarnings` exists so a skip is surfaceable rather than
//! silent; silence is the failure mode this file is about. A malformed step
//! number (`float("1.2.3")`, a `ValueError` in Python) is handled the same way:
//! the step is dropped from that file's `max` and named.
//!
//! THE TWO REGEXES CROSSED OVER BY HAND. There is no `regex` crate here, so
//! `^\s*[\d.]+\s*:?\s*\(` (MULTILINE) and
//! `^\s*([\d.]+)\s*:\s*\([^)]*\)\s*(?:\[\s*([\d.]+)\s*\])?` (MULTILINE) are
//! open-coded. They accept EXACTLY the same strings, verified twice: against
//! Python's own engine over all 2,469 members, and over 200,000 random strings
//! drawn from the alphabet these patterns care about. The load-bearing detail
//! is that `\s` matches `\r`: six members of this archive carry CRLF, and one
//! of them, `RESULTS/mips-xxl/TPP/Propositional/p11.soln` with its 140 actions,
//! sits in the propositional set that feeds length scoring. A matcher that
//! accepted only space and tab would drop that plan out of the field, and
//! mips-xxl's 140 is the worst plan on that instance -- losing it would move
//! nothing, which is precisely why nobody would notice. A test pins the 140.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

/// `(domain, track, instance)` -- the archive's join key, exactly the Python
/// tuple. `track` is the directory path BELOW the domain with `/` kept
/// (`"Time"`, but also `"MetricTime/Strips-MetricTime"`), because the archive
/// nests some tracks one level deeper and the Python joins on the joined path.
pub type ArchKey = (String, String, u64);

/// Our 2006 variant name -> `(archive domain dir, archive track dir)`.
///
/// The remainder table is closed on purpose: an unknown suffix yields `None`
/// and therefore no join at all. See the module header for why the constraints
/// variants must keep landing here.
pub fn arch_track(variant: &str) -> (String, Option<String>) {
    // Python's `variant.partition("-")` splits on the FIRST '-' only. Splitting
    // on the last, or on all, would leave `metric-time-strips` as `strips` and
    // send a metric-time board to a track that does not exist.
    let (dom, rest) = match variant.find('-') {
        Some(i) => (&variant[..i], &variant[i + 1..]),
        None => (variant, ""),
    };
    // `ARCH_DOM`, all one entry of it: the archive's domain directories are
    // lowercase-identical to our variant names except TPP, which is shouted.
    let dom = if dom == "tpp" { "TPP" } else { dom };
    let track = match rest {
        "propositional" => Some("Propositional"),
        "propositional-strips" => Some("Propositional/Strips"),
        "time" => Some("Time"),
        "time-strips" => Some("Time/Strips-Time"),
        "metric-time" => Some("MetricTime"),
        "metric-time-strips" => Some("MetricTime/Strips-MetricTime"),
        _ => None,
    };
    (dom.to_string(), track.map(str::to_string))
}

/// The full join in one step: a board row's variant plus its instance number,
/// or `None` where `arch_track` declines to map the variant.
///
/// Callers that hold an `Instance` must reach the `u64` through
/// `Instance::as_num` first -- a multipart label has no archive counterpart and
/// must not be coerced into one.
pub fn arch_key(variant: &str, instance: u64) -> Option<ArchKey> {
    let (dom, track) = arch_track(variant);
    track.map(|t| (dom, t, instance))
}

/// What the pass could not use. Never a hard failure, never silent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArchiveWarnings {
    /// One line per member or step that was skipped, with the member named.
    pub messages: Vec<String>,
    /// `.soln` members seen, before any skipping. 2,469 in the committed
    /// archive; a number that moves means the archive itself moved.
    pub members: usize,
    /// Members whose body yielded no action lines at all. A recorded FACT, not
    /// a defect -- the archive carries 33 such stubs, and the "a zero is not a
    /// measurement" rule is what keeps them out of the field.
    pub empty: usize,
}

impl ArchiveWarnings {
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.messages.iter()
    }
}

/// The archive could not be read. A MISSING archive is not one of these: the
/// Python does `if not os.path.exists(ARCHIVE): return {}`, and a box holding
/// no vendored archive must still be able to regenerate the coverage columns.
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    /// Present but unreadable: a truncated gzip stream, a corrupt tar header, a
    /// permission refusal. Python raises here too.
    #[error("{path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// The IPC-5 official results, indexed for the two currencies the 2006 boards
/// are scored in.
#[derive(Debug, Clone, Default)]
pub struct Ipc5Archive {
    lengths: BTreeMap<ArchKey, BTreeMap<String, u64>>,
    makespans: BTreeMap<ArchKey, BTreeMap<String, f64>>,
    warnings: ArchiveWarnings,
}

impl Ipc5Archive {
    /// Read the vendored `IPC5-results.tgz`.
    ///
    /// A missing file is an EMPTY FIELD, not an error -- the quality columns
    /// then simply do not render, which is what the Python's `os.path.exists`
    /// guard buys. Anything else that goes wrong is reported: an archive that
    /// exists and will not parse is a broken checkout, and publishing a board
    /// with a silently empty field would understate nothing and overstate
    /// everything.
    pub fn open(path: &Path) -> Result<Self, ArchiveError> {
        // The guard is `os.path.exists`, not `ErrorKind::NotFound`, and the
        // difference is the whole point of it. `os.path.exists` swallows
        // EVERY stat failure -- ENOENT, but equally a parent directory that
        // refuses execute, a path component that is not a directory, a symlink
        // loop, a name too long -- and returns False, so the Python renders a
        // coverage-only board. `Path::exists` is `fs::metadata(..).is_ok()`,
        // the same test, verified against Python on all three shapes.
        // Matching only NotFound aborted a whole standings regeneration on the
        // two shapes Python degrades on, and the coverage columns -- which
        // need no archive at all -- are exactly what the guard exists to keep
        // publishable.
        if !path.exists() {
            return Ok(Self::default());
        }
        // Past the guard, Python is `tarfile.open(ARCHIVE)` with no net: a file
        // that stats but will not open (mode 000) or will not parse raises
        // there and is reported here. That includes the race where the archive
        // is unlinked between the stat and the open -- Python loses the same
        // race the same way.
        let file = std::fs::File::open(path).map_err(|source| ArchiveError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::ingest(file).map_err(|source| ArchiveError::Read {
            path: path.display().to_string(),
            source,
        })
    }

    /// Same ingest over an already-open gzip stream. Useful to a test that
    /// builds a synthetic archive, and to any caller holding the bytes.
    pub fn from_gz<R: Read>(reader: R) -> Result<Self, ArchiveError> {
        Self::ingest(reader).map_err(|source| ArchiveError::Read {
            path: "<reader>".to_string(),
            source,
        })
    }

    /// The single walk. See the module header for why it is single.
    fn ingest<R: Read>(reader: R) -> std::io::Result<Self> {
        let mut out = Self::default();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(reader));
        let mut buf: Vec<u8> = Vec::new();

        for entry in archive.entries()? {
            let mut entry = entry?;
            // `String::from_utf8_lossy` rather than `entry.path()`: a member
            // name that is not UTF-8 must still be reportable, and Python's
            // `TarInfo.name` is a (surrogate-escaped) str either way.
            let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
            if !name.ends_with(".soln") {
                continue;
            }
            out.warnings.members += 1;

            // RESULTS/planner/dom/track.../pNN.soln -- five components at the
            // shallowest. Anything shorter cannot name a planner AND a domain
            // AND an instance, so there is nothing to key it by.
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() < 5 {
                out.warnings
                    .messages
                    .push(format!("{name}: fewer than 5 path components, skipped"));
                continue;
            }
            if !entry.header().entry_type().is_file() {
                out.warnings.messages.push(format!(
                    "{name}: not a regular file, skipped (Python would raise on \
                     extractfile's None)"
                ));
                continue;
            }

            let planner = parts[1];
            let dom = parts[2];
            let track = parts[3..parts.len() - 1].join("/");
            let Some(inst) = instance_of(parts[parts.len() - 1]) else {
                out.warnings.messages.push(format!(
                    "{name}: basename carries no pNN.soln instance number, \
                     skipped (Python would raise on search's None)"
                ));
                continue;
            };

            buf.clear();
            entry.read_to_end(&mut buf)?;
            // Python decodes with errors="replace". The committed archive is
            // pure ASCII across all 2,469 members, so this can only matter to a
            // future one -- and it would not matter then either: a replacement
            // char is whitespace in neither language, is not a digit in either,
            // and is not a paren or a bracket, so it can neither create nor
            // destroy a match however many bad bytes collapse into it.
            let body = String::from_utf8_lossy(&buf);

            let n = count_action_lines(&body);
            if n > 0 {
                out.lengths
                    .entry((dom.to_string(), track.clone(), inst))
                    .or_default()
                    .insert(planner.to_string(), n);
            } else {
                out.warnings.empty += 1;
            }

            // The makespan half of the pass. `"Time" not in track` is the
            // Python's own test and it is a SUBSTRING test: it takes Time,
            // Time/Strips-Time, MetricTime, MetricTime/Strips-MetricTime and
            // also TimeConstraints and MetricTimeConstraints, which is exactly
            // the inertness described in the module header.
            if track.contains("Time") {
                let (ms, unparsed) = makespan_of(&body);
                for tok in unparsed {
                    out.warnings.messages.push(format!(
                        "{name}: step `{tok}` has a number that will not parse, \
                         step dropped from this file's makespan"
                    ));
                }
                if ms > 0.0 {
                    out.makespans
                        .entry((dom.to_string(), track.clone(), inst))
                        .or_default()
                        .insert(planner.to_string(), ms);
                }
            }
        }
        Ok(out)
    }

    /// Nothing was read at all -- the missing-archive case. Callers deciding
    /// whether to render a quality column want `has_lengths` or `has_makespans`
    /// instead: those are the two dicts the Python tests for truthiness
    /// separately (`if ... and arch:` for the propositional column,
    /// `if ... and arch_ms:` for the temporal one).
    pub fn is_empty(&self) -> bool {
        self.lengths.is_empty() && self.makespans.is_empty()
    }

    pub fn has_lengths(&self) -> bool {
        !self.lengths.is_empty()
    }

    pub fn has_makespans(&self) -> bool {
        !self.makespans.is_empty()
    }

    /// The field's plan lengths on one instance, or `None` where no planner in
    /// the archive produced a countable plan. Python returns `{}` from a
    /// `.get(key, {})` and the caller does `if not field: continue`; an empty
    /// inner map is never stored here, so `None` and "empty" are the same case.
    pub fn lengths(&self, k: &ArchKey) -> Option<&BTreeMap<String, u64>> {
        self.lengths.get(k)
    }

    pub fn makespans(&self, k: &ArchKey) -> Option<&BTreeMap<String, f64>> {
        self.makespans.get(k)
    }

    /// Best-of-field plan length: Python's `min(field.values())`.
    ///
    /// The planner map is a `BTreeMap` where Python's is insertion-ordered, and
    /// that is safe here for one specific reason: the Python takes the min of
    /// the VALUES, never of the items, so no planner NAME is ever selected
    /// under a tie and the returned number cannot depend on iteration order.
    /// Anything that ever wants the leader's name must fold by hand with a
    /// strict `>` in the archive's own member order, not reach for `max_by_key`.
    pub fn best_length(&self, k: &ArchKey) -> Option<u64> {
        let mut it = self.lengths.get(k)?.values().copied();
        let mut best = it.next()?;
        for v in it {
            if v < best {
                best = v;
            }
        }
        Some(best)
    }

    /// Best-of-field makespan. Same order argument as `best_length`; the strict
    /// `<` keeps the FIRST minimum, matching Python's `min`, so that even a
    /// hypothetical `-0.0`/`0.0` pair would resolve the way Python resolves it.
    pub fn best_makespan(&self, k: &ArchKey) -> Option<f64> {
        let mut it = self.makespans.get(k)?.values().copied();
        let mut best = it.next()?;
        for v in it {
            if v < best {
                best = v;
            }
        }
        Some(best)
    }

    /// The whole length index, for callers that iterate rather than join.
    pub fn lengths_map(&self) -> &BTreeMap<ArchKey, BTreeMap<String, u64>> {
        &self.lengths
    }

    pub fn makespans_map(&self) -> &BTreeMap<ArchKey, BTreeMap<String, f64>> {
        &self.makespans
    }

    pub fn warnings(&self) -> &ArchiveWarnings {
        &self.warnings
    }
}

// ---------------------------------------------------------------------------
// The two hand-rolled matchers.
//
// Both patterns are anchored at `^` under MULTILINE, so a match can only START
// at the beginning of the string or just past a '\n'. `re.finditer`/`findall`
// then scan left to right and never overlap: a match that runs past later '^'
// positions swallows them. Both scanners reproduce that with an explicit floor,
// because it is observable -- see the `indented` test.
// ---------------------------------------------------------------------------

/// Python's `\s` for a `str` pattern, as far as it can matter here.
///
/// `\r` IS in this set. Six members of the archive are CRLF and one of them
/// feeds length scoring; a space-and-tab-only matcher loses it. `\x1c`-`\x1f`
/// ARE in it too -- Python's `\s` follows `str.isspace`, not the Unicode
/// White_Space property, and the two disagree exactly there; a 330k-case
/// differential fuzz against Python's own engine pins it. Python's class
/// additionally covers U+0085, U+00A0, U+2028 and the U+2000 block, none of
/// which can occur: every byte in all 2,469 members is ASCII, and a
/// lossily-decoded replacement char is not whitespace in either language.
#[inline]
fn is_py_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c | 0x1c..=0x1f)
}

/// The `[\d.]` class. `\d` is Unicode-aware in Python and would also accept,
/// say, Arabic-Indic digits; the archive is ASCII, and an ASCII-only class
/// cannot accept anything Python's would reject.
#[inline]
fn is_py_digit_or_dot(c: u8) -> bool {
    c.is_ascii_digit() || c == b'.'
}

fn skip_space(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && is_py_space(b[i]) {
        i += 1;
    }
    i
}

/// The next position `^` matches at under MULTILINE: just past the next '\n'.
///
/// Just past a '\n' and nothing else -- Python's MULTILINE does not treat a
/// bare '\r' as a line break, which is why the CRLF members are read as lines
/// whose CONTENT ends in '\r' rather than as one long line.
fn next_caret(b: &[u8], from: usize) -> Option<usize> {
    if from > b.len() {
        return None;
    }
    b[from..]
        .iter()
        .position(|&c| c == b'\n')
        .map(|i| from + i + 1)
}

/// `^\s*[\d.]+\s*:?\s*\(` at `p`, returning the match end.
///
/// No backtracking is needed anywhere, and that is a property of the pattern
/// rather than a shortcut: every greedy run here is followed by a literal that
/// is not in the run's own class, so a shorter run can never rescue a failed
/// match. The colon being OPTIONAL is what lets the classical `1 (drive a b)`
/// shape count as an action alongside `1: (drive a b)`.
fn match_action_line(b: &[u8], p: usize) -> Option<usize> {
    let i = skip_space(b, p);
    let mut j = i;
    while j < b.len() && is_py_digit_or_dot(b[j]) {
        j += 1;
    }
    if j == i {
        return None; // `[\d.]+` needs at least one
    }
    let mut k = skip_space(b, j);
    if k < b.len() && b[k] == b':' {
        k = skip_space(b, k + 1);
    }
    if k < b.len() && b[k] == b'(' {
        Some(k + 1)
    } else {
        None
    }
}

/// `^\s*([\d.]+)\s*:\s*\([^)]*\)\s*(?:\[\s*([\d.]+)\s*\])?` at `p`, returning
/// `(match end, group 1 range, group 2 range)`.
///
/// The Python's own note on this pattern: sgplan glues the bracket to the
/// paren, mips-xxl spaces everything, yochanps lowercases -- one pattern reads
/// all three, and the classical `T: (action)` shape simply leaves group 2 empty.
/// Here the colon is REQUIRED, unlike the length pattern: an untimed step has no
/// makespan currency.
#[allow(clippy::type_complexity)]
fn match_step(b: &[u8], p: usize) -> Option<(usize, (usize, usize), Option<(usize, usize)>)> {
    let i = skip_space(b, p);
    let mut j = i;
    while j < b.len() && is_py_digit_or_dot(b[j]) {
        j += 1;
    }
    if j == i {
        return None;
    }
    let mut k = skip_space(b, j);
    if !(k < b.len() && b[k] == b':') {
        return None;
    }
    k = skip_space(b, k + 1);
    if !(k < b.len() && b[k] == b'(') {
        return None;
    }
    // `[^)]*\)`: the class excludes ')', so the greedy run stops at the FIRST
    // one -- and it does not exclude '\n', so an action broken across lines
    // still closes here.
    let close = b[k + 1..].iter().position(|&c| c == b')')? + k + 1;

    let mut end = skip_space(b, close + 1);
    let mut dur = None;
    // The optional `[ D ]`. When any part of it fails, the group matches EMPTY
    // and the whole match ends where the preceding greedy `\s*` stopped -- which
    // for a bracketless step is past the newline, and, when the NEXT line is
    // indented, past that line's `^` as well. That next step is then
    // unmatchable and drops out of the file's max. It is faithful rather than
    // tidy, it is what the published numbers were computed with, and the
    // `indented` test pins it so nobody "fixes" it into a silent re-score.
    if end < b.len() && b[end] == b'[' {
        let ds = skip_space(b, end + 1);
        let mut de = ds;
        while de < b.len() && is_py_digit_or_dot(b[de]) {
            de += 1;
        }
        if de > ds {
            let rb = skip_space(b, de);
            if rb < b.len() && b[rb] == b']' {
                dur = Some((ds, de));
                end = rb + 1;
            }
        }
    }
    Some((end, (i, j), dur))
}

/// The plan length of one `.soln`: `len(re.findall(...))` over the action lines.
///
/// Counting, not reading: `; NrActions` is present and empty on whole planners
/// in this archive.
pub fn count_action_lines(body: &str) -> u64 {
    let b = body.as_bytes();
    let mut count = 0u64;
    let mut floor = 0usize; // finditer's non-overlap floor
    let mut caret = Some(0usize);
    while let Some(p) = caret {
        if p >= floor {
            if let Some(end) = match_action_line(b, p) {
                count += 1;
                floor = end;
            }
        }
        caret = next_caret(b, p);
    }
    count
}

/// The makespan of one `.soln`: `max` over its timed steps of `t + duration`,
/// with a missing bracket reading as a duration of zero.
///
/// Returns the makespan and the `T:[D]` text of every step whose numbers would
/// not parse. Python raises `ValueError` on those; see the module header.
/// A file with no timed steps returns `0.0`, and the caller must not store it:
/// zero is the absence of a measurement, not a measurement of zero.
pub fn makespan_of(body: &str) -> (f64, Vec<String>) {
    let b = body.as_bytes();
    let mut ms = 0.0f64;
    let mut unparsed = Vec::new();
    let mut floor = 0usize;
    let mut caret = Some(0usize);
    while let Some(p) = caret {
        if p >= floor {
            if let Some((end, g1, g2)) = match_step(b, p) {
                floor = end;
                let t_tok = body.get(g1.0..g1.1);
                let d_tok = g2.and_then(|r| body.get(r.0..r.1));
                let t = t_tok.and_then(|s| s.parse::<f64>().ok());
                // Python: `float(g2) if g2 else 0.0`. `[\d.]+` can never match
                // an empty string, so "absent" is the only falsy case and a
                // duration of "0" stays a duration.
                let d = match d_tok {
                    None => Some(0.0),
                    Some(s) => s.parse::<f64>().ok(),
                };
                match (t, d) {
                    // `max(ms, ...)` spelled out: replace only on a STRICT
                    // greater, in the archive's own step order. Float addition
                    // is not associative and the published numbers were
                    // accumulated exactly this way.
                    (Some(t), Some(d)) => {
                        let v = t + d;
                        if v > ms {
                            ms = v;
                        }
                    }
                    _ => unparsed.push(match d_tok {
                        Some(d) => format!("{}:[{}]", t_tok.unwrap_or(""), d),
                        None => format!("{}:", t_tok.unwrap_or("")),
                    }),
                }
            }
        }
        caret = next_caret(b, p);
    }
    (ms, unparsed)
}

/// `int(re.search(r"p(\d+)\.soln", basename).group(1))`.
///
/// `search`, not `match`: the number is found anywhere in the basename, and the
/// `.soln` need only follow the digits, not end the name. Leading zeros are
/// decimal, so `p01.soln` is instance 1 -- the archive's own numbering, and the
/// same integer the board raws carry.
fn instance_of(basename: &str) -> Option<u64> {
    let b = basename.as_bytes();
    for i in 0..b.len() {
        if b[i] != b'p' {
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let tail_ok = basename.get(j..).is_some_and(|s| s.starts_with(".soln"));
        if j == i + 1 || !tail_ok {
            continue;
        }
        // The FIRST position that matches is the match, exactly as `re.search`
        // has it -- so this returns here even when the number will not parse.
        // A number too large for u64 cannot be an IPC instance; Python's ints
        // are unbounded, so this is a skip-and-warn rather than a silent wrap.
        return basename.get(i + 1..j).and_then(|s| s.parse::<u64>().ok());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../benchmarks/IPC5-results.tgz")
    }

    fn archive() -> Ipc5Archive {
        Ipc5Archive::open(&archive_path()).expect("IPC5-results.tgz reads")
    }

    fn k(dom: &str, track: &str, inst: u64) -> ArchKey {
        (dom.to_string(), track.to_string(), inst)
    }

    /// Python's `assertAlmostEqual(a, b, places=n)` asserts `round(a-b, n) == 0`.
    ///
    /// `<=`, not `<`: `round` is half-to-EVEN, so a difference of exactly
    /// `0.5 * 10^-n` rounds to `0.0` and Python PASSES on the boundary. A
    /// strict `<` fails there, which is a stricter test than the one this
    /// claims to be.
    fn almost(a: f64, b: f64, places: i32) -> bool {
        (a - b).abs() <= 0.5 * 10f64.powi(-places)
    }

    // ---- the four hand-computed makespans, ported verbatim from -----------
    // benchmarks/test_standings.py::ArchiveMakespans. They are on the record
    // there so that a matcher regression cannot silently re-score a board, and
    // they are on the record here for the same reason.

    /// test_archive_present: the vendored archive is tracked and must parse.
    #[test]
    fn archive_present() {
        let a = archive();
        assert!(!a.is_empty(), "IPC5-results.tgz missing or empty");
        assert!(a.has_lengths() && a.has_makespans());
    }

    /// test_sgplan_empty_header_computed_from_steps: sgplan.ipc04's
    /// `; MakeSpan` header is EMPTY, and sgplan dominates these tracks. The
    /// steps say 0.010+1.000, 1.020+2.000, 3.030+2.000.
    #[test]
    fn sgplan_empty_header_computed_from_steps() {
        let a = archive();
        let v = a.makespans(&k("storage", "Time", 1)).unwrap()["sgplan.ipc04"];
        assert!(almost(v, 5.030, 3), "{v}");
    }

    /// test_mips_xxl_time_and_metric_time: header present and agreeing on the
    /// first, large values and spaced brackets on the second.
    #[test]
    fn mips_xxl_time_and_metric_time() {
        let a = archive();
        let v = a.makespans(&k("storage", "Time", 1)).unwrap()["mips-xxl"];
        assert!(almost(v, 3.00, 3), "{v}");
        let v = a.makespans(&k("TPP", "MetricTime", 1)).unwrap()["mips-xxl"];
        assert!(almost(v, 2734.02, 2), "{v}");
    }

    /// test_yochanps_lowercase_steps: lowercase actions, and a file whose own
    /// header says 3.02 where the steps say 3.03 -- one eps slot apart, which
    /// is exactly why one instrument is used for all planners.
    #[test]
    fn yochanps_lowercase_steps() {
        let a = archive();
        let v = a.makespans(&k("storage", "Time", 1)).unwrap()["yochanps"];
        assert!(almost(v, 3.03, 3), "{v}");
    }

    /// test_propositional_members_not_parsed: a propositional key in the
    /// makespan dict would mean the pass paid for -- and could mis-join
    /// against -- tracks that have no makespan currency.
    #[test]
    fn propositional_members_not_parsed() {
        let a = archive();
        assert!(a.makespans(&k("storage", "Propositional", 1)).is_none());
        assert!(a
            .makespans_map()
            .keys()
            .all(|(_, track, _)| track.contains("Time")));
    }

    /// test_arch_track_maps_the_two_reentry_variants.
    #[test]
    fn arch_track_maps_the_two_reentry_variants() {
        assert_eq!(
            arch_track("storage-time"),
            ("storage".into(), Some("Time".into()))
        );
        assert_eq!(
            arch_track("trucks-time-strips"),
            ("trucks".into(), Some("Time/Strips-Time".into()))
        );
        assert_eq!(
            arch_track("tpp-metric-time"),
            ("TPP".into(), Some("MetricTime".into()))
        );
    }

    // ---- what the Python's tests do not cover, and this port must ---------

    /// Defends the CRLF hazard by name. `RESULTS/mips-xxl/TPP/Propositional/
    /// p11.soln` is one of six CRLF members and the only one in the
    /// propositional set that feeds length scoring; a matcher that took only
    /// space and tab for `\s` would drop its 140-action plan out of the field
    /// and nothing downstream would look wrong.
    #[test]
    fn crlf_propositional_member_counts_all_140_actions() {
        let a = archive();
        let field = a.lengths(&k("TPP", "Propositional", 11)).unwrap();
        assert_eq!(field["mips-xxl"], 140);
        // ...and it is the WORST plan on that instance, so losing it would have
        // moved no published number. That is the whole danger.
        assert_eq!(a.best_length(&k("TPP", "Propositional", 11)), Some(63));
    }

    /// Defends the declared one-pass deviation with the two-pass Python's own
    /// totals: same keys, same entries, same action-line count over every
    /// member. A single walk that filled either map differently would move one
    /// of these five numbers.
    #[test]
    fn one_pass_reproduces_the_two_pass_totals() {
        let a = archive();
        assert_eq!(a.warnings().members, 2469, "`.soln` members");
        assert_eq!(a.lengths_map().len(), 1094, "archive_lengths keys");
        assert_eq!(
            a.lengths_map().values().map(|f| f.len()).sum::<usize>(),
            2436,
            "archive_lengths planner entries"
        );
        assert_eq!(a.makespans_map().len(), 360, "archive_makespans keys");
        assert_eq!(
            a.makespans_map().values().map(|f| f.len()).sum::<usize>(),
            576,
            "archive_makespans planner entries"
        );
        // The matcher's global checksum: every action line in the archive.
        assert_eq!(
            a.lengths_map()
                .values()
                .flat_map(|f| f.values())
                .sum::<u64>(),
            206_897,
            "action lines over all members"
        );
        // 33 members carry no action line at all, and none of them reaches
        // either map. A zero is not a measurement.
        assert_eq!(a.warnings().empty, 33);
        assert!(
            a.warnings().is_empty(),
            "the committed archive must produce no skips: {:?}",
            a.warnings().messages
        );
    }

    /// The length pass takes EVERY track, including the temporal ones -- that
    /// is what makes the single walk equivalent to the Python's two.
    #[test]
    fn lengths_cover_temporal_tracks_too() {
        let a = archive();
        assert_eq!(a.lengths(&k("storage", "Time", 1)).unwrap()["cpt2"], 3);
        assert_eq!(a.best_makespan(&k("storage", "Time", 1)), Some(3.0));
    }

    /// The float accumulation, bit for bit. 3.03 + 2.0 is 5.029999999999999 in
    /// both languages; a "tidier" fold, or a reordered max, would produce a
    /// number that prints the same and diffs differently.
    #[test]
    fn makespan_accumulates_in_python_order() {
        let a = archive();
        let f = a.makespans(&k("storage", "Time", 1)).unwrap();
        assert_eq!(f["sgplan.ipc04"], 3.03f64 + 2.0);
        assert_eq!(f["yochanps"], 1.03f64 + 2.0);
        assert_eq!(f["mips-xxl"], 3.0);
    }

    /// Why the constraints boards render coverage-only: they never map to a
    /// track, so they never reach a field. The archive HOLDS those tracks --
    /// they are inert, not filtered.
    #[test]
    fn constraints_variants_never_join() {
        assert_eq!(arch_track("storage-time-constraints").1, None);
        assert_eq!(
            arch_track("trucks-time-constraints-timed-initial-literals").1,
            None
        );
        let a = archive();
        assert!(a.makespans(&k("storage", "TimeConstraints", 1)).is_some());
    }

    /// `partition("-")` splits on the FIRST hyphen, and a variant with no
    /// hyphen at all still names a domain.
    #[test]
    fn arch_track_partitions_on_the_first_hyphen() {
        assert_eq!(
            arch_track("pipesworld-propositional-strips"),
            ("pipesworld".into(), Some("Propositional/Strips".into()))
        );
        assert_eq!(
            arch_track("openstacks-metric-time-strips"),
            (
                "openstacks".into(),
                Some("MetricTime/Strips-MetricTime".into())
            )
        );
        assert_eq!(arch_track("storage"), ("storage".into(), None));
        assert_eq!(arch_track("tpp"), ("TPP".into(), None));
    }

    /// A missing archive is an empty field, never an error -- a box holding no
    /// vendored archive must still regenerate the coverage columns.
    #[test]
    fn missing_archive_is_an_empty_field() {
        let a = Ipc5Archive::open(Path::new("/nonexistent/IPC5-results.tgz"))
            .expect("a missing archive is not an error");
        assert!(a.is_empty());
        assert!(!a.has_lengths() && !a.has_makespans());
        assert_eq!(a.best_length(&k("storage", "Propositional", 1)), None);
    }

    /// ...and "missing" is `os.path.exists`'s definition of it, not ENOENT's.
    /// A path whose parent component is a regular file stats ENOTDIR, which
    /// `os.path.exists` reports as False and the Python degrades on; a guard
    /// keyed to `ErrorKind::NotFound` alone raised here and took the coverage
    /// columns down with it. (`Cargo.toml` is used because it is the one
    /// regular file this test can be sure exists.)
    #[test]
    fn an_unstattable_archive_path_degrades_like_a_missing_one() {
        let not_a_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .join("IPC5-results.tgz");
        let a = Ipc5Archive::open(&not_a_dir)
            .expect("a path Python's os.path.exists calls absent is not an error");
        assert!(a.is_empty());
        assert!(!a.has_lengths() && !a.has_makespans());
    }

    // ---- the two matchers, on the shapes that made them look odd ----------

    /// Headers are not steps: `; NrActions 140` carries digits and must not
    /// count, which is the whole reason a comment character is not `\s`.
    #[test]
    fn headers_never_count_as_actions() {
        let body = "; Time 0.00\n; NrActions 140\n; MakeSpan\n0: (A) [1]\n";
        assert_eq!(count_action_lines(body), 1);
        assert_eq!(makespan_of(body).0, 1.0);
    }

    /// CRLF at the character level, independent of the archive.
    #[test]
    fn crlf_lines_match() {
        let body = "0: (A) [1]\r\n1: (B) [2]\r\n";
        assert_eq!(count_action_lines(body), 2);
        assert_eq!(makespan_of(body).0, 3.0);
    }

    /// The colon is optional for LENGTH and required for MAKESPAN: an untimed
    /// step is still an action but has no temporal currency.
    #[test]
    fn colonless_step_is_an_action_but_not_a_timed_step() {
        let body = "1 (drive a b)\n";
        assert_eq!(count_action_lines(body), 1);
        assert_eq!(makespan_of(body).0, 0.0);
    }

    /// A bracketless step reads as duration zero, and its match runs past the
    /// newline -- harmlessly, while the next line starts at column zero.
    #[test]
    fn bracketless_steps_read_as_duration_zero() {
        let body = "0: (a)\n1: (b)\n";
        assert_eq!(makespan_of(body).0, 1.0);
        assert_eq!(count_action_lines(body), 2);
    }

    /// ...and NOT harmlessly when the next line is indented: the trailing `\s*`
    /// eats the newline and the indent, leaving no `^` for the next step to
    /// match at, so it is silently dropped from the max. Pinned because it is
    /// what the published numbers were computed with -- changing it is a
    /// re-score, not a bug fix.
    #[test]
    fn indented_bracketless_step_is_swallowed_exactly_as_in_python() {
        let body = "5: (a)\n  7: (b)\n";
        assert_eq!(makespan_of(body).0, 5.0);
        // The length pattern is unaffected: its match ends at the '(', which
        // never reaches the next line.
        assert_eq!(count_action_lines(body), 2);
    }

    /// sgplan's real shape: tab-indented, bracket glued to the paren. This is
    /// the file behind the 5.030 above.
    #[test]
    fn glued_brackets_and_tab_indent() {
        let body = "\t 0.010:  (GO)[1.000]\n\t 1.020:  (LIFT)[2.000]\n";
        assert_eq!(count_action_lines(body), 2);
        assert_eq!(makespan_of(body).0, 3.02);
    }

    /// `[^)]*` does not exclude newlines, so an action broken across lines
    /// still closes -- and the duration after it is still read.
    #[test]
    fn action_may_span_lines() {
        let body = "3: (a\nb) [2]\n";
        assert_eq!(makespan_of(body).0, 5.0);
        assert_eq!(count_action_lines(body), 1);
    }

    /// An all-zero plan yields zero, and the caller must not store it. Zero is
    /// the absence of a measurement.
    #[test]
    fn zero_makespan_is_not_a_measurement() {
        assert_eq!(makespan_of("0: (a) [0]\n").0, 0.0);
        assert_eq!(count_action_lines(""), 0);
        assert_eq!(makespan_of("").0, 0.0);
    }

    /// Python raises `ValueError` on `float("1.2.3")` and loses the whole
    /// standings run. Here the step is dropped and NAMED, and the rest of the
    /// file still scores.
    #[test]
    fn unparseable_step_number_is_skipped_and_named() {
        let (ms, bad) = makespan_of("1.2.3: (a) [1]\n2: (b) [1]\n");
        assert_eq!(ms, 3.0);
        assert_eq!(bad, vec!["1.2.3:[1]".to_string()]);
    }

    /// Rust and Python agree on every token `[\d.]+` can produce: `"1."` and
    /// `".5"` parse in both, a bare `"."` in neither.
    #[test]
    fn float_tokens_parse_the_way_python_parses_them() {
        assert_eq!(makespan_of("1. : (a) [.5]\n").0, 1.5);
        assert_eq!(makespan_of(". : (a) [1]\n").1.len(), 1);
    }

    /// `p(\d+)\.soln` by hand: searched anywhere in the basename, leading zeros
    /// decimal, and nothing else accepted.
    #[test]
    fn instance_numbers_come_from_the_basename() {
        assert_eq!(instance_of("p01.soln"), Some(1));
        assert_eq!(instance_of("p20.soln"), Some(20));
        assert_eq!(instance_of("prob-p7.soln"), Some(7));
        assert_eq!(instance_of("p1p2.soln"), Some(2));
        assert_eq!(instance_of("p.soln"), None);
        assert_eq!(instance_of("plan.txt"), None);
    }
}
