//! The ladder's time table (0.31 Lane A, docs/roadmap-0.31.md).
//!
//! Every bounded rung of the classical ladder takes a SLICE of the wall:
//! EHC a quarter of what remains at its entry, novelty-light a tenth, LAMA a
//! quarter, the novelty driver a half -- each a `FF_*_WALL_FRAC` knob, each
//! hand-set from one or two cells, and three rungs in the record died of
//! their slice rather than of what they do (the 0.17 novelty rung at
//! −51/+7, the 0.28 seed tightening, 0.30's Lane 6 at −26). Fast Downward
//! Stone Soup's lesson (Helmert, Röger & Karpas 2011) is that the slices are
//! DATA: run every rung solo on a training sample, then allocate the wall
//! greedily by coverage per second. `benchmarks/ladder-table.py` does the
//! allocating over the crucible's matrix rows; this module reads what it
//! wrote.
//!
//! The table is a text file, one `rung = seconds` line per rung at a
//! reference `wall = 60` (seconds), `#` comments:
//!
//! ```text
//! # learned 2026-10-xx on matrix31, held-out +N
//! wall = 60
//! ehc = 9
//! novlight = 2
//! lama = 18
//! driver = 24
//! ```
//!
//! `FF_LADDER_TABLE=<path>` loads it once per process; a rung's share is its
//! seconds scaled by the armed wall over the reference wall, capped by what
//! remains at the rung's entry. The precedence is fixed and narrow: an
//! explicit `FF_<RUNG>_WALL_FRAC` still wins (the A/B arm and every probe's
//! habit), the table is next, and the hand-set default fraction of the
//! remaining wall is what every rung had before this module existed --
//! `FF_LADDER_TABLE` unset, or set to `0.30` / `none`, is byte-identical to
//! 0.30. A rung the table does not name keeps its default. Built-in tables
//! (`FF_LADDER_TABLE=<name>`) are added here once a learned one has held on
//! the boards; none has yet.

use std::sync::OnceLock;

/// A loaded table: the reference wall and the seconds per rung.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub wall: f64,
    pub secs: Vec<(String, f64)>,
}

impl Table {
    /// Parse the `key = value` text form. Unknown keys are kept (a rung this
    /// build does not have is a rung another build may); a malformed line
    /// is an error naming its number, never a silently dropped slice.
    pub fn parse(src: &str) -> Result<Table, String> {
        let mut wall = 60.0;
        let mut secs = Vec::new();
        for (i, raw) in src.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line
                .split_once('=')
                .ok_or_else(|| format!("line {}: expected `rung = seconds`", i + 1))?;
            let k = k.trim().to_ascii_lowercase();
            let v: f64 = v
                .trim()
                .parse()
                .map_err(|_| format!("line {}: `{}` is not a number", i + 1, v.trim()))?;
            if !v.is_finite() || v < 0.0 {
                return Err(format!("line {}: {v} is not a non-negative time", i + 1));
            }
            if k == "wall" {
                if v <= 0.0 {
                    return Err(format!(
                        "line {}: the reference wall must be positive",
                        i + 1
                    ));
                }
                wall = v;
            } else {
                if let Some(slot) = secs.iter_mut().find(|(n, _)| *n == k) {
                    slot.1 = v;
                    continue;
                }
                secs.push((k, v));
            }
        }
        Ok(Table { wall, secs })
    }

    /// The table's seconds for `rung`, scaled to a `total` wall.
    pub fn share(&self, rung: &str, total: f64) -> Option<f64> {
        self.secs
            .iter()
            .find(|(n, _)| n == rung)
            .map(|(_, s)| s * total / self.wall)
    }
}

static TABLE: OnceLock<Option<Table>> = OnceLock::new();

/// The process's table, loaded on first use from `FF_LADDER_TABLE`. `None`
/// when unset, `0.30`, `none`, or unreadable -- an unreadable table is
/// narrated on stderr once and the ladder runs on its defaults, which is
/// the honest failure for a sweep (a row that silently ran a different
/// ladder is a row nobody can attribute).
pub(crate) fn table() -> Option<&'static Table> {
    TABLE
        .get_or_init(|| {
            let v = std::env::var("FF_LADDER_TABLE").ok()?;
            let v = v.trim();
            if v.is_empty() || v.eq_ignore_ascii_case("none") || v == "0.30" {
                return None;
            }
            match std::fs::read_to_string(v) {
                Ok(src) => match Table::parse(&src) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        eprintln!("FF_LADDER_TABLE={v}: {e}; the ladder runs on its defaults");
                        None
                    }
                },
                Err(e) => {
                    eprintln!("FF_LADDER_TABLE={v}: {e}; the ladder runs on its defaults");
                    None
                }
            }
        })
        .as_ref()
}

/// The seconds a rung may spend, given `rem` seconds of wall left at its
/// entry: the explicit `var` fraction of `rem` when set, else the table's
/// share (capped at `rem`), else `default` of `rem`. Pure in its arguments
/// but for the env and the table, so the precedence has one home.
pub(crate) fn slice_secs(rung: &str, var: &str, default: f64, rem: f64) -> f64 {
    if std::env::var(var).is_ok() {
        return crate::search::wall_frac_env(var, default) * rem;
    }
    if let (Some(t), Some(total)) = (table(), crate::search::wall_total_secs()) {
        if let Some(s) = t.share(rung, total) {
            let s = s.min(rem);
            if std::env::var("FF_WALL_DEBUG").is_ok() {
                eprintln!("wall: table gives {rung} {s:.2}s of {rem:.2}s remaining");
            }
            return s;
        }
    }
    default * rem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_text_form() {
        let t = Table::parse(
            "# learned\nwall = 60\nehc = 9\nNovLight=2.5 # tenth\n\nlama = 18\ndriver = 24\n",
        )
        .unwrap();
        assert_eq!(t.wall, 60.0);
        assert_eq!(t.share("ehc", 60.0), Some(9.0));
        assert_eq!(t.share("novlight", 60.0), Some(2.5));
        // Scaled to the armed wall.
        assert_eq!(t.share("lama", 30.0), Some(9.0));
        assert_eq!(t.share("driver", 120.0), Some(48.0));
        // A rung the table does not name keeps its default.
        assert_eq!(t.share("front", 60.0), None);
    }

    #[test]
    fn a_later_line_overrides_an_earlier_one() {
        let t = Table::parse("ehc = 9\nehc = 3\n").unwrap();
        assert_eq!(t.share("ehc", 60.0), Some(3.0));
    }

    #[test]
    fn malformed_lines_are_errors_with_their_number() {
        assert!(Table::parse("ehc 9").unwrap_err().starts_with("line 1"));
        assert!(Table::parse("ehc = nine")
            .unwrap_err()
            .starts_with("line 1"));
        assert!(Table::parse("ehc = -1").unwrap_err().starts_with("line 1"));
        assert!(Table::parse("wall = 0").unwrap_err().starts_with("line 1"));
        assert!(Table::parse("ok = 1\nbad")
            .unwrap_err()
            .starts_with("line 2"));
    }
}
