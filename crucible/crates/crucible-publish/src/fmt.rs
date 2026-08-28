//! The formatting primitives every renderer shares: Python's rounding, the
//! percentage, the coverage bar, and the exact glyphs the committed tables are
//! made of. Ported from `benchmarks/standings.py` (`_bar` :578, `_ord` :500,
//! and the literal format strings in `write_summary` :583 and `main` :761).
//!
//! Nothing here decides anything. It exists because the LAST step of the
//! pipeline is where two implementations that agree on every number still ship
//! different bytes, and this project publishes those bytes into a release
//! record. Four ways that happens, every one of them live in this codebase:
//!
//! * **Ties.** Python's `round` is half-to-EVEN; Rust's `f64::round` is
//!   half-away-from-zero. `_bar` does `int(round(pct / 100 * width))`, so a
//!   board landing on exactly 62.5% at width 20 asks for `round(12.5)` --
//!   Python fills twelve cells, a naive port fills thirteen. No board on the
//!   current table lands on a tie, which is exactly why the golden battery
//!   cannot catch this one. It gets its own test below instead.
//!
//! * **Reaching a tie that was never there.** The obvious way to write
//!   `py_round` -- scale by `10^n`, `round_ties_even`, scale back -- is not
//!   Python's algorithm and is not equivalent to it. Multiplying by a power of
//!   ten manufactures ties the decimal value does not have and destroys ties it
//!   does: measured against CPython over 1,106 cases it disagreed on 181, among
//!   them `round(2.675, 2)`, where it answers 2.68 and Python answers 2.67.
//!   `py_round` therefore does what CPython does -- format to `ndigits` places,
//!   parse the string back -- which is exact, because Rust's `{:.N}` and
//!   CPython's `_Py_dg_dtoa` are the same correctly-rounded, half-to-even
//!   decimal conversion.
//!
//! * **Argument order.** Every percentage in the Python is `100.0 * s / n`,
//!   never `s / n * 100.0`. These are different doubles: at 23/40 the first is
//!   exactly 57.5 and renders "58%", the second is 57.49999999999999 and
//!   renders "57%". `pct` fixes the order so no caller can pick the wrong one.
//!
//! * **Glyphs.** The proof-track mark is TWO codepoints (U+2696 SCALES plus
//!   U+FE0F VARIATION SELECTOR-16); a bare U+2696 is a silent one-byte diff
//!   against every committed table. The minus sign on a negative delta is
//!   U+2212, not an ASCII hyphen. Both are named consts here so nobody
//!   hand-types them twice.

/// The exact characters the committed tables are made of.
///
/// Every one of these was read back out of `STANDINGS.md`,
/// `benchmarks/ipc-standings.md` and `README.md` rather than typed from
/// memory. A glyph is the one kind of drift that survives every numeric check,
/// so they live as named consts with their codepoints written down.
pub mod glyph {
    /// U+2588 FULL BLOCK -- one filled cell of the coverage bar.
    pub const BAR_FULL: &str = "\u{2588}";

    /// U+2591 LIGHT SHADE -- one empty cell of the coverage bar. Note it is
    /// LIGHT shade, not the medium (U+2592) or dark (U+2593) neighbours that
    /// look near-identical in most editors.
    pub const BAR_EMPTY: &str = "\u{2591}";

    /// U+2014 EM DASH -- "no data held". The `vs field` cell when no cohort is
    /// known, the `entered`/`coverage`/`quality` cells of an absent board, and
    /// the lead of `— *baseline*` / `— *new*` in the delta column.
    pub const EM_DASH: &str = "\u{2014}";

    /// U+2212 MINUS SIGN -- the sign on a NEGATIVE delta (`_delta` :758). Not
    /// an ASCII hyphen and not an en dash: the Python pairs a plain ASCII `+`
    /// with this typographic minus, which looks like an inconsistency and is
    /// not one. Emitting `-` here diffs against every committed table that has
    /// ever shown a regression.
    pub const MINUS: &str = "\u{2212}";

    /// U+00B7 MIDDLE DOT -- the joiner. Used as `" · "` between the per-IPC
    /// halves of a split field cell, and as `"`{name}` · "` in the pending and
    /// cloud-era board lists.
    pub const MIDDOT: &str = "\u{00b7}";

