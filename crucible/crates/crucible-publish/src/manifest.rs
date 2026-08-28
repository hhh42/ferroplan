//! `benchmarks/manifest.toml` -- the sweep instrument, read.
//!
//! Until 0.26 the same board was described in five places that could disagree,
//! and every one of them had disagreed at least once:
//!
//! * `standings.py:SWEEPS` -- raw `.jsonl` to (label, competition, budget).
//! * `standings.py:AIR_REBASELINED` -- which box produced a board. A missing
//!   raw means "sweep in flight" for an Air board and "never re-baselined, the
//!   record lives in git history" for a cloud-era one, and rendering both as
//!   "not swept" claims we never measured them at all.
//! * `standings.py:PROOF_TRACKS` -- the boards where coverage IS proof rate.
//!   45% there is a categorically different claim from 45% on a satisficing
//!   board and must never be read as "worse".
//! * `standings.py:MD_FOR` -- the `.md` naming exception, hard-coded in
//!   `main()`. Exactly one board in the whole system has an id that is not its
//!   raw stem (`ipc67-results` for `ipc67-default.jsonl`), and turning that one
//!   fact from code into data is a large part of why this file exists.
//! * `ipc67.py:TRACK_PATTERNS` + `TRACK_IPCS` -- the corpus selector, plus each
//!   sweep driver's own `BOARDS=()` array. `ipc5-complex-pref` was registered in
//!   `SWEEPS` and swept by `post-entries25.sh` while appearing in NO driver
//!   array: a board that the schedule could not see. The manifest's `[[set]]`
//!   tables are where that stops being possible.
//!
//! Two rules here are assertions rather than descriptions, because both have a
//! failure mode that is invisible in the output:
//!
//! * **The mco wall-clock rule.** The competition scores wall time on a fixed
//!   box however many cores a planner burns, so a `--threads N` board runs ONE
//!   instance at a time. The drivers infer that at runtime from the presence of
//!   `--threads`; [`Manifest::validate`] asserts it, so a board can never be
//!   scheduled against the rule by accident and quietly post a number measured
//!   two-at-a-time.
//! * **A tier move in flight is a warning, never a failure.** `ipc5-time` and
//!   `ipc5-metric-time` sweep at 60 s while still SCORED at 30 s, and that is
//!   correct: the timeout class is denominated in the budget a row actually ran
//!   under, so flipping the scored budget before the 60 s raws land would
//!   re-class every 30 s wall-exit as `early-exit` -- a lie in the one column
//!   the refill loop is refereed by. It must be visible, not fatal.
//!
//! # The selector, and what it deliberately cannot do
//!
//! `TRACK_PATTERNS` are Python regexes, and two of them used negative
//! lookbehind (`(?<!metric)-time(-strips)?$`, `(?<!-sat)-numeric-2026`), which
//! Rust's `regex` crate refuses to compile at all -- it costs the linear-time
//! guarantee. `gen-manifest.py` decomposed those two into `include`/`exclude`
//! pairs and PROVED the equivalence against all 292 variant directories on
//! disk before writing the file.
//!
//! What survives is a pattern language with exactly three constructs: literal
//! text, one optional literal group (`(-strips)?`), and one trailing `$`
//! anchor. [`Pattern`] implements those three and **nothing else**. Alternation
//! (`(agl|sat|opt)`), character classes, `.`, `*`, `+`, `^` and escapes are all
//! HARD ERRORS at [`TrackSpec::selector`], surfaced by [`Manifest::validate`],
//! and never a silent partial match -- a selector that picks one variant too
//! many or too few does not fail, it changes a board's denominator, and the
//! wrong denominator is a wrong published number. If a future track needs a
//! construct this matcher lacks, the matcher grows or the track does not ship.
//! (Alternation does appear elsewhere in the port -- the 2018/2023 bounds-file
//! path regex -- but not in a track selector, and it is not smuggled in here.)
//!
//! `$` is Python's non-MULTILINE `$`: end of string, or immediately before a
//! final newline. Variant names come from `readdir` and cannot contain one, so
//! the second case is unreachable today; it is implemented anyway, because an
//! unreachable divergence is just a divergence nobody has reached yet.
//!
//! # Order
//!
//! Board iteration order is the manifest's FILE order -- the standings renderer
//! leans on it for a stable tiebreak, and a TOML array-of-tables deserializes
//! into a `Vec` in document order. Tracks are a `BTreeMap`, which agrees with
//! the file only because `gen-manifest.py` writes them `sorted()`: Python sorts
//! strings by codepoint and Rust's `str` `Ord` is bytewise, and UTF-8 is
//! order-preserving, so the two orders are identical by construction rather
//! than by luck.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The schema this loader understands. A manifest stamped with anything else is
/// refused rather than half-read: the fields it would silently ignore are the
/// fields a board's identity is made of.
pub const SCHEMA: u32 = 1;

/// Every board raw is a `.jsonl`; the board id is its stem.
const RAW_SUFFIX: &str = ".jsonl";

/// THE one board whose id is not its raw stem, and the raw it belongs to.
/// `standings.py` carried this as a two-entry `MD_FOR` dict, of which only this
/// entry was ever an exception -- `ipc67-temporal.jsonl` maps to
/// `ipc67-temporal.md`, which is just the rule.
pub const NAMING_EXCEPTION_ID: &str = "ipc67-results";
/// The raw that [`NAMING_EXCEPTION_ID`] names.
pub const NAMING_EXCEPTION_RAW: &str = "ipc67-default.jsonl";

/// Prefix on the [`Manifest::validate`] lines that are NOT failures.
///
/// `validate` returns one flat list so a caller cannot forget half of it, but a
/// caller that treats the whole list as fatal would refuse to publish a
/// perfectly legitimate manifest -- the tier move in flight is the standing
/// example. Filter on this, or use [`Manifest::errors`].
pub const WARNING: &str = "warning: ";

/// Reading the manifest failed. Both variants name the file: this is read from
/// a path the caller chose, and "invalid TOML" without a path is a bug report
/// nobody can act on.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("{path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

/// A construct the selector language does not have.
///
/// Every variant is a refusal, never a fallback. The whole point of a small
/// matcher is that it cannot half-understand a pattern.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    #[error(
        "unsupported regex construct `{0}`: this selector language is literal text, \
         one optional literal group like `(-strips)?`, and one trailing `$` -- nothing else"
    )]
    Unsupported(char),
    #[error("`$` anchors the end and may only be the final character")]
    MisplacedAnchor,
    #[error("unterminated `(` group")]
    UnterminatedGroup,
    #[error("group `({0})` is not followed by `?`; a required group is not supported")]
    NonOptionalGroup(String),
    #[error("empty pattern would select every variant")]
    Empty,
}

/// One piece of a [`Pattern`].
#[derive(Debug, Clone, PartialEq, Eq)]
enum Term {
    Lit(String),
    /// `(-strips)?` -- greedy, exactly as Python's `?`, with backtracking.
    Opt(String),
}

/// A compiled `include`/`exclude` pattern: literals, optional literal groups,
/// and an optional trailing end-anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    terms: Vec<Term>,
    end_anchor: bool,
}

/// The characters that mean something in a regex and nothing here. `)` is on
/// the list so a stray close paren is a refusal rather than a literal.
fn is_meta(c: char) -> bool {
    matches!(
        c,
        '.' | '*' | '+' | '?' | '[' | ']' | '{' | '}' | '\\' | '|' | '^' | ')'
    )
}

impl Pattern {
    /// Compile, or say precisely what stopped you.
    pub fn parse(src: &str) -> Result<Self, PatternError> {
        if src.is_empty() {
            return Err(PatternError::Empty);
        }
        let mut terms: Vec<Term> = Vec::new();
        let mut lit = String::new();
        let mut end_anchor = false;
        let mut it = src.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '(' => {
                    let mut body = String::new();
                    let mut closed = false;
                    for c2 in it.by_ref() {
                        if c2 == ')' {
                            closed = true;
                            break;
                        }
                        // `is_meta` omits `(` and `$` because the top-level
                        // loop gives them meaning of their own -- but INSIDE a
                        // group there is no such handling, and reading them as
                        // literals would be exactly the silent partial match
                        // this language refuses. `|` lands here too: alternation
                        // is the construct most likely to be reached for next,
                        // and it must fail loudly rather than become a bar.
                        if is_meta(c2) || c2 == '(' || c2 == '$' {
                            return Err(PatternError::Unsupported(c2));
                        }
                        body.push(c2);
                    }
                    if !closed {
                        return Err(PatternError::UnterminatedGroup);
                    }
                    if it.peek() != Some(&'?') {
                        return Err(PatternError::NonOptionalGroup(body));
                    }
                    it.next();
                    if !lit.is_empty() {
                        terms.push(Term::Lit(std::mem::take(&mut lit)));
                    }
                    terms.push(Term::Opt(body));
                }
                '$' => {
                    if it.peek().is_some() {
                        return Err(PatternError::MisplacedAnchor);
                    }
                    end_anchor = true;
                }
                c if is_meta(c) => return Err(PatternError::Unsupported(c)),
                c => lit.push(c),
            }
        }
        if !lit.is_empty() {
            terms.push(Term::Lit(lit));
        }
        if terms.is_empty() {
            // A bare `$` selects every variant, which no track means.
            return Err(PatternError::Empty);
        }
        Ok(Self { terms, end_anchor })
    }

    /// Python's `re.search`: is there ANY position where this matches?
    ///
    /// The scan runs left to right like Python's, though only existence is
    /// observable here -- a track selector asks whether a variant is in the
    /// track, never where the match sat.
    pub fn search(&self, hay: &str) -> bool {
        let mut start = 0usize;
        loop {
            if self.match_from(hay, start, 0) {
                return true;
            }
            if start >= hay.len() {
                return false;
            }
            start += hay[start..].chars().next().map_or(1, char::len_utf8);
        }
    }

    fn match_from(&self, hay: &str, pos: usize, idx: usize) -> bool {
        let Some(term) = self.terms.get(idx) else {
            return self.end_ok(hay, pos);
        };
        match term {
            Term::Lit(s) => {
                hay[pos..].starts_with(s.as_str()) && self.match_from(hay, pos + s.len(), idx + 1)
            }
            // Greedy first, then the empty alternative -- the backtracking that
            // makes `propositional(-strips)?$` refuse
            // `...-propositional-strips-extra`.
            Term::Opt(s) => {
                (hay[pos..].starts_with(s.as_str()) && self.match_from(hay, pos + s.len(), idx + 1))
                    || self.match_from(hay, pos, idx + 1)
            }
        }
    }

    /// Python's non-MULTILINE `$`: end of string, or just before a final `\n`.
    fn end_ok(&self, hay: &str, pos: usize) -> bool {
        if !self.end_anchor {
            return true;
        }
        pos == hay.len() || (pos + 1 == hay.len() && hay.as_bytes()[pos] == b'\n')
    }
}

