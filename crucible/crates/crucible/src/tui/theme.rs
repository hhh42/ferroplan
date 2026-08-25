//! Forge and iron: deep charcoal ground, ember for what is running, steel for
//! structure, green for solved, amber for dirty, red for a regression.
//!
//! The palette is a function of terminal capability, not a constant. Kitty
//! reports truecolor and gets the real thing; a 256-colour terminal gets the
//! nearest indexed match; anything that claims neither gets the eight ANSI
//! colours and ASCII glyphs. Degrading is not a nicety here -- this program is
//! meant to sit resident in a pane for days, and a dashboard that renders as
//! mojibake on the one terminal you have is a dashboard you turn off.

use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    True,
    Indexed,
    Ansi,
}

impl Depth {
    /// Read the terminal's own claims. COLORTERM is the only reliable signal
    /// for truecolor; TERM carries the rest.
    pub fn detect() -> Depth {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm.contains("truecolor") || colorterm.contains("24bit") {
            return Depth::True;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term.contains("kitty") || term.contains("direct") {
            return Depth::True;
        }
        if term.contains("256") {
            return Depth::Indexed;
        }
        Depth::Ansi
    }
}

/// Whether the terminal can be trusted with block-drawing and braille.
///
/// A bar rendered as replacement characters is worse than a bar made of `#`,
/// so this is checked rather than assumed.
pub fn unicode_ok() -> bool {
    let lang = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LC_CTYPE"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
        .to_ascii_uppercase();
    lang.contains("UTF-8") || lang.contains("UTF8")
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub depth: Depth,
    pub unicode: bool,
    pub ground: Color,
    pub structure: Color,
    pub dim: Color,
    pub text: Color,
    /// Running right now.
    pub ember: Color,
    pub solved: Color,
    /// Dirty timing, or a timeout -- a real result measured under a cloud.
    pub amber: Color,
    /// A regression. Loud on purpose.
    pub alarm: Color,
}

impl Theme {
    pub fn forge() -> Theme {
        let depth = Depth::detect();
        let c = |r: u8, g: u8, b: u8, idx: u8, ansi: Color| match depth {
            Depth::True => Color::Rgb(r, g, b),
            Depth::Indexed => Color::Indexed(idx),
            Depth::Ansi => ansi,
        };
        Theme {
            depth,
            unicode: unicode_ok(),
            ground: c(18, 18, 20, 234, Color::Black),
            structure: c(88, 110, 140, 67, Color::Blue),
            dim: c(96, 96, 104, 242, Color::DarkGray),
            text: c(214, 214, 210, 252, Color::Gray),
            ember: c(224, 122, 40, 208, Color::Yellow),
            solved: c(126, 176, 96, 107, Color::Green),
            amber: c(206, 170, 60, 179, Color::Yellow),
            alarm: c(206, 78, 66, 167, Color::Red),
        }
    }

    /// Filled and empty cells for a progress bar.
    pub fn bar_cells(&self) -> (&'static str, &'static str) {
        if self.unicode {
            ("\u{2588}", "\u{2591}")
        } else {
            ("#", ".")
        }
    }

    /// The eight braille-ish levels a sparkline draws with.
    pub fn spark_cells(&self) -> &'static [&'static str] {
        if self.unicode {
            &[
                " ", "\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}",
                "\u{2587}", "\u{2588}",
            ]
        } else {
            &["_", ".", ".", "-", "-", "=", "=", "#", "#"]
        }
    }

    pub fn glyph(&self, unicode: &'static str, ascii: &'static str) -> &'static str {
        if self.unicode {
            unicode
        } else {
            ascii
        }
    }
}