    /// U+2265 GREATER-THAN OR EQUAL TO -- the rank-floor mark in `_placement`.
    /// A cohort that KNOWS more entrants sit ahead than it lists says `≥` here
    /// rather than pretending to a strict rank.
    pub const GE: &str = "\u{2265}";

    /// U+2696 SCALES followed by U+FE0F VARIATION SELECTOR-16 -- the
    /// proof-track mark.
    ///
    /// TWO codepoints, five UTF-8 bytes (`e2 9a 96 ef b8 8f`). The selector is
    /// what asks for the emoji presentation rather than the text one, and it is
    /// present in all four occurrences in `standings.py` and every occurrence
    /// in the committed tables. Dropping it renders almost identically and
    /// diffs on every proof row, which is the definition of a silent diff --
    /// hence the dedicated test.
    pub const SCALES: &str = "\u{2696}\u{fe0f}";

    /// U+00D7 MULTIPLICATION SIGN -- "official budgets are ~30× ours" in the
    /// how-to-read prose.
    pub const TIMES: &str = "\u{00d7}";

    /// U+2260 NOT EQUAL TO -- "coverage ≠ IPC's quality-weighted scoring" in
    /// the how-to-read prose.
    pub const NE: &str = "\u{2260}";

    /// U+2192 RIGHTWARDS ARROW -- "Full standings → `STANDINGS.md`" in the
    /// generated README block.
    pub const ARROW: &str = "\u{2192}";
}

/// The bar width `_bar` defaults to, and the width `STANDINGS.md` is rendered
/// at. `_patch_readme` overrides it to 16 for the front-page block, which is
/// why `bar` takes the width rather than assuming it.
pub const BAR_WIDTH: usize = 20;

/// Python's `round(x, ndigits)`: round-half-to-EVEN on the decimal value.
///
/// This is CPython's algorithm, not an approximation of it. `_Py_double_round`
/// formats `x` to `ndigits` places with `_Py_dg_dtoa` in mode 3 and parses the
/// result back with `_Py_dg_strtod`; Rust's `{:.N}` is the same
/// correctly-rounded conversion with the same half-to-even tie rule, so
/// format-and-parse reproduces it exactly. Verified against `python3` over
/// 1,106 values, including every tie the naive scale-by-`10^n` approach gets
/// wrong.
///
/// The two guards bracket the range in which an `ndigits` can still move a
/// double at all. Above +323 the rounding cannot change any double, so `x` comes
/// back untouched; below -323 every finite double has already rounded away, so
/// the answer is a zero carrying `x`'s sign. Non-finite values round to
/// themselves, as they do in Python.
///
/// The low guard is -323, NOT -93. `NDIGITS_MIN` is the one CPython constant it
/// is tempting to copy off a half-remembered macro and get wrong: whatever the
/// macro evaluates to, the short-circuit sits far enough out to be
/// unobservable, and `round` really does keep rounding on the way down.
/// Measured against `python3`: `round(1e300, -94)` is `1e300`, `round(1e300,
/// -300)` is `1e300`, `round(5e93, -94)` is `1e94`, and `round(9e307, -308)` is
/// `1e308` -- every one of which a -93 cut-off answers `0.0` for. The first
/// `ndigits` at which every finite double is necessarily zero is -309 (nothing
/// exceeds `DBL_MAX`, so nothing reaches half of `10^309`), so -323 -- the
/// mirror of the high guard -- is safely past the last observable case and
/// exists only to stop an absurd `ndigits` asking for an absurd digit string.
///
/// One divergence, unreachable from any call site: a negative `ndigits` that
/// rounds `x` up past `DBL_MAX` (`round(1.7e308, -308)`) raises `OverflowError`
/// in Python and yields an infinity here. Nothing in this crate rounds at a
/// negative `ndigits` at all -- `standings.py` calls `round` in exactly one
/// place, `_bar`, with no `ndigits`.
pub fn py_round(x: f64, ndigits: i32) -> f64 {
    // Python: "nans and infinities round to themselves".
    if !x.is_finite() {
        return x;
    }
    if ndigits > 323 {
        return x;
    }
    if ndigits < -323 {
        // Zero with x's sign, exactly as `0.0 * x` gives in Python.
        return 0.0 * x;
    }
    if ndigits >= 0 {
        // The whole point. See the module header.
        return format!("{:.*}", ndigits as usize, x)
            .parse::<f64>()
            .unwrap_or(x);
    }
    round_left_of_point(x, (-ndigits) as u32)
}