/// A track's corpus selector: `include` minus `exclude`.
///
/// The pair is how the two negative lookbehinds survive the port. Keep them
/// together in one type so nobody can apply an include and forget its exclude
/// -- `time-2006` without its exclude silently swallows all six metric-time
/// variants, and the board's denominator grows by six with no error anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    include: Pattern,
    exclude: Option<Pattern>,
}

impl Selector {
    pub fn is_match(&self, variant: &str) -> bool {
        self.include.search(variant) && !self.exclude.as_ref().is_some_and(|e| e.search(variant))
    }
}

/// `[corpus]` -- where the instances live and how a domain file is found.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusSpec {
    /// Relative to `benchmarks/`; `$FERROPLAN_IPC_CORPUS` wins over it.
    pub root: String,
    pub domain_shared: String,
    /// `{first}` is the instance name's FIRST digit group -- the pairing
    /// convention that keeps working for multipart names like `3_10_50_10`.
    pub domain_per_instance: String,
}

/// `[defaults]` -- the box-shaped knobs a board inherits unless it says
/// otherwise. The values carry their reasons in the TOML's own comments
/// (`jobs = 2` not 3 on a fanless box; `mem_gb = 6` not 8 so two jobs against
/// 16 GiB leave headroom for the RSS poll).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Defaults {
    pub timeout_secs: u64,
    pub jobs: u32,
    pub threads: u32,
    pub mode: String,
    pub mem_gb: f64,
}

/// `[track.NAME]` -- the corpus selector.
///
/// Deliberately NOT an enumeration of instances: the corpus is gitignored, so a
/// list of 6,584 paths would drift from disk with nothing to notice.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackSpec {
    /// Which `ipc-YYYY` directories to scan, in order.
    pub ipcs: Vec<String>,
    pub include: String,
    /// Present only on the two tracks that were negative lookbehinds.
    #[serde(default)]
    pub exclude: Option<String>,
}

impl TrackSpec {
    /// Compile the selector. Cheap -- these patterns are a few dozen bytes --
    /// but not free, so hoist it out of a loop over a corpus.
    pub fn selector(&self) -> Result<Selector, PatternError> {
        Ok(Selector {
            include: Pattern::parse(&self.include)?,
            exclude: match &self.exclude {
                Some(e) => Some(Pattern::parse(e)?),
                None => None,
            },
        })
    }

    /// Does this track contain `variant`? Returns the compile error rather than
    /// a verdict, because "no" and "I could not read the pattern" are not the
    /// same answer and only one of them is safe to act on.
    pub fn selects(&self, variant: &str) -> Result<bool, PatternError> {
        Ok(self.selector()?.is_match(variant))
    }
}

/// `[[board]]` -- the unit of work AND the unit of row identity.
///
/// The resume gate compares (budget, mode, jobs, threads) EXACTLY, so this is
/// the tuple every row is stamped with. Renaming a field here re-identifies
/// every row already on disk.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardSpec {
    pub id: String,
    pub raw: String,
    /// The scoreboard sibling. `ipc67.py` writes it at sweep END, so a raw
    /// without its `.md` is a sweep still in flight and must not be read as a
    /// finished board.
    pub md: String,
    /// The name this board publishes under. Labels are the join key between the
    /// manifest and the standings tables, which is why they must be unique.
    pub label: String,
    pub competition: String,
    /// The budget a row is SCORED against. Not necessarily the wall a sweep
    /// arms -- see `timeout_secs`.
    pub budget_secs: f64,
    pub track: String,
    /// The wall the sweep arms, when it differs from `budget_secs`: a tier move
    /// in flight. A row measured since 0.23 carries its own `budget` stamp and
    /// that stamp wins at classification time, so the two can diverge for one
    /// cycle without a single row being misclassified.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// `ff --mode` passthrough; `None` means the manifest's default.
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub threads: Option<u32>,
    #[serde(default)]
    pub jobs: Option<u32>,
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// `FF_*` hatches this board declares.
    ///
    /// The runner scrubs the child environment and applies exactly this, so a
    /// row can never have been measured under a hatch that is not on the
    /// record. `ipc67.py` builds the environment as `dict(os.environ, ...)`,
    /// which means any `FF_*` exported in the operator's shell silently changes
    /// every board in the sweep with nothing anywhere recording that it
    /// happened -- and there are 132 such hatches in the engine.
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, String>,
    /// Coverage IS proof rate on this board: every solved row carries an
    /// optimality certificate.
    #[serde(default)]
    pub proof_track: bool,
    /// Which boxes have produced this board. EMPTY is meaningful: it marks a
    /// cloud-era board that was never re-baselined, whose absence of a raw must
    /// render "see git history" and not "sweep in flight".
    #[serde(default)]
    pub rebaselined_on: Vec<String>,
}

/// `[[set]]` -- what one driver invocation sweeps.
///
/// Two staging directories by design: the standing 22 keep their like-for-like
/// identity, the entries stage apart, and the cut record carries two headlines.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetSpec {
    pub name: String,
    pub stage: String,
    /// The workspace version this set was scoped for, as a string -- version
    /// ORDER is a comparison this type deliberately does not offer, because the
    /// only ordering that has ever mattered here (previous release, same box)
    /// belongs to the history module and got it wrong once already.
    #[serde(default)]
    pub requires_version: Option<String>,
    pub boards: Vec<String>,
}

/// The whole instrument.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub corpus: CorpusSpec,
    pub defaults: Defaults,
    /// `[track.NAME]`. A `BTreeMap` because the file is written sorted; see the
    /// module header on why the two orders agree by construction.
    #[serde(rename = "track", default)]
    pub tracks: BTreeMap<String, TrackSpec>,
    /// `[[board]]`, in FILE order. Do not sort this.
    #[serde(rename = "board", default)]
    pub boards: Vec<BoardSpec>,
    #[serde(rename = "set", default)]
    pub sets: Vec<SetSpec>,
}

