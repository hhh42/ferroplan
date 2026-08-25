//! The small drawing primitives, kept pure so they can be tested without a
//! terminal.
//!
//! Everything here is cheap by construction. The dashboard redraws at 4 fps and
//! has to stay well under one percent of a core: it is watching a benchmark,
//! and a dashboard that perturbs its own measurement is worse than no
//! dashboard. So: no allocation per cell, no gradients, no animation that is
//! not a single character changing.

use super::theme::Theme;

/// A block progress bar of exactly `width` cells.
pub fn bar(theme: &Theme, frac: f64, width: usize) -> String {
    let (full, empty) = theme.bar_cells();
    let frac = frac.clamp(0.0, 1.0);
    // Truncate rather than round: a bar must never show full until it IS full.
    // On a three-day sweep a bar that reads 100% at 99.6% is a small lie told
    // for hours.
    let n = ((frac * width as f64) as usize).min(width);
    let mut s = String::with_capacity(width * 3);
    for _ in 0..n {
        s.push_str(full);
    }
    for _ in n..width {
        s.push_str(empty);
    }
    s
}

/// A sparkline over the most recent `width` samples.
pub fn spark(theme: &Theme, values: &[f64], width: usize) -> String {
    let cells = theme.spark_cells();
    if values.is_empty() || width == 0 {
        return " ".repeat(width);
    }
    let tail = &values[values.len().saturating_sub(width)..];
    let hi = tail.iter().cloned().fold(f64::MIN, f64::max);
    let lo = tail.iter().cloned().fold(f64::MAX, f64::min);
    let span = (hi - lo).max(f64::EPSILON);
    let mut s = String::with_capacity(width * 3);
    for _ in tail.len()..width {
        s.push(' ');
    }
    for v in tail {
        let idx = (((v - lo) / span) * (cells.len() - 1) as f64).round() as usize;
        s.push_str(cells[idx.min(cells.len() - 1)]);
    }
    s
}

/// How long something has been running: `3d 04:12`, `04:12:33`, `12:33`.
pub fn duration(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if d > 0 {
        format!("{d}d {h:02}:{m:02}")
    } else if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// How long UNTIL something: `14h 22m`, `2h 41m`, `18m`.
///
/// A different shape from `duration` on purpose. An ETA is read at a glance
/// against a wall clock, where `14h 22m` lands immediately and `14:22:00`
/// invites a second of arithmetic about whether it is a duration or a time.
pub fn until(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{secs}s")
    }
}

/// Truncate to `width` columns, marking the cut so a clipped name never reads
/// as a shorter name.
pub fn ellipsize(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    if width <= 1 {
        return ".".repeat(width);
    }
    let mut out: String = s.chars().take(width - 1).collect();
    out.push('~');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Theme {
        Theme {
            unicode: true,
            ..Theme::forge()
        }
    }

    /// A bar must never read full until it IS full: on a multi-day sweep, a
    /// rounded bar shows 100% for hours before the work is done.
    #[test]
    fn a_bar_is_only_full_when_complete() {
        let th = t();
        let (full, _) = th.bar_cells();
        assert!(
            !bar(&th, 0.999, 20).ends_with(full) || bar(&th, 0.999, 20).matches(full).count() < 20
        );
        assert_eq!(bar(&th, 1.0, 20).matches(full).count(), 20);
        assert_eq!(bar(&th, 0.0, 20).matches(full).count(), 0);
    }

    #[test]
    fn a_bar_is_always_exactly_its_width() {
        let th = t();
        for f in [0.0, 0.13, 0.5, 0.999, 1.0, 1.5, -0.2] {
            assert_eq!(bar(&th, f, 20).chars().count(), 20, "frac {f}");
        }
    }

    /// Terminal size is not a promise. A pane narrowed below the minimum must
    /// render something, never panic.
    #[test]
    fn widgets_survive_degenerate_widths() {
        let th = t();
        assert_eq!(bar(&th, 0.5, 0), "");
        assert_eq!(spark(&th, &[1.0, 2.0], 0), "");
        assert_eq!(ellipsize("abcdef", 0), "");
        assert_eq!(ellipsize("abcdef", 1), ".");
    }

    #[test]
    fn a_flat_series_does_not_divide_by_zero() {
        let th = t();
        let s = spark(&th, &[5.0, 5.0, 5.0], 3);
        assert_eq!(s.chars().count(), 3);
    }

    #[test]
    fn a_short_series_is_right_aligned() {
        let th = t();
        let s = spark(&th, &[1.0, 9.0], 5);
        assert_eq!(s.chars().count(), 5);
        assert!(s.starts_with("   "));
    }

    /// An ASCII terminal gets a bar it can actually draw. A row of replacement
    /// characters is worse than a row of hashes.
    #[test]
    fn ascii_terminals_get_ascii_bars() {
        let th = Theme {
            unicode: false,
            ..Theme::forge()
        };
        let b = bar(&th, 0.5, 10);
        assert!(b.is_ascii(), "{b:?}");
        assert_eq!(b.chars().count(), 10);
    }

    /// An ETA reads as a countdown, not as a time of day.
    #[test]
    fn an_eta_reads_at_a_glance() {
        assert_eq!(until(51_720), "14h 22m");
        assert_eq!(until(9_660), "2h 41m");
        assert_eq!(until(1_080), "18m");
        assert_eq!(until(42), "42s");
        assert_eq!(until(3 * 86_400 + 4 * 3600), "3d 4h");
    }

    #[test]
    fn a_clipped_name_is_marked_as_clipped() {
        assert_eq!(ellipsize("openstacks", 6), "opens~");
        assert_eq!(ellipsize("short", 20), "short");
    }
}