/// Python's single-argument `round(x)`: round-half-to-even, no `ndigits`.
///
/// CPython does `round(x)` and then, on the halfway case, `2.0 * round(x / 2.0)`
/// to pull the tie back to even -- which is precisely `round_ties_even`,
/// stabilised in Rust 1.77. This is the one used in anger: `_bar` calls
/// `int(round(...))` on every rendered row.
///
/// Non-finite input saturates rather than panicking (NaN to 0, infinities to
/// the `i64` bounds) where Python would raise. No caller can reach that: the
/// only argument this ever sees is a percentage from [`pct`].
pub fn py_round_i(x: f64) -> i64 {
    x.round_ties_even() as i64
}

/// Round half-to-even at a digit position LEFT of the decimal point, for
/// `py_round`'s negative-`ndigits` branch.
///
/// No ported call site reaches this -- `standings.py` calls `round` in exactly
/// one place, `_bar`, with no `ndigits` at all -- but `py_round` is shared API
/// and the scale-by-`10^n` shortcut is wrong here for the same reason it is
/// wrong on the positive side. So it is done on the digits: `trunc` is exact,
/// formatting an already-integral double is exact, and `fract() != 0.0` is the
/// only thing needed to know whether a tie is a true tie or has a tail under
/// it. Verified against `python3` over the whole range `py_round` now hands it
/// -- `ndigits` from -1 to -329, magnitudes from `5e-324` to `DBL_MAX`, ties
/// and near-ties at every decade -- 5.5M cases, agreeing on the bits of every
/// one.
fn round_left_of_point(x: f64, k: u32) -> f64 {
    let negative = x.is_sign_negative();
    let magnitude = x.abs();
    let integral = magnitude.trunc();
    let has_tail = magnitude.fract() != 0.0;

    let mut digits: Vec<u8> = format!("{integral:.0}").into_bytes();
    let k = k as usize;
    if k >= digits.len() {
        // Rounding at or above the leading digit: pad so there is always a
        // digit to inspect and a (possibly empty) kept prefix.
        let mut padded = vec![b'0'; k + 1 - digits.len()];
        padded.append(&mut digits);
        digits = padded;
    }

    let cut = digits.len() - k;
    let first_dropped = digits[cut] - b'0';
    let rest_nonzero = digits[cut + 1..].iter().any(|c| *c != b'0') || has_tail;
    let kept_last_is_odd = cut > 0 && (digits[cut - 1] - b'0') % 2 == 1;
    // Half-to-even: a 5 with anything under it rounds up; a bare 5 rounds up
    // only when it would otherwise leave an odd digit standing.
    let round_up = first_dropped > 5 || (first_dropped == 5 && (rest_nonzero || kept_last_is_odd));

    let mut kept: Vec<u8> = digits[..cut].to_vec();
    if round_up {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }

    let mut s = String::from_utf8(kept).unwrap_or_default();
    if s.is_empty() {
        s.push('0');
    }
    for _ in 0..k {
        s.push('0');
    }
    let value: f64 = s.parse().unwrap_or(0.0);
    if negative {
        -value
    } else {
        value
    }
}

/// A coverage percentage, in the argument order the Python uses.
///
/// `100.0 * s / n`, multiply BEFORE divide. Not a stylistic preference: float
/// multiplication and division do not commute through each other, and at 23/40
/// this order gives exactly 57.5 (rendering "58%") where `s / n * 100.0` gives
/// 57.49999999999999 (rendering "57%"). Over the boards with denominators under
/// 700, the two orders disagree on the double for 61,212 (solved, total) pairs
/// and on the published percentage for twenty of them.
///
/// `total == 0` yields 0.0. Python would raise `ZeroDivisionError`, but it never
/// gets the chance: `write_summary` drops empty boards with `if not n: continue`
/// before any percentage is taken, so this branch stands in for a row that is
/// not rendered at all.
pub fn pct(solved: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    100.0 * solved as f64 / total as f64
}