impl Manifest {
    /// Read and parse. A missing manifest is an ERROR, not an empty manifest:
    /// the other loaders in this crate degrade to nothing when their optional
    /// input is absent, but a sweep with no instrument is not a sweep with
    /// nothing to say -- it is a sweep that cannot be described.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: shown.clone(),
            source,
        })?;
        Self::parse(&text, &shown)
    }

    /// Parse from text already in hand, naming `path` in any error.
    pub fn parse(text: &str, path: &str) -> Result<Self, ManifestError> {
        toml::from_str(text).map_err(|source| ManifestError::Parse {
            path: path.to_string(),
            source,
        })
    }

    /// Every problem, in a stable order -- never just the first.
    ///
    /// A manifest with two faults that reports one gets fixed twice, and the
    /// second fix lands after somebody has already re-run a multi-hour sweep.
    /// Lines prefixed [`WARNING`] are legitimate states that must be VISIBLE,
    /// not fatal; see [`Manifest::errors`].
    pub fn validate(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut warn: Vec<String> = Vec::new();

        if self.schema != SCHEMA {
            out.push(format!(
                "schema {} is not the schema this loader understands ({SCHEMA})",
                self.schema
            ));
        }

        // Uniqueness, reported at the SECOND occurrence: the message should
        // name the entry that broke the rule, not the innocent first one.
        let mut ids: BTreeSet<&str> = BTreeSet::new();
        let mut raws: BTreeSet<&str> = BTreeSet::new();
        let mut labels: BTreeSet<&str> = BTreeSet::new();
        for b in &self.boards {
            if !ids.insert(&b.id) {
                out.push(format!("duplicate board id `{}`", b.id));
            }
            if !raws.insert(&b.raw) {
                out.push(format!(
                    "duplicate board raw `{}` (board `{}`)",
                    b.raw, b.id
                ));
            }
            if !labels.insert(&b.label) {
                // Labels join the manifest to the standings tables and to the
                // history snapshots. Two boards sharing one is two boards
                // sharing a published row.
                out.push(format!(
                    "duplicate board label `{}` (board `{}`)",
                    b.label, b.id
                ));
            }
        }

        // The `.md` naming rule, and its single exception.
        let mut odd: Vec<&str> = Vec::new();
        for b in &self.boards {
            match b.raw.strip_suffix(RAW_SUFFIX) {
                None => out.push(format!(
                    "board `{}` raw `{}` does not end in `{RAW_SUFFIX}`",
                    b.id, b.raw
                )),
                Some(stem) => {
                    if b.id != stem {
                        odd.push(&b.id);
                    }
                }
            }
            if b.md != format!("{}.md", b.id) {
                out.push(format!(
                    "board `{}` md `{}` should be `{}.md`: the scoreboard name is the board id",
                    b.id, b.md, b.id
                ));
            }
        }
        if odd.len() != 1 || odd[0] != NAMING_EXCEPTION_ID {
            out.push(format!(
                "exactly one board may have an id that is not its raw stem, and it must be \
                 `{NAMING_EXCEPTION_ID}` (raw `{NAMING_EXCEPTION_RAW}`); found {odd:?}"
            ));
        } else if let Some(b) = self.board(NAMING_EXCEPTION_ID) {
            if b.raw != NAMING_EXCEPTION_RAW {
                out.push(format!(
                    "`{NAMING_EXCEPTION_ID}` is the exception for raw `{NAMING_EXCEPTION_RAW}`, \
                     not `{}`",
                    b.raw
                ));
            }
        }

        for b in &self.boards {
            if !self.tracks.contains_key(&b.track) {
                out.push(format!(
                    "board `{}` names track `{}`, which no [track.*] declares",
                    b.id, b.track
                ));
            }
        }

        // A pattern that does not compile is not a track that selects nothing;
        // it is a track nobody can schedule. Report it here so one `validate`
        // call is the whole gate.
        for (name, t) in &self.tracks {
            if let Err(e) = t.selector() {
                out.push(format!("track `{name}` selector: {e}"));
            }
        }

        // The mco wall-clock rule, asserted rather than inferred.
        //
        // BOTH sides resolve through `[defaults]`, because `None` on a board
        // means "the manifest's default" and nothing else. Reading an unset
        // `threads` as a hardcoded 1 would let every board escape this
        // assertion the day the default is raised above 1 -- which is the
        // exact day the rule starts to bite.
        for b in &self.boards {
            let threads = b.threads.unwrap_or(self.defaults.threads);
            let jobs = b.jobs.unwrap_or(self.defaults.jobs);
            if threads > 1 && jobs != 1 {
                out.push(format!(
                    "board `{}` burns {threads} threads but declares jobs = {}: the competition \
                     scores WALL TIME on a fixed box however many cores a planner uses, so a \
                     multi-threaded board must run one instance at a time (jobs = 1)",
                    b.id,
                    b.jobs
                        .map_or_else(|| format!("unset (default {jobs})"), |j| j.to_string())
                ));
            }
        }

        for s in &self.sets {
            for id in &s.boards {
                if self.board(id).is_none() {
                    out.push(format!(
                        "set `{}` names board `{id}`, which no [[board]] declares",
                        s.name
                    ));
                }
            }
        }

        // --- warnings: legitimate states that must not be silent -------------
        for b in &self.boards {
            if let Some(t) = b.timeout_secs {
                // Both sides are small integers, exact in f64 -- but convert
                // EXACTLY or not at all: a saturating cast would let an
                // out-of-range wall compare equal to a budget and drop the one
                // warning that says the two tiers have diverged.
                if !u32::try_from(t).is_ok_and(|secs| f64::from(secs) == b.budget_secs) {
                    warn.push(format!(
                        "{WARNING}board `{}` arms a {t}s wall but is scored at {}s: a TIER MOVE \
                         IN FLIGHT. Legitimate -- rows carry their own budget stamp and classify \
                         against it -- but the two must be reconciled at promote time",
                        b.id, b.budget_secs
                    ));
                }
            }
        }
        for b in &self.boards {
            if !self.sets.iter().any(|s| s.boards.contains(&b.id)) {
                warn.push(format!(
                    "{WARNING}board `{}` belongs to no [[set]]: registered, but nothing sweeps it",
                    b.id
                ));
            }
        }

        out.extend(warn);
        out
    }

    /// [`Manifest::validate`] minus the [`WARNING`] lines -- the fatal half.
    pub fn errors(&self) -> Vec<String> {
        self.validate()
            .into_iter()
            .filter(|l| !l.starts_with(WARNING))
            .collect()
    }

    /// [`Manifest::validate`]'s [`WARNING`] lines only.
    pub fn warnings(&self) -> Vec<String> {
        self.validate()
            .into_iter()
            .filter(|l| l.starts_with(WARNING))
            .collect()
    }

    /// First board with this id, in file order. `validate` rejects duplicates,
    /// so "first" is only a tiebreak for a manifest already known bad.
    pub fn board(&self, id: &str) -> Option<&BoardSpec> {
        self.boards.iter().find(|b| b.id == id)
    }

    /// The board that produced a given raw `.jsonl`. This is `SWEEPS`, inverted.
    pub fn board_by_raw(&self, raw: &str) -> Option<&BoardSpec> {
        self.boards.iter().find(|b| b.raw == raw)
    }

    /// The board a standings row's label belongs to.
    pub fn board_by_label(&self, label: &str) -> Option<&BoardSpec> {
        self.boards.iter().find(|b| b.label == label)
    }

    pub fn track(&self, name: &str) -> Option<&TrackSpec> {
        self.tracks.get(name)
    }

    pub fn set(&self, name: &str) -> Option<&SetSpec> {
        self.sets.iter().find(|s| s.name == name)
    }

    /// `AIR_REBASELINED`, as data: has `label`'s board been measured on `box_`?
    ///
    /// A label with no board answers `false`, which is exactly what the Python
    /// set membership answered, and renders the cloud-era note rather than
    /// "sweep in flight". Getting this backwards claims a board was never
    /// measured when its numbers are sitting in git history.
    pub fn rebaselined_on(&self, label: &str, box_: &str) -> bool {
        self.board_by_label(label)
            .is_some_and(|b| b.rebaselined_on.iter().any(|x| x == box_))
    }

    /// `PROOF_TRACKS`, as data: is this label's coverage a proof rate?
    pub fn is_proof_track(&self, label: &str) -> bool {
        self.board_by_label(label).is_some_and(|b| b.proof_track)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------
    // Baked corpus + expected selections. The corpus is gitignored, so a test
    // that reads it from disk would silently skip on every box that has not
    // fetched 6,584 instances -- and a selector test that skips proves nothing.
    // ---------------------------------------------------------------------

    // Three baked tables, all in one shape: one record per line, first word
    // the key. They are string literals rather than nested arrays only so
    // `rustfmt` leaves them at a readable density -- it explodes an array of
    // 292 short literals to one element per line.

    /// Every variant directory in the vendored corpus -- all 292, across the
    /// eight competition dirs -- so the proof below runs on a box that has never
    /// fetched the (gitignored) corpus. Key: the `ipc-YYYY` dir; then its
    /// variants in `sorted()` order.
    const CORPUS: &str = "\
        ipc-2006 openstacks-metric-time openstacks-metric-time-strips \
        openstacks-preferences-qualitative openstacks-preferences-simple \
        openstacks-propositional openstacks-propositional-strips openstacks-time \
        openstacks-time-strips pathways-metric-time pathways-preferences-complex \
        pathways-preferences-simple pathways-propositional pathways-propositional-strips \
        pipesworld-metric-time pipesworld-metric-time-constraints \
        pipesworld-preferences-complex pipesworld-propositional \
        pipesworld-propositional-strips rovers-metric-preferences-simple rovers-metric-time \
        rovers-preferences-qualitative rovers-propositional rovers-propositional-strips \
        storage-preferences-complex storage-preferences-qualitative storage-preferences-simple \
        storage-preferences-simple-grounded-preferences storage-propositional storage-time \
        storage-time-constraints tpp-metric tpp-metric-time tpp-metric-time-constraints \
        tpp-preferences-complex tpp-preferences-qualitative tpp-preferences-simple \
        tpp-preferences-simple-grounded-preferences tpp-propositional tpp-propositional-strips \
        trucks-preferences-complex trucks-preferences-qualitative trucks-preferences-simple \
        trucks-preferences-simple-grounded trucks-preferences-simple-grounded-preferences \
        trucks-propositional trucks-propositional-strips trucks-time trucks-time-constraints \
        trucks-time-constraints-timed-initial-literals trucks-time-strips
        ipc-2008 crew-planning-net-benefit-optimal-numeric-fluents \
        crew-planning-temporal-satisficing-strips cyber-security-sequential-satisficing-strips \
        elevator-net-benefit-optimal-numeric-fluents elevator-net-benefit-optimal-strips \
        elevator-sequential-optimal-strips elevator-sequential-satisficing-strips \
        elevator-temporal-satisficing-numeric-fluents elevator-temporal-satisficing-strips \
        model-train-temporal-satisficing-numeric-fluents openstacks-net-benefit-optimal-adl \
        openstacks-net-benefit-optimal-adl-numeric-fluents \
        openstacks-net-benefit-optimal-strips-negative-preconditions \
        openstacks-sequential-optimal-adl openstacks-sequential-optimal-strips \
        openstacks-sequential-satisficing-adl openstacks-sequential-satisficing-strips \
        openstacks-temporal-satisficing-adl \
        openstacks-temporal-satisficing-adl-numeric-fluents \
        openstacks-temporal-satisficing-numeric-fluents openstacks-temporal-satisficing-strips \
        parc-printer-sequential-optimal-strips parc-printer-sequential-satisficing-strips \
        parc-printer-temporal-satisficing-strips peg-solitaire-net-benefit-optimal-strips \
        peg-solitaire-sequential-optimal-strips peg-solitaire-sequential-satisficing-strips \
        peg-solitaire-temporal-satisficing-strips scanalyzer-3d-sequential-optimal-strips \
        scanalyzer-3d-sequential-satisficing-strips sokoban-sequential-optimal-strips \
        sokoban-sequential-satisficing-strips sokoban-temporal-satisficing-strips \
        transport-net-benefit-optimal-numeric-fluents transport-sequential-optimal-strips \
        transport-sequential-satisficing-strips transport-temporal-satisficing-numeric-fluents \
        woodworking-net-benefit-optimal-numeric-fluents woodworking-sequential-optimal-strips \
        woodworking-sequential-satisficing-strips \
        woodworking-temporal-satisficing-numeric-fluents
        ipc-2011 barman-sequential-multi-core barman-sequential-optimal \
        barman-sequential-satisficing crew-planning-temporal-satisficing \
        elevator-sequential-multi-core elevator-sequential-optimal \
        elevator-sequential-satisficing elevator-temporal-satisficing \
        floor-tile-sequential-multi-core floor-tile-sequential-optimal \
        floor-tile-sequential-satisficing floor-tile-temporal-satisficing \
        match-cellar-temporal-satisficing no-mystery-sequential-multi-core \
        no-mystery-sequential-optimal no-mystery-sequential-satisficing \
        openstacks-sequential-multi-core openstacks-sequential-optimal \
        openstacks-sequential-satisficing openstacks-temporal-satisficing \
        parc-printer-sequential-multi-core parc-printer-sequential-optimal \
        parc-printer-sequential-satisficing parc-printer-temporal-satisficing \
        parking-sequential-multi-core parking-sequential-optimal \
        parking-sequential-satisficing parking-temporal-satisficing \
        peg-solitaire-sequential-multi-core peg-solitaire-sequential-optimal \
        peg-solitaire-sequential-satisficing peg-solitaire-temporal-satisficing \
        scanalyzer-3d-sequential-multi-core scanalyzer-3d-sequential-optimal \
        scanalyzer-3d-sequential-satisficing sokoban-sequential-multi-core \
        sokoban-sequential-optimal sokoban-sequential-satisficing sokoban-temporal-satisficing \
        storage-temporal-satisficing temporal-machine-shop-temporal-satisficing \
        tidybot-sequential-multi-core tidybot-sequential-optimal \
        tidybot-sequential-satisficing transport-sequential-multi-core \
        transport-sequential-optimal transport-sequential-satisficing \
        turn-and-open-temporal-satisficing visit-all-sequential-multi-core \
        visit-all-sequential-optimal visit-all-sequential-satisficing \
        woodworking-sequential-multi-core woodworking-sequential-optimal \
        woodworking-sequential-satisficing
        ipc-2014 barman-sequential-agile barman-sequential-multi-core \
        barman-sequential-optimal barman-sequential-satisficing cave-diving-sequential-agile \
        cave-diving-sequential-multi-core cave-diving-sequential-optimal \
        cave-diving-sequential-satisficing child-snack-sequential-agile \
        child-snack-sequential-multi-core child-snack-sequential-optimal \
        child-snack-sequential-satisficing city-car-sequential-agile \
        city-car-sequential-multi-core city-car-sequential-optimal \
        city-car-sequential-satisficing driver-log-temporal-satisficing \
        floor-tile-sequential-agile floor-tile-sequential-multi-core \
        floor-tile-sequential-optimal floor-tile-sequential-satisficing \
        floor-tile-temporal-satisficing genome-edit-distances-sequential-agile \
        genome-edit-distances-sequential-multi-core genome-edit-distances-sequential-optimal \
        genome-edit-distances-sequential-satisficing hiking-sequential-agile \
        hiking-sequential-multi-core hiking-sequential-optimal hiking-sequential-satisficing \
        maintenance-sequential-agile maintenance-sequential-multi-core \
        maintenance-sequential-optimal maintenance-sequential-satisficing \
        map-analyzer-temporal-satisficing match-cellar-temporal-satisficing \
        openstacks-sequential-agile openstacks-sequential-multi-core \
        openstacks-sequential-optimal openstacks-sequential-satisficing \
        parking-sequential-agile parking-sequential-multi-core parking-sequential-optimal \
        parking-sequential-satisficing parking-temporal-satisficing \
        road-traffic-accident-management-temporal-satisficing satellite-temporal-satisficing \
        storage-temporal-satisficing temporal-machine-shop-temporal-satisficing \
        tetris-sequential-agile tetris-sequential-multi-core tetris-sequential-optimal \
        tetris-sequential-satisficing thoughtful-sequential-agile \
        thoughtful-sequential-multi-core thoughtful-sequential-satisficing \
        tidybot-sequential-optimal transport-sequential-agile transport-sequential-multi-core \
        transport-sequential-optimal transport-sequential-satisficing \
        turn-and-open-temporal-satisficing visit-all-sequential-agile \
        visit-all-sequential-multi-core visit-all-sequential-optimal \
        visit-all-sequential-satisficing
        ipc-2018 agricola-sequential-optimal agricola-sequential-satisficing \
        caldera-sequential-optimal caldera-sequential-satisficing \
        caldera-split-sequential-optimal caldera-split-sequential-satisficing \
        data-network-sequential-optimal data-network-sequential-satisficing \
        flashfill-sequential-satisficing nurikabe-sequential-optimal \
        nurikabe-sequential-satisficing organic-synthesis-sequential-optimal \
        organic-synthesis-sequential-satisficing organic-synthesis-split-sequential-optimal \
        organic-synthesis-split-sequential-satisficing petri-net-alignment-sequential-optimal \
        settlers-sequential-optimal settlers-sequential-satisficing snake-sequential-optimal \
        snake-sequential-satisficing spider-sequential-optimal spider-sequential-satisficing \
        termes-sequential-optimal termes-sequential-satisficing
        ipc-2023 folding-agile folding-optimal folding-satisficing labyrinth-agile \
        labyrinth-optimal labyrinth-satisficing quantum-layout-agile quantum-layout-optimal \
        quantum-layout-satisficing recharging-robots-agile recharging-robots-optimal \
        recharging-robots-satisficing ricochet-robots-agile ricochet-robots-optimal \
        ricochet-robots-satisficing rubiks-cube-agile rubiks-cube-optimal \
        rubiks-cube-satisficing slitherlink-agile slitherlink-optimal slitherlink-satisficing
        ipc-2023n block-grouping-numeric-satisficing counters-numeric-satisficing \
        delivery-numeric-satisficing drone-numeric-satisficing expedition-numeric-satisficing \
        ext-plant-watering-numeric-satisficing farmland-numeric-satisficing \
        fo-counters-numeric-satisficing fo-farmland-numeric-satisficing \
        fo-sailing-numeric-satisficing hydropower-numeric-satisficing \
        markettrader-numeric-satisficing mprime-numeric-satisficing \
        pathwaysmetric-numeric-satisficing rover-numeric-satisficing \
        sailing-numeric-satisficing settlersnumeric-numeric-satisficing \
        sugar-numeric-satisficing tpp-numeric-satisficing zenotravel-numeric-satisficing
        ipc-2026n 2048-numeric-2026 coins-numeric-2026 expedition-numeric-2026 \
        factory-robot-numeric-2026 forestfire-numeric-2026 gear-car-numeric-2026 \
        line-exchange-snp-numeric-2026 onlycraft-opt-numeric-2026 onlycraft-sat-numeric-2026 \
        petri-net-numeric-2026 rainbowttles-opt-numeric-2026 rainbowttles-sat-numeric-2026 \
        sailing-wind-opt-numeric-2026 sailing-wind-sat-numeric-2026 settlers-snp-numeric-2026 \
        ztalloc-sum-numeric-2026
        ";

    /// The 26 track selectors exactly as `benchmarks/manifest.toml` carries
    /// them. Key: track name; then `include`, then `exclude` (`-` where the
    /// track has none -- no real pattern is a bare hyphen), then the
    /// `ipc-YYYY` dirs it scans, in the order it declares them.
    const SELECTORS: &str = "\
        agile-2023 -agile$ - ipc-2023
        complex-pref-2006 preferences-complex$ - ipc-2006
        constraints-2006 constraints - ipc-2006
        metric-time-2006 metric-time(-strips)?$ - ipc-2006
        net-benefit net-benefit - ipc-2008 ipc-2011
        numeric-2023 numeric-satisficing - ipc-2023n
        numeric-2026 numeric-2026 - ipc-2026n
        opt-2018 sequential-optimal - ipc-2018
        opt-2023 -optimal$ - ipc-2023
        opt-2026 -opt-numeric-2026 - ipc-2026n
        opt-2026-full -numeric-2026 -sat-numeric-2026 ipc-2026n
        prop-2006 propositional(-strips)?$ - ipc-2006
        qual-pref-2006 preferences-qualitative$ - ipc-2006
        sat-2018 sequential-satisficing - ipc-2018
        sat-2023 -satisficing$ - ipc-2023
        seq-agile-2014 sequential-agile - ipc-2014
        seq-mco sequential-multi-core - ipc-2011
        seq-mco-2014 sequential-multi-core - ipc-2014
        seq-opt sequential-optimal - ipc-2008 ipc-2011
        seq-opt-2014 sequential-optimal - ipc-2014
        seq-sat sequential-satisficing - ipc-2008 ipc-2011
        seq-sat-2014 sequential-satisficing - ipc-2014
        simple-pref-2006 preferences-simple$ - ipc-2006
        tempo-sat temporal-satisficing - ipc-2008 ipc-2011
        tempo-sat-2014 temporal-satisficing - ipc-2014
        time-2006 -time(-strips)?$ metric-time(-strips)?$ ipc-2006
        ";

    /// What `python3 benchmarks/ipc67.py --track T --list` selected over that
    /// corpus: the Python `re.search` verdict, frozen. Key: track name.
    const EXPECTED: &str = "\
        agile-2023 ipc-2023/folding-agile ipc-2023/labyrinth-agile \
        ipc-2023/quantum-layout-agile ipc-2023/recharging-robots-agile \
        ipc-2023/ricochet-robots-agile ipc-2023/rubiks-cube-agile ipc-2023/slitherlink-agile
        complex-pref-2006 ipc-2006/pathways-preferences-complex \
        ipc-2006/pipesworld-preferences-complex ipc-2006/storage-preferences-complex \
        ipc-2006/tpp-preferences-complex ipc-2006/trucks-preferences-complex
        constraints-2006 ipc-2006/pipesworld-metric-time-constraints \
        ipc-2006/storage-time-constraints ipc-2006/tpp-metric-time-constraints \
        ipc-2006/trucks-time-constraints \
        ipc-2006/trucks-time-constraints-timed-initial-literals
        metric-time-2006 ipc-2006/openstacks-metric-time \
        ipc-2006/openstacks-metric-time-strips ipc-2006/pathways-metric-time \
        ipc-2006/pipesworld-metric-time ipc-2006/rovers-metric-time ipc-2006/tpp-metric-time
        net-benefit ipc-2008/crew-planning-net-benefit-optimal-numeric-fluents \
        ipc-2008/elevator-net-benefit-optimal-numeric-fluents \
        ipc-2008/elevator-net-benefit-optimal-strips \
        ipc-2008/openstacks-net-benefit-optimal-adl \
        ipc-2008/openstacks-net-benefit-optimal-adl-numeric-fluents \
        ipc-2008/openstacks-net-benefit-optimal-strips-negative-preconditions \
        ipc-2008/peg-solitaire-net-benefit-optimal-strips \
        ipc-2008/transport-net-benefit-optimal-numeric-fluents \
        ipc-2008/woodworking-net-benefit-optimal-numeric-fluents
        numeric-2023 ipc-2023n/block-grouping-numeric-satisficing \
        ipc-2023n/counters-numeric-satisficing ipc-2023n/delivery-numeric-satisficing \
        ipc-2023n/drone-numeric-satisficing ipc-2023n/expedition-numeric-satisficing \
        ipc-2023n/ext-plant-watering-numeric-satisficing \
        ipc-2023n/farmland-numeric-satisficing ipc-2023n/fo-counters-numeric-satisficing \
        ipc-2023n/fo-farmland-numeric-satisficing ipc-2023n/fo-sailing-numeric-satisficing \
        ipc-2023n/hydropower-numeric-satisficing ipc-2023n/markettrader-numeric-satisficing \
        ipc-2023n/mprime-numeric-satisficing ipc-2023n/pathwaysmetric-numeric-satisficing \
        ipc-2023n/rover-numeric-satisficing ipc-2023n/sailing-numeric-satisficing \
        ipc-2023n/settlersnumeric-numeric-satisficing ipc-2023n/sugar-numeric-satisficing \
        ipc-2023n/tpp-numeric-satisficing ipc-2023n/zenotravel-numeric-satisficing
        numeric-2026 ipc-2026n/2048-numeric-2026 ipc-2026n/coins-numeric-2026 \
        ipc-2026n/expedition-numeric-2026 ipc-2026n/factory-robot-numeric-2026 \
        ipc-2026n/forestfire-numeric-2026 ipc-2026n/gear-car-numeric-2026 \
        ipc-2026n/line-exchange-snp-numeric-2026 ipc-2026n/onlycraft-opt-numeric-2026 \
        ipc-2026n/onlycraft-sat-numeric-2026 ipc-2026n/petri-net-numeric-2026 \
        ipc-2026n/rainbowttles-opt-numeric-2026 ipc-2026n/rainbowttles-sat-numeric-2026 \
        ipc-2026n/sailing-wind-opt-numeric-2026 ipc-2026n/sailing-wind-sat-numeric-2026 \
        ipc-2026n/settlers-snp-numeric-2026 ipc-2026n/ztalloc-sum-numeric-2026
        opt-2018 ipc-2018/agricola-sequential-optimal ipc-2018/caldera-sequential-optimal \
        ipc-2018/caldera-split-sequential-optimal ipc-2018/data-network-sequential-optimal \
        ipc-2018/nurikabe-sequential-optimal ipc-2018/organic-synthesis-sequential-optimal \
        ipc-2018/organic-synthesis-split-sequential-optimal \
        ipc-2018/petri-net-alignment-sequential-optimal ipc-2018/settlers-sequential-optimal \
        ipc-2018/snake-sequential-optimal ipc-2018/spider-sequential-optimal \
        ipc-2018/termes-sequential-optimal
        opt-2023 ipc-2023/folding-optimal ipc-2023/labyrinth-optimal \
        ipc-2023/quantum-layout-optimal ipc-2023/recharging-robots-optimal \
        ipc-2023/ricochet-robots-optimal ipc-2023/rubiks-cube-optimal \
        ipc-2023/slitherlink-optimal
        opt-2026 ipc-2026n/onlycraft-opt-numeric-2026 ipc-2026n/rainbowttles-opt-numeric-2026 \
        ipc-2026n/sailing-wind-opt-numeric-2026
        opt-2026-full ipc-2026n/2048-numeric-2026 ipc-2026n/coins-numeric-2026 \
        ipc-2026n/expedition-numeric-2026 ipc-2026n/factory-robot-numeric-2026 \
        ipc-2026n/forestfire-numeric-2026 ipc-2026n/gear-car-numeric-2026 \
        ipc-2026n/line-exchange-snp-numeric-2026 ipc-2026n/onlycraft-opt-numeric-2026 \
        ipc-2026n/petri-net-numeric-2026 ipc-2026n/rainbowttles-opt-numeric-2026 \
        ipc-2026n/sailing-wind-opt-numeric-2026 ipc-2026n/settlers-snp-numeric-2026 \
        ipc-2026n/ztalloc-sum-numeric-2026
        prop-2006 ipc-2006/openstacks-propositional ipc-2006/openstacks-propositional-strips \
        ipc-2006/pathways-propositional ipc-2006/pathways-propositional-strips \
        ipc-2006/pipesworld-propositional ipc-2006/pipesworld-propositional-strips \
        ipc-2006/rovers-propositional ipc-2006/rovers-propositional-strips \
        ipc-2006/storage-propositional ipc-2006/tpp-propositional \
        ipc-2006/tpp-propositional-strips ipc-2006/trucks-propositional \
        ipc-2006/trucks-propositional-strips
        qual-pref-2006 ipc-2006/openstacks-preferences-qualitative \
        ipc-2006/rovers-preferences-qualitative ipc-2006/storage-preferences-qualitative \
        ipc-2006/tpp-preferences-qualitative ipc-2006/trucks-preferences-qualitative
        sat-2018 ipc-2018/agricola-sequential-satisficing \
        ipc-2018/caldera-sequential-satisficing ipc-2018/caldera-split-sequential-satisficing \
        ipc-2018/data-network-sequential-satisficing ipc-2018/flashfill-sequential-satisficing \
        ipc-2018/nurikabe-sequential-satisficing \
        ipc-2018/organic-synthesis-sequential-satisficing \
        ipc-2018/organic-synthesis-split-sequential-satisficing \
        ipc-2018/settlers-sequential-satisficing ipc-2018/snake-sequential-satisficing \
        ipc-2018/spider-sequential-satisficing ipc-2018/termes-sequential-satisficing
        sat-2023 ipc-2023/folding-satisficing ipc-2023/labyrinth-satisficing \
        ipc-2023/quantum-layout-satisficing ipc-2023/recharging-robots-satisficing \
        ipc-2023/ricochet-robots-satisficing ipc-2023/rubiks-cube-satisficing \
        ipc-2023/slitherlink-satisficing
        seq-agile-2014 ipc-2014/barman-sequential-agile ipc-2014/cave-diving-sequential-agile \
        ipc-2014/child-snack-sequential-agile ipc-2014/city-car-sequential-agile \
        ipc-2014/floor-tile-sequential-agile ipc-2014/genome-edit-distances-sequential-agile \
        ipc-2014/hiking-sequential-agile ipc-2014/maintenance-sequential-agile \
        ipc-2014/openstacks-sequential-agile ipc-2014/parking-sequential-agile \
        ipc-2014/tetris-sequential-agile ipc-2014/thoughtful-sequential-agile \
        ipc-2014/transport-sequential-agile ipc-2014/visit-all-sequential-agile
        seq-mco ipc-2011/barman-sequential-multi-core ipc-2011/elevator-sequential-multi-core \
        ipc-2011/floor-tile-sequential-multi-core ipc-2011/no-mystery-sequential-multi-core \
        ipc-2011/openstacks-sequential-multi-core ipc-2011/parc-printer-sequential-multi-core \
        ipc-2011/parking-sequential-multi-core ipc-2011/peg-solitaire-sequential-multi-core \
        ipc-2011/scanalyzer-3d-sequential-multi-core ipc-2011/sokoban-sequential-multi-core \
        ipc-2011/tidybot-sequential-multi-core ipc-2011/transport-sequential-multi-core \
        ipc-2011/visit-all-sequential-multi-core ipc-2011/woodworking-sequential-multi-core
        seq-mco-2014 ipc-2014/barman-sequential-multi-core \
        ipc-2014/cave-diving-sequential-multi-core ipc-2014/child-snack-sequential-multi-core \
        ipc-2014/city-car-sequential-multi-core ipc-2014/floor-tile-sequential-multi-core \
        ipc-2014/genome-edit-distances-sequential-multi-core \
        ipc-2014/hiking-sequential-multi-core ipc-2014/maintenance-sequential-multi-core \
        ipc-2014/openstacks-sequential-multi-core ipc-2014/parking-sequential-multi-core \
        ipc-2014/tetris-sequential-multi-core ipc-2014/thoughtful-sequential-multi-core \
        ipc-2014/transport-sequential-multi-core ipc-2014/visit-all-sequential-multi-core
        seq-opt ipc-2008/elevator-sequential-optimal-strips \
        ipc-2008/openstacks-sequential-optimal-adl \
        ipc-2008/openstacks-sequential-optimal-strips \
        ipc-2008/parc-printer-sequential-optimal-strips \
        ipc-2008/peg-solitaire-sequential-optimal-strips \
        ipc-2008/scanalyzer-3d-sequential-optimal-strips \
        ipc-2008/sokoban-sequential-optimal-strips \
        ipc-2008/transport-sequential-optimal-strips \
        ipc-2008/woodworking-sequential-optimal-strips ipc-2011/barman-sequential-optimal \
        ipc-2011/elevator-sequential-optimal ipc-2011/floor-tile-sequential-optimal \
        ipc-2011/no-mystery-sequential-optimal ipc-2011/openstacks-sequential-optimal \
        ipc-2011/parc-printer-sequential-optimal ipc-2011/parking-sequential-optimal \
        ipc-2011/peg-solitaire-sequential-optimal ipc-2011/scanalyzer-3d-sequential-optimal \
        ipc-2011/sokoban-sequential-optimal ipc-2011/tidybot-sequential-optimal \
        ipc-2011/transport-sequential-optimal ipc-2011/visit-all-sequential-optimal \
        ipc-2011/woodworking-sequential-optimal
        seq-opt-2014 ipc-2014/barman-sequential-optimal \
        ipc-2014/cave-diving-sequential-optimal ipc-2014/child-snack-sequential-optimal \
        ipc-2014/city-car-sequential-optimal ipc-2014/floor-tile-sequential-optimal \
        ipc-2014/genome-edit-distances-sequential-optimal ipc-2014/hiking-sequential-optimal \
        ipc-2014/maintenance-sequential-optimal ipc-2014/openstacks-sequential-optimal \
        ipc-2014/parking-sequential-optimal ipc-2014/tetris-sequential-optimal \
        ipc-2014/tidybot-sequential-optimal ipc-2014/transport-sequential-optimal \
        ipc-2014/visit-all-sequential-optimal
        seq-sat ipc-2008/cyber-security-sequential-satisficing-strips \
        ipc-2008/elevator-sequential-satisficing-strips \
        ipc-2008/openstacks-sequential-satisficing-adl \
        ipc-2008/openstacks-sequential-satisficing-strips \
        ipc-2008/parc-printer-sequential-satisficing-strips \
        ipc-2008/peg-solitaire-sequential-satisficing-strips \
        ipc-2008/scanalyzer-3d-sequential-satisficing-strips \
        ipc-2008/sokoban-sequential-satisficing-strips \
        ipc-2008/transport-sequential-satisficing-strips \
        ipc-2008/woodworking-sequential-satisficing-strips \
        ipc-2011/barman-sequential-satisficing ipc-2011/elevator-sequential-satisficing \
        ipc-2011/floor-tile-sequential-satisficing ipc-2011/no-mystery-sequential-satisficing \
        ipc-2011/openstacks-sequential-satisficing \
        ipc-2011/parc-printer-sequential-satisficing ipc-2011/parking-sequential-satisficing \
        ipc-2011/peg-solitaire-sequential-satisficing \
        ipc-2011/scanalyzer-3d-sequential-satisficing ipc-2011/sokoban-sequential-satisficing \
        ipc-2011/tidybot-sequential-satisficing ipc-2011/transport-sequential-satisficing \
        ipc-2011/visit-all-sequential-satisficing ipc-2011/woodworking-sequential-satisficing
        seq-sat-2014 ipc-2014/barman-sequential-satisficing \
        ipc-2014/cave-diving-sequential-satisficing \
        ipc-2014/child-snack-sequential-satisficing ipc-2014/city-car-sequential-satisficing \
        ipc-2014/floor-tile-sequential-satisficing \
        ipc-2014/genome-edit-distances-sequential-satisficing \
        ipc-2014/hiking-sequential-satisficing ipc-2014/maintenance-sequential-satisficing \
        ipc-2014/openstacks-sequential-satisficing ipc-2014/parking-sequential-satisficing \
        ipc-2014/tetris-sequential-satisficing ipc-2014/thoughtful-sequential-satisficing \
        ipc-2014/transport-sequential-satisficing ipc-2014/visit-all-sequential-satisficing
        simple-pref-2006 ipc-2006/openstacks-preferences-simple \
        ipc-2006/pathways-preferences-simple ipc-2006/rovers-metric-preferences-simple \
        ipc-2006/storage-preferences-simple ipc-2006/tpp-preferences-simple \
        ipc-2006/trucks-preferences-simple
        tempo-sat ipc-2008/crew-planning-temporal-satisficing-strips \
        ipc-2008/elevator-temporal-satisficing-numeric-fluents \
        ipc-2008/elevator-temporal-satisficing-strips \
        ipc-2008/model-train-temporal-satisficing-numeric-fluents \
        ipc-2008/openstacks-temporal-satisficing-adl \
        ipc-2008/openstacks-temporal-satisficing-adl-numeric-fluents \
        ipc-2008/openstacks-temporal-satisficing-numeric-fluents \
        ipc-2008/openstacks-temporal-satisficing-strips \
        ipc-2008/parc-printer-temporal-satisficing-strips \
        ipc-2008/peg-solitaire-temporal-satisficing-strips \
        ipc-2008/sokoban-temporal-satisficing-strips \
        ipc-2008/transport-temporal-satisficing-numeric-fluents \
        ipc-2008/woodworking-temporal-satisficing-numeric-fluents \
        ipc-2011/crew-planning-temporal-satisficing ipc-2011/elevator-temporal-satisficing \
        ipc-2011/floor-tile-temporal-satisficing ipc-2011/match-cellar-temporal-satisficing \
        ipc-2011/openstacks-temporal-satisficing ipc-2011/parc-printer-temporal-satisficing \
        ipc-2011/parking-temporal-satisficing ipc-2011/peg-solitaire-temporal-satisficing \
        ipc-2011/sokoban-temporal-satisficing ipc-2011/storage-temporal-satisficing \
        ipc-2011/temporal-machine-shop-temporal-satisficing \
        ipc-2011/turn-and-open-temporal-satisficing
        tempo-sat-2014 ipc-2014/driver-log-temporal-satisficing \
        ipc-2014/floor-tile-temporal-satisficing ipc-2014/map-analyzer-temporal-satisficing \
        ipc-2014/match-cellar-temporal-satisficing ipc-2014/parking-temporal-satisficing \
        ipc-2014/road-traffic-accident-management-temporal-satisficing \
        ipc-2014/satellite-temporal-satisficing ipc-2014/storage-temporal-satisficing \
        ipc-2014/temporal-machine-shop-temporal-satisficing \
        ipc-2014/turn-and-open-temporal-satisficing
        time-2006 ipc-2006/openstacks-time ipc-2006/openstacks-time-strips \
        ipc-2006/storage-time ipc-2006/trucks-time ipc-2006/trucks-time-strips
        ";

    /// A minimal VALID manifest, so a test can introduce exactly one fault and
    /// read exactly one message. `extra` is appended; TOML appends `[[board]]`
    /// tables to the board array wherever they appear, so a test can add boards
    /// after the `[[set]]` and still observe file order.
    fn fixture(extra: &str) -> Manifest {
        let src = format!(
            r#"
schema = 1

[corpus]
root = ".ipc-corpus"
domain_shared = "domain.pddl"
domain_per_instance = "domains/domain-{{first}}.pddl"

[defaults]
timeout_secs = 60
jobs = 2
threads = 1
mode = "auto"
mem_gb = 6.0

[track.seq-sat]
ipcs = ["ipc-2008", "ipc-2011"]
include = "sequential-satisficing"

[[board]]
id = "ipc67-results"
raw = "ipc67-default.jsonl"
md = "ipc67-results.md"
label = "seq-sat"
competition = "ipc67"
budget_secs = 60
track = "seq-sat"
rebaselined_on = ["m5-air"]

[[set]]
name = "cut25"
stage = "benchmarks/air25"
requires_version = "0.25"
boards = ["ipc67-results"]
{extra}
"#
        );
        Manifest::parse(&src, "<fixture>").expect("fixture must parse")
    }

    /// A fault-free board. `extra` adds keys the base does not already set --
    /// TOML rejects a duplicate key outright, so an override is hand-written.
    fn board(id: &str, label: &str, extra: &str) -> String {
        format!(
            "\n[[board]]\nid = \"{id}\"\nraw = \"{id}.jsonl\"\nmd = \"{id}.md\"\n\
             label = \"{label}\"\ncompetition = \"modern\"\nbudget_secs = 60\n\
             track = \"seq-sat\"\n{extra}"
        )
    }

    /// The generated manifest, if this checkout has one. It is produced by
    /// `crucible/tools/gen-manifest.py` rather than tracked, so it can
    /// legitimately be absent; the baked tables above are what keep the
    /// selector proof independent of that.
    fn committed() -> Option<Manifest> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../benchmarks/manifest.toml");
        if !p.exists() {
            return None;
        }
        Some(Manifest::load(&p).expect("the generated manifest must parse"))
    }

    /// Defends the standings renderer's stable tiebreak: boards come back in
    /// FILE order. A `BTreeMap` or a stray `sort()` would put `aaa-after`
    /// first and silently reorder every table that iterates boards.
    #[test]
    fn board_order_is_file_order() {
        let m = fixture(&(board("zzz-last", "zed", "") + &board("aaa-after", "ay", "")));
        let ids: Vec<&str> = m.boards.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["ipc67-results", "zzz-last", "aaa-after"]);
    }

    /// Budgets are TOML integers (`60`, `30`, `300`) read into f64. The budget
    /// is the denominator of the timeout class, so assert the conversion rather
    /// than assume it.
    #[test]
    fn integer_budgets_read_as_f64() {
        let m = fixture(
            "\n[[board]]\nid = \"tier\"\nraw = \"tier.jsonl\"\nmd = \"tier.md\"\n\
             label = \"tiered\"\ncompetition = \"ipc5\"\nbudget_secs = 30\n\
             timeout_secs = 60\ntrack = \"seq-sat\"\n",
        );
        assert_eq!(m.board("tier").unwrap().budget_secs, 30.0);
        assert_eq!(m.board("tier").unwrap().timeout_secs, Some(60));
        assert_eq!(m.board("ipc67-results").unwrap().budget_secs, 60.0);
    }

    /// The single naming exception in the whole system, as data. Two exceptions
    /// is a system with no rule; zero is a manifest that lost the one board
    /// whose scoreboard is not named after its raw.
    #[test]
    fn exactly_one_board_may_rename_itself() {
        assert!(fixture("").errors().is_empty());

        let m = fixture(
            "\n[[board]]\nid = \"renamed\"\nraw = \"other-name.jsonl\"\nmd = \"renamed.md\"\n\
             label = \"lbl\"\ncompetition = \"modern\"\nbudget_secs = 60\ntrack = \"seq-sat\"\n",
        );
        let errs = m.errors();
        assert!(
            errs.iter().any(|e| {
                e.contains("exactly one board")
                    && e.contains("renamed")
                    && e.contains(NAMING_EXCEPTION_ID)
            }),
            "{errs:?}"
        );
    }

    /// `MD_FOR` as a rule instead of a lookup table: the scoreboard is named
    /// after the board id, always. A raw whose `.md` sibling is missing is a
    /// sweep still in flight, and a board pointed at the wrong `.md` would read
    /// a finished sibling and publish an unfinished sweep.
    #[test]
    fn scoreboard_name_follows_the_board_id() {
        let m = fixture(
            "\n[[board]]\nid = \"plain\"\nraw = \"plain.jsonl\"\nmd = \"something-else.md\"\n\
             label = \"lbl\"\ncompetition = \"modern\"\nbudget_secs = 60\ntrack = \"seq-sat\"\n",
        );
        let errs = m.errors();
        assert!(
            errs.iter()
                .any(|e| e.contains("`plain`") && e.contains("`plain.md`")),
            "{errs:?}"
        );
    }

    /// The mco wall-clock rule. The competition scores wall time on a fixed box
    /// however many cores a planner burns, so a `--threads N` board runs ONE
    /// instance at a time. The drivers infer it at runtime; the manifest
    /// asserts it, so a board can never be scheduled against the rule by
    /// accident and quietly publish a number measured two-at-a-time.
    #[test]
    fn multi_threaded_boards_must_declare_one_job() {
        let unset = fixture(&board("mco", "mco t4", "threads = 4\n"));
        assert!(
            unset
                .errors()
                .iter()
                .any(|e| e.contains("`mco`") && e.contains("WALL TIME")),
            "{:?}",
            unset.errors()
        );

        let two = fixture(&board("mco", "mco t4", "threads = 4\njobs = 2\n"));
        assert!(two.errors().iter().any(|e| e.contains("jobs = 2")));

        let good = fixture(&board("mco", "mco t4", "threads = 4\njobs = 1\n"));
        assert!(good.errors().is_empty(), "{:?}", good.errors());
    }

    /// And the rule resolves through `[defaults]`, because `None` on a board
    /// means the manifest's default. A board that inherits a multi-threaded
    /// default is still a multi-threaded board; reading unset `threads` as a
    /// hardcoded 1 would let every one of them post a wall-clock number
    /// measured two-at-a-time with no message anywhere.
    #[test]
    fn the_mco_rule_reads_the_manifest_defaults() {
        let src = format!(
            "schema = 1\n[corpus]\nroot = \".\"\ndomain_shared = \"d\"\n\
             domain_per_instance = \"p\"\n[defaults]\ntimeout_secs = 60\njobs = 2\n\
             threads = 4\nmode = \"auto\"\nmem_gb = 6.0\n[track.seq-sat]\n\
             ipcs = [\"ipc-2011\"]\ninclude = \"sequential-satisficing\"\n{}{}",
            board("inherits", "inherited", ""),
            board("declares", "declared", "jobs = 1\n"),
        );
        let m = Manifest::parse(&src, "<fixture>").expect("must parse");
        let errs = m.errors();
        assert!(
            errs.iter()
                .any(|e| e.contains("`inherits`") && e.contains("burns 4 threads")),
            "{errs:?}"
        );
        assert!(
            !errs.iter().any(|e| e.contains("`declares`")),
            "a board that declares jobs = 1 obeys the rule: {errs:?}"
        );
    }

    /// A tier move in flight is LEGITIMATE and must be visible. `ipc5-time` and
    /// `ipc5-metric-time` are in exactly this state: swept at 60 s, still
    /// scored at 30 s, because a row classifies against the budget it actually
    /// ran under. A validator that failed here would block a correct release.
    #[test]
    fn tier_move_warns_and_never_fails() {
        let m = fixture(&board("tier", "tiered", "timeout_secs = 90\n"));
        assert!(m.errors().is_empty(), "{:?}", m.errors());
        let w = m.warnings();
        assert!(w.iter().all(|l| l.starts_with(WARNING)), "{w:?}");
        assert!(
            w.iter()
                .any(|l| l.contains("`tier`") && l.contains("TIER MOVE")),
            "{w:?}"
        );
    }

    /// A board in no set is `ipc5-complex-pref` before `post-entries25.sh`:
    /// registered in the sweep list, invisible to every driver. A warning, not
    /// an error -- the fix is to schedule it, not to refuse to publish.
    #[test]
    fn a_board_nothing_sweeps_is_warned_about() {
        let m = fixture(&board("orphan", "orphaned", ""));
        assert!(m.errors().is_empty());
        assert!(m
            .warnings()
            .iter()
            .any(|w| w.contains("`orphan`") && w.contains("nothing sweeps it")));
    }

    /// Every problem, not the first. A manifest with two faults that reports
    /// one gets fixed twice, and each round trip costs a multi-hour sweep.
    #[test]
    fn validate_reports_every_problem() {
        let m = fixture(
            "\n[[board]]\nid = \"lost\"\nraw = \"lost.jsonl\"\nmd = \"lost.md\"\n\
             label = \"lost\"\ncompetition = \"modern\"\nbudget_secs = 60\n\
             track = \"no-such-track\"\n\
             \n[[set]]\nname = \"ghost\"\nstage = \"x\"\nboards = [\"nobody\"]\n",
        );
        let errs = m.errors();
        assert!(errs.iter().any(|e| e.contains("no-such-track")), "{errs:?}");
        assert!(errs.iter().any(|e| e.contains("`nobody`")), "{errs:?}");
        assert_eq!(errs.len(), 2, "{errs:?}");
    }

    /// Ids, raws and labels are join keys -- to the sweep queue, to the raw on
    /// disk, and to the published standings row. Two boards sharing any of the
    /// three is two boards sharing one published number.
    #[test]
    fn duplicate_join_keys_are_named() {
        let m = fixture(&(board("twin", "one", "") + &board("twin", "one", "")));
        let errs = m.errors();
        assert!(
            errs.iter().any(|e| e.contains("duplicate board id `twin`")),
            "{errs:?}"
        );
        assert!(
            errs.iter().any(|e| e.contains("duplicate board raw")),
            "{errs:?}"
        );
        assert!(
            errs.iter()
                .any(|e| e.contains("duplicate board label `one`")),
            "{errs:?}"
        );
    }

    /// `AIR_REBASELINED`, as data. The three answers are distinct: measured on
    /// this box, declared but measured elsewhere, and no board at all -- the
    /// cloud-era ghost whose numbers live in git history, which is NOT a board
    /// we never measured.
    #[test]
    fn rebaselined_on_separates_the_absences() {
        let m = fixture(&board("cloudy", "cloud-era", ""));
        assert!(m.rebaselined_on("seq-sat", "m5-air"));
        assert!(!m.rebaselined_on("seq-sat", "cloud-4core"));
        assert!(!m.rebaselined_on("cloud-era", "m5-air"));
        assert!(!m.rebaselined_on("a label no board carries", "m5-air"));
    }

    /// `PROOF_TRACKS`, as data. Reading a proof board's 45% against a
    /// satisficing board's 45% is the whole reason the flag exists.
    #[test]
    fn proof_track_is_a_board_property() {
        let m = fixture(&board("proof", "opt board", "proof_track = true\n"));
        assert!(m.is_proof_track("opt board"));
        assert!(!m.is_proof_track("seq-sat"));
        assert!(!m.is_proof_track("no such label"));
    }

    /// A key the loader does not know is refused, not ignored. `proof_tracks =
    /// true`, one letter off, would otherwise turn a proof board into a
    /// satisficing one with no message anywhere.
    #[test]
    fn an_unknown_key_is_a_hard_error() {
        let src = format!(
            "schema = 1\n[corpus]\nroot = \".\"\ndomain_shared = \"d\"\n\
             domain_per_instance = \"p\"\n[defaults]\ntimeout_secs = 60\njobs = 2\n\
             threads = 1\nmode = \"auto\"\nmem_gb = 6.0\n{}",
            board("b", "l", "proof_tracks = true\n")
        );
        let e = Manifest::parse(&src, "<fixture>").unwrap_err();
        assert!(format!("{e}").contains("proof_tracks"), "{e}");
    }

    /// The three constructs the selector language HAS, including the group's
    /// backtracking -- greedy first, then empty, exactly as Python's `?`.
    #[test]
    fn the_selector_language_matches_what_it_claims() {
        let p = Pattern::parse("propositional(-strips)?$").unwrap();
        assert!(p.search("tpp-propositional"));
        assert!(p.search("tpp-propositional-strips"));
        assert!(!p.search("tpp-propositional-strips-extra"));
        assert!(!p.search("propositional-grounded"));

        // The group BACKTRACKS, not just "take it if it is there". No manifest
        // pattern puts a literal after the group today, so the corpus proof
        // cannot see the difference -- pin it here, or a later simplification
        // to greedy-only passes every test and is silently a different engine.
        let b = Pattern::parse("a(bc)?bc$").unwrap();
        assert!(b.search("abc"));
        assert!(b.search("abcbc"));

        // Unanchored: a bare literal is a substring test anywhere in the name.
        // `constraints-2006` relies on it to reach the timed-initial-literals
        // variant, whose name continues past the word.
        let c = Pattern::parse("constraints").unwrap();
        assert!(c.search("trucks-time-constraints-timed-initial-literals"));
        assert!(!c.search("trucks-time"));
    }

    /// The lookbehind that became include/exclude. Dropping the exclude pulls
    /// all six `metric-time` variants into the `time` board and grows its
    /// denominator by six, with no error anywhere.
    #[test]
    fn include_minus_exclude_reproduces_the_lookbehind() {
        let t = TrackSpec {
            ipcs: vec!["ipc-2006".into()],
            include: "-time(-strips)?$".into(),
            exclude: Some("metric-time(-strips)?$".into()),
        };
        let s = t.selector().unwrap();
        assert!(s.is_match("openstacks-time"));
        assert!(s.is_match("openstacks-time-strips"));
        assert!(!s.is_match("openstacks-metric-time"));
        assert!(!s.is_match("openstacks-metric-time-strips"));
        // And the `$` is what keeps the constraints variants off the time board.
        assert!(!s.is_match("storage-time-constraints"));
    }

    /// Python's non-MULTILINE `$` also matches just before a final newline.
    /// Unreachable from `readdir`, implemented anyway: an unreachable
    /// divergence is a divergence nobody has reached yet.
    #[test]
    fn dollar_matches_before_one_trailing_newline() {
        let p = Pattern::parse("-agile$").unwrap();
        assert!(p.search("folding-agile"));
        assert!(p.search("folding-agile\n"));
        assert!(!p.search("folding-agile\n\n"));
        assert!(!p.search("folding-agile-x"));
    }

    /// Everything the matcher does NOT have fails loudly. A selector that
    /// silently picks one variant too many or too few does not error -- it
    /// changes a board's denominator, and a wrong denominator is a wrong
    /// published number.
    #[test]
    fn unsupported_constructs_are_refused_not_approximated() {
        assert_eq!(
            Pattern::parse("(agl|sat|opt)"),
            Err(PatternError::Unsupported('|'))
        );
        assert_eq!(Pattern::parse("p.*q"), Err(PatternError::Unsupported('.')));
        assert_eq!(Pattern::parse("^seq"), Err(PatternError::Unsupported('^')));
        assert_eq!(Pattern::parse("a[bc]"), Err(PatternError::Unsupported('[')));
        assert_eq!(Pattern::parse("a\\d"), Err(PatternError::Unsupported('\\')));
        assert_eq!(Pattern::parse("a$b"), Err(PatternError::MisplacedAnchor));
        assert_eq!(Pattern::parse("(ab"), Err(PatternError::UnterminatedGroup));
        // Inside a group, `(` and `$` have no handling at all -- reading them
        // as literals would be a pattern that compiles and means something
        // else than the Python it came from.
        assert_eq!(Pattern::parse("(a$)?"), Err(PatternError::Unsupported('$')));
        assert_eq!(
            Pattern::parse("(a(b)?)?"),
            Err(PatternError::Unsupported('('))
        );
        assert_eq!(
            Pattern::parse("(ab)"),
            Err(PatternError::NonOptionalGroup("ab".into()))
        );
        assert_eq!(Pattern::parse(""), Err(PatternError::Empty));
        assert_eq!(Pattern::parse("$"), Err(PatternError::Empty));

        // And an unusable pattern surfaces through validate(), so one call is
        // the whole gate rather than a compile step somebody can forget.
        let m = fixture("\n[track.bad]\nipcs = [\"ipc-2011\"]\ninclude = \"(a|b)\"\n");
        assert!(
            m.errors()
                .iter()
                .any(|e| e.contains("track `bad` selector") && e.contains('|')),
            "{:?}",
            m.errors()
        );
    }

    /// Split a baked table into records: one per line, first word the key.
    fn table(src: &str) -> BTreeMap<&str, Vec<&str>> {
        src.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| {
                let mut it = l.split_whitespace();
                let key = it.next().expect("every record starts with its key");
                (key, it.collect::<Vec<&str>>())
            })
            .collect()
    }

    /// One [`SELECTORS`] record, unpacked: include, exclude (`-` for none),
    /// then the ipcs.
    fn baked_selector(fields: &[&str]) -> TrackSpec {
        TrackSpec {
            ipcs: fields[2..].iter().map(|s| (*s).to_string()).collect(),
            include: fields[0].to_string(),
            exclude: (fields[1] != "-").then(|| fields[1].to_string()),
        }
    }

    /// THE equivalence proof. For all 26 tracks, over all 292 variant
    /// directories in the vendored corpus, this matcher selects exactly the
    /// names `python3 benchmarks/ipc67.py --track T --list` selected. A
    /// selector that picks one variant too many or too few raises no error
    /// anywhere -- it just changes a board's denominator.
    #[test]
    fn every_track_selects_exactly_what_the_python_regex_selected() {
        let corpus = table(CORPUS);
        let expected = table(EXPECTED);
        let selectors = table(SELECTORS);
        assert_eq!(selectors.len(), 26, "the manifest declares 26 tracks");
        assert_eq!(
            corpus.values().map(Vec::len).sum::<usize>(),
            292,
            "the same 292 variant dirs gen-manifest.py proved the lookbehinds over"
        );

        for (name, fields) in &selectors {
            let spec = baked_selector(fields);
            let sel = spec
                .selector()
                .unwrap_or_else(|e| panic!("track `{name}` selector: {e}"));
            // Same iteration order as ipc67.py's `variants()`: the ipcs in the
            // order the track declares them, names sorted within each. Python
            // sorts by codepoint and this table was written in that order;
            // Rust's bytewise `str` order agrees because UTF-8 preserves it.
            let mut got: Vec<String> = Vec::new();
            for ipc in &spec.ipcs {
                for v in corpus.get(ipc.as_str()).map(Vec::as_slice).unwrap_or(&[]) {
                    if sel.is_match(v) {
                        got.push(format!("{ipc}/{v}"));
                    }
                }
            }
            assert_eq!(got, expected[name], "track `{name}` selected wrongly");
            assert!(!got.is_empty(), "track `{name}` selected nothing");
        }
    }

    /// The baked selector table is a transcription, and a transcription that
    /// can drift is a sixth registry. Where the generated manifest exists,
    /// prove the table still says exactly what the file says.
    #[test]
    fn the_baked_selector_table_has_not_drifted() {
        let Some(m) = committed() else {
            return;
        };
        let selectors = table(SELECTORS);
        assert_eq!(m.tracks.len(), selectors.len());
        for (name, fields) in &selectors {
            let t = m
                .track(name)
                .unwrap_or_else(|| panic!("manifest lost track `{name}`"));
            assert_eq!(*t, baked_selector(fields), "track `{name}` drifted");
        }
    }

    /// The generated manifest itself: no errors, and no warnings. The one
    /// warning this project ever carried -- the `ipc5-time` /
    /// `ipc5-metric-time` tier move, 30 s scored against a 60 s wall -- landed
    /// at the 0.25 promote, so both boards are scored at the wall they run
    /// under again. A warning here now is a board that lost its schedule, or
    /// a tier move nobody recorded.
    #[test]
    fn the_generated_manifest_is_clean() {
        let Some(m) = committed() else {
            return;
        };
        assert_eq!(m.schema, SCHEMA);
        assert!(m.errors().is_empty(), "{:?}", m.errors());
        assert!(m.warnings().is_empty(), "{:?}", m.warnings());

        // The five consolidated registries, spot-checked through the API the
        // renderer will actually use.
        assert_eq!(
            m.board_by_raw(NAMING_EXCEPTION_RAW).unwrap().md,
            "ipc67-results.md"
        );
        assert_eq!(m.board_by_label("seq-mco t4").unwrap().threads, Some(4));
        assert_eq!(m.board_by_label("seq-mco t4").unwrap().jobs, Some(1));
        assert!(m.is_proof_track("2026 numeric-opt FULL"));
        assert!(!m.is_proof_track("2023 numeric"));
        assert!(m.rebaselined_on("propositional", "m5-air"));
        assert!(!m.rebaselined_on("propositional", "cloud-4core"));
        assert_eq!(
            m.set("post-entries25").unwrap().boards,
            ["ipc5-complex-pref"]
        );
    }
}