/// Python's `"{:.Nf}"`.
///
/// Rust's `{:.N}` and CPython's float formatter are both exact decimal
/// conversions rounding half-to-even, so they agree digit for digit -- verified
/// over 539 values across 0, 1, 2 and 3 places, including negative zero (both
/// keep the sign: `-0.0` at one place is `"-0.0"`) and the infinities (both
/// `"inf"` / `"-inf"`).
///
/// The single exception is NaN, which Rust spells `"NaN"` and Python spells
/// `"nan"`, normalised here. No call site can produce one -- every mean in the
/// port is guarded by `if not ratios: return None` upstream -- but this module's
/// contract is byte-identity with Python's formatter, so the one place they
/// disagree is closed here rather than left as a landmine for a renderer.
pub fn fmt_f(x: f64, places: usize) -> String {
    if x.is_nan() {
        return "nan".to_string();
    }
    format!("{:.*}", places, x)
}

/// The coverage bar: `"█" * filled + "░" * (width - filled)`.
///
/// `filled` is `int(round(pct / 100 * width))` -- divided first, then
/// multiplied, in that order, because the Python does it in that order.
///
/// Out-of-range input is reproduced rather than corrected. Python's `"x" * n`
/// is the empty string for `n <= 0`, so a percentage above 100 renders MORE
/// full cells than the bar is wide and no empty ones, and one below 0 renders
/// more empty cells. `pct` cannot produce either (a board's solved rows are a
/// subset of its rows), which is the point: if an upstream counting bug ever
/// makes solved exceed total, this renders visibly, loudly wrong instead of
/// quietly clamping to a plausible full bar.
///
/// A non-finite percentage is the one deliberate divergence: Python raises
/// `OverflowError` from `int(round(inf))`, while the `i64` cast would saturate
/// and ask for an eighteen-exabyte string. It degenerates to an empty bar
/// instead. Unreachable from `pct`.
pub fn bar(pct: f64, width: usize) -> String {
    let filled = if pct.is_finite() {
        py_round_i(pct / 100.0 * width as f64)
    } else {
        0
    };
    // Python's `"x" * n` is the empty string for n <= 0; `max(0)` before the
    // cast is what reproduces that, and it is also what keeps a negative
    // `filled` from wrapping into an enormous `usize`.
    let full = filled.max(0) as usize;
    let empty = (width as i64).saturating_sub(filled).max(0) as usize;
    glyph::BAR_FULL.repeat(full) + &glyph::BAR_EMPTY.repeat(empty)
}

/// Python's `"{:,}"` -- comma-grouped thousands, as in `3,981/6,366`.
///
/// Counts only, hence `u64`: every call site in the Python passes a summed row
/// count (`tot_s`, `tot_n`, `proofs`), and none can be negative.
pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(bytes.len() + bytes.len() / 3);
    for (i, c) in bytes.iter().enumerate() {
        // A separator goes before every digit that starts a group of three
        // counting from the right, except at the very front.
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*c as char);
    }
    out
}

/// `_ord` :500 -- the ordinal SUFFIX alone, which is how the Python uses it
/// (`f"{approx}{r}{_ord(r)} of {total}"`).
///
/// The Python's teens band is `10 <= n % 100 <= 20`, wider than the textbook
/// 11-13. It is ported verbatim because it is not a bug: every extra value it
/// captures (10, 14 through 20, and their multiples of 100) has a last digit of
/// 0 or 4-9 and would fall to "th" through the dictionary lookup anyway. The
/// two spellings agree on every input, so there is nothing to fix and nothing
/// to risk by "fixing" it.
pub fn ordinal_suffix(n: usize) -> &'static str {
    if (10..=20).contains(&(n % 100)) {
        return "th";
    }
    match n % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

/// The number and its suffix together: `1st`, `2nd`, `3rd`, `4th`, `11th`,
/// `21st`.
///
/// `_placement` prefixes this with `~` or `≥` before writing it, so a caller
/// composes rather than asking for a decorated form.
pub fn ordinal(n: usize) -> String {
    format!("{n}{}", ordinal_suffix(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Defends the half-to-even rule itself: the classic ties, where Rust's own
    /// `f64::round` (half-away-from-zero) gives a different answer for three of
    /// the six.
    #[test]
    fn py_round_breaks_ties_toward_even() {
        assert_eq!(py_round_i(0.5), 0);
        assert_eq!(py_round_i(1.5), 2);
        assert_eq!(py_round_i(2.5), 2);
        assert_eq!(py_round_i(3.5), 4);
        assert_eq!(py_round_i(-0.5), 0);
        assert_eq!(py_round_i(-1.5), -2);
        assert_eq!(py_round_i(-2.5), -2);
        // What a naive port would have produced, for the record.
        assert_eq!(0.5f64.round(), 1.0);
        assert_eq!(2.5f64.round(), 3.0);
    }

    /// Defends `py_round` against a table taken from real `python3` output.
    /// Every row was produced by `python3 -c "print(repr(round(x, n)))"`; the
    /// comparison is on bits so that `-0.0` cannot pass as `0.0`.
    #[test]
    fn py_round_matches_cpython_on_a_verified_table() {
        // (x, ndigits, python3 round(x, ndigits))
        let table: &[(f64, i32, f64)] = &[
            (0.5, 0, 0.0),
            (1.5, 0, 2.0),
            (2.5, 0, 2.0),
            (3.5, 0, 4.0),
            (-0.5, 0, -0.0),
            (-1.5, 0, -2.0),
            (-2.5, 0, -2.0),
            (0.125, 2, 0.12),
            (0.135, 2, 0.14),
            (0.145, 2, 0.14),
            (2.675, 2, 2.67),
            (2.665, 2, 2.67),
            (1.005, 2, 1.0),
            (8.835, 2, 8.84),
            (0.15, 1, 0.1),
            (0.25, 1, 0.2),
            (0.35, 1, 0.3),
            (0.45, 1, 0.5),
            (12.35, 1, 12.3),
            (12.5, 0, 12.0),
            (62.5, 0, 62.0),
            (18.5, 0, 18.0),
            (0.285, 2, 0.28),
            (1871.55, 1, 1871.5),
            (100.0, 0, 100.0),
            (91.85185185185185, 0, 92.0),
            (62.535344014389, 0, 63.0),
            (25.0, -1, 20.0),
            (15.0, -1, 20.0),
            (250.0, -2, 200.0),
        ];
        for &(x, n, want) in table {
            let got = py_round(x, n);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "py_round({x:?}, {n}) = {got:?}, python3 says {want:?}"
            );
        }
    }

    /// Defends the LOW `ndigits` guard, which is -323 and NOT -93.
    ///
    /// `NDIGITS_MIN` is the CPython constant it is tempting to copy off a
    /// half-remembered macro: `-(int)((DBL_MAX_10_EXP + 1) * 0.30103)` reads
    /// like -93, and a -93 cut-off answers `0.0` for every row in this table.
    /// CPython's real short-circuit is far enough out to be unobservable, so
    /// `round` keeps rounding all the way down to -308, where the last finite
    /// double stops reaching half of `10^-ndigits`. Every expectation is
    /// `python3 -c "print(repr(round(x, n)))"`, compared on bits so a signed
    /// zero cannot pass for the other one.
    #[test]
    fn py_round_keeps_rounding_past_the_naive_low_guard() {
        let table: &[(f64, i32, f64)] = &[
            (1e300, -94, 1e300),
            (1e300, -200, 1e300),
            (1e300, -300, 1e300),
            (1e300, -301, 0.0),
            (5e93, -94, 1e94),
            (4.9e93, -94, 0.0),
            (9e307, -308, 1e308),
            (9e307, -309, 0.0),
            (-1e300, -94, -1e300),
            (-5e93, -94, -1e94),
            // Past the last observable ndigits: a zero carrying x's sign,
            // which is what both the guard and CPython return.
            (-1e300, -323, -0.0),
            (1e300, -400, 0.0),
            (-1e300, -400, -0.0),
        ];
        for &(x, n, want) in table {
            let got = py_round(x, n);
            assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "py_round({x:?}, {n}) = {got:?}, python3 says {want:?}"
            );
        }
    }

    /// Defends against the scale-by-`10^n` shortcut specifically. Each of these
    /// is a value where `(x * 10^n).round_ties_even() / 10^n` disagrees with
    /// CPython, because the scaling either invented a tie or destroyed one.
    #[test]
    fn py_round_does_not_scale_by_a_power_of_ten() {
        let cases: &[(f64, i32, f64)] = &[
            (2.675, 2, 2.67),
            (12.35, 1, 12.3),
            (0.15, 1, 0.1),
            (0.35, 1, 0.3),
            (0.45, 1, 0.5),
            (1871.55, 1, 1871.5),
        ];
        for &(x, n, want) in cases {
            assert_eq!(py_round(x, n), want, "py_round({x:?}, {n})");
            // And prove the shortcut really is the thing being defended
            // against, so this test cannot quietly become a tautology.
            let scale = 10f64.powi(n);
            let naive = (x * scale).round_ties_even() / scale;
            assert_ne!(
                naive, want,
                "the naive path is supposed to be wrong at {x:?}"
            );
        }
    }

    /// Defends the tie inside `_bar`. At 62.5% over 20 cells the fill is
    /// exactly 12.5, which Python rounds DOWN to 12. No board on the committed
    /// table lands on a tie, so the goldens can never catch this -- only this
    /// test can.
    #[test]
    fn bar_breaks_its_tie_toward_the_even_cell() {
        assert_eq!(py_round_i(62.5 / 100.0 * 20.0), 12);
        let b = bar(62.5, 20);
        assert_eq!(b.chars().filter(|c| *c == '\u{2588}').count(), 12);
        assert_eq!(b.chars().filter(|c| *c == '\u{2591}').count(), 8);
        // Half-away-from-zero would have filled thirteen.
        assert_eq!((62.5f64 / 100.0 * 20.0).round(), 13.0);
    }

    /// Defends the bar against the committed bytes: four rows copied out of
    /// `STANDINGS.md` at width 20, and two out of the generated README block at
    /// width 16. Coverage counts are the real ones from those files.
    #[test]
    fn bar_reproduces_committed_rows() {
        // STANDINGS.md, width 20.
        assert_eq!(bar(pct(248, 270), 20), "██████████████████░░");
        assert_eq!(bar(pct(28, 120), 20), "█████░░░░░░░░░░░░░░░");
        assert_eq!(bar(pct(287, 550), 20), "██████████░░░░░░░░░░");
        assert_eq!(bar(pct(37, 140), 20), "█████░░░░░░░░░░░░░░░");
        // README.md's generated block, width 16.
        assert_eq!(bar(pct(248, 270), 16), "███████████████░");
        assert_eq!(bar(pct(230, 280), 16), "█████████████░░░");
        // And the headline row's percentage, which shares the same pipeline.
        assert_eq!(fmt_f(pct(3981, 6366), 0), "63");
        assert_eq!(fmt_f(pct(248, 270), 0), "92");
    }

    /// Defends the out-of-range behaviour: Python's `"x" * n` is empty for
    /// `n <= 0`, so an impossible percentage renders loudly rather than
    /// clamping to something plausible -- and nothing panics.
    #[test]
    fn bar_reproduces_python_out_of_range_and_never_panics() {
        // Above 100: more full cells than the width, no empties.
        assert_eq!(
            bar(125.0, 20).chars().filter(|c| *c == '\u{2588}').count(),
            25
        );
        assert_eq!(
            bar(125.0, 20).chars().filter(|c| *c == '\u{2591}').count(),
            0
        );
        // Below 0: no full cells, more empties than the width.
        assert_eq!(
            bar(-25.0, 20).chars().filter(|c| *c == '\u{2588}').count(),
            0
        );
        assert_eq!(
            bar(-25.0, 20).chars().filter(|c| *c == '\u{2591}').count(),
            25
        );
        // The degenerate inputs a renderer must survive.
        assert_eq!(bar(f64::NAN, 20).chars().count(), 20);
        assert_eq!(bar(f64::INFINITY, 20).chars().count(), 20);
        assert_eq!(bar(f64::NEG_INFINITY, 20).chars().count(), 20);
        assert_eq!(bar(0.0, 0), "");
    }

    /// Defends the proof-track mark's SECOND codepoint. `⚖` alone renders
    /// almost identically and diffs against every proof row in every committed
    /// table, which is the classic silent diff this module exists to prevent.
    #[test]
    fn scales_is_two_codepoints_not_one() {
        let cps: Vec<u32> = glyph::SCALES.chars().map(|c| c as u32).collect();
        assert_eq!(
            cps,
            vec![0x2696, 0xFE0F],
            "SCALES must carry the U+FE0F selector"
        );
        assert_eq!(glyph::SCALES.chars().count(), 2);
        assert_eq!(
            glyph::SCALES.as_bytes(),
            &[0xE2, 0x9A, 0x96, 0xEF, 0xB8, 0x8F],
            "the bytes committed in STANDINGS.md and ipc-standings.md"
        );
        // How it is actually written onto a row.
        assert_eq!(format!(" {}", glyph::SCALES), " ⚖️");
    }

    /// Defends every other glyph against its codepoint, so a paste from a
    /// lookalike (U+2592 medium shade for U+2591, an ASCII hyphen or en dash
    /// for U+2212) cannot survive.
    #[test]
    fn glyphs_are_the_committed_codepoints() {
        for (name, s, cp) in [
            ("BAR_FULL", glyph::BAR_FULL, 0x2588u32),
            ("BAR_EMPTY", glyph::BAR_EMPTY, 0x2591),
            ("EM_DASH", glyph::EM_DASH, 0x2014),
            ("MINUS", glyph::MINUS, 0x2212),
            ("MIDDOT", glyph::MIDDOT, 0x00B7),
            ("GE", glyph::GE, 0x2265),
            ("TIMES", glyph::TIMES, 0x00D7),
            ("NE", glyph::NE, 0x2260),
            ("ARROW", glyph::ARROW, 0x2192),
        ] {
            let mut it = s.chars();
            assert_eq!(it.next().map(|c| c as u32), Some(cp), "{name}");
            assert_eq!(it.next(), None, "{name} must be a single codepoint");
        }
        // The delta column's sign pair: an ASCII '+' against a typographic
        // minus. It looks inconsistent and is exactly what `_delta` emits.
        assert_ne!(glyph::MINUS, "-");
        assert_ne!(glyph::MINUS, "\u{2013}"); // not an en dash either
        assert_eq!(
            format!("{}{} pts", glyph::MINUS, fmt_f(1.25, 1)),
            "−1.2 pts"
        );
    }

    /// Defends the argument order in `pct`. 23/40 is the smallest board shape
    /// where the two orders render a DIFFERENT published percentage.
    #[test]
    fn pct_multiplies_before_it_divides() {
        assert_eq!(pct(23, 40), 57.5);
        assert_eq!(fmt_f(pct(23, 40), 0), "58");
        // The other order, and the number it would have published.
        let wrong = (23.0f64 / 40.0) * 100.0;
        assert_eq!(wrong, 57.49999999999999);
        assert_eq!(fmt_f(wrong, 0), "57");
        assert_ne!(pct(23, 40), wrong);
        // A second shape, for the other direction of the same slip.
        assert_ne!(pct(1, 3), (1.0f64 / 3.0) * 100.0);
    }

    /// Defends the empty-board guard. Python never divides by zero here because
    /// `write_summary` drops the board first; this stands in for the row that
    /// is not rendered.
    #[test]
    fn pct_guards_an_empty_board() {
        assert_eq!(pct(0, 0), 0.0);
        assert_eq!(pct(7, 0), 0.0);
        assert_eq!(pct(0, 100), 0.0);
        assert_eq!(pct(100, 100), 100.0);
    }

    /// Defends `fmt_f` against Python's `"{:.Nf}"`, including the negative zero
    /// and infinity cases where they agree and the NaN case where they do not.
    #[test]
    fn fmt_f_agrees_with_python_format() {
        assert_eq!(fmt_f(0.5, 0), "0"); // half to even, like Python
        assert_eq!(fmt_f(1.5, 0), "2");
        assert_eq!(fmt_f(2.5, 0), "2");
        assert_eq!(fmt_f(0.25, 1), "0.2");
        assert_eq!(fmt_f(0.35, 1), "0.3");
        assert_eq!(fmt_f(2.675, 2), "2.67");
        assert_eq!(fmt_f(91.85185185185185, 0), "92");
        assert_eq!(fmt_f(0.9166666666666666, 2), "0.92");
        assert_eq!(fmt_f(-0.0, 1), "-0.0");
        assert_eq!(fmt_f(-0.0, 0), "-0");
        assert_eq!(fmt_f(f64::INFINITY, 2), "inf");
        assert_eq!(fmt_f(f64::NEG_INFINITY, 2), "-inf");
        // The one divergence, normalised to Python's spelling.
        assert_eq!(fmt_f(f64::NAN, 2), "nan");
    }

    /// Defends the comma grouping against Python's `"{:,}"`, at the group
    /// boundaries and on the two headline numbers as committed.
    #[test]
    fn thousands_matches_python_comma_format() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(7), "7");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234), "1,234");
        assert_eq!(thousands(999999), "999,999");
        assert_eq!(thousands(1000000), "1,000,000");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(
            thousands(18446744073709551615),
            "18,446,744,073,709,551,615"
        );
        // STANDINGS.md's headline: "(3,981/6,366)".
        assert_eq!(thousands(3981), "3,981");
        assert_eq!(thousands(6366), "6,366");
        assert_eq!(
            format!("{}/{}", thousands(3981), thousands(6366)),
            "3,981/6,366"
        );
    }

    /// Defends `_ord`, including the teens and the multiples of 100 where the
    /// Python's wider `10..=20` band and the textbook `11..=13` band must give
    /// the same answer.
    #[test]
    fn ordinal_matches_python_ord() {
        let cases: &[(usize, &str)] = &[
            (1, "1st"),
            (2, "2nd"),
            (3, "3rd"),
            (4, "4th"),
            (10, "10th"),
            (11, "11th"),
            (12, "12th"),
            (13, "13th"),
            (14, "14th"),
            (20, "20th"),
            (21, "21st"),
            (22, "22nd"),
            (23, "23rd"),
            (100, "100th"),
            (101, "101st"),
            (110, "110th"),
            (111, "111th"),
            (112, "112th"),
            (113, "113th"),
            (120, "120th"),
            (121, "121st"),
            (0, "0th"),
        ];
        for &(n, want) in cases {
            assert_eq!(ordinal(n), want);
        }
        // The Python's band is wider than 11..=13 and agrees with it anyway --
        // proved here rather than asserted in a comment.
        for n in 0usize..1000 {
            let textbook = if (11..=13).contains(&(n % 100)) {
                "th"
            } else {
                match n % 10 {
                    1 => "st",
                    2 => "nd",
                    3 => "rd",
                    _ => "th",
                }
            };
            assert_eq!(ordinal_suffix(n), textbook, "the two bands disagree at {n}");
        }
        // As `_placement` writes it, with its two prefixes.
        assert_eq!(format!("~{} of 21", ordinal(2)), "~2nd of 21");
        assert_eq!(format!("{}{} of 25", glyph::GE, ordinal(13)), "≥13th of 25");
    }

    /// Defends the non-finite guards, which stand where Python raises. Nothing
    /// here may panic: these are renderers, and a renderer that takes the
    /// process down loses the whole table.
    #[test]
    fn rounding_survives_non_finite_input() {
        assert!(py_round(f64::NAN, 2).is_nan());
        assert_eq!(py_round(f64::INFINITY, 2), f64::INFINITY);
        assert_eq!(py_round(f64::NEG_INFINITY, 2), f64::NEG_INFINITY);
        assert_eq!(py_round_i(f64::NAN), 0);
        // CPython's own short-circuits, at the far ends of ndigits.
        assert_eq!(py_round(1.5, 400), 1.5);
        assert_eq!(py_round(1.5, -200), 0.0);
        assert!(py_round(-1.5, -200).is_sign_negative());
    }
}
