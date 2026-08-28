//! The block-letter banner.
//!
//! The spec reaches for `figlet-rs` with an embedded font. A five-row alphabet
//! is about forty lines of data, so the dependency buys nothing but a file
//! format -- and this crate already carries a terminal library, a database and
//! a process supervisor. One fewer thing to audit.
//!
//! The banner is drawn once and cached; at 4 fps for three days, redrawing
//! letterforms would be the single most expensive thing on screen.

/// Five rows per glyph, six columns each, ASCII only so it renders anywhere.
const ROWS: usize = 5;
const W: usize = 6;

/// A-Z, 0-9, space, and a few marks. Anything unmapped renders blank.
const FONT: &[(char, [&str; ROWS])] = &[
    ('A', ["  ##  ", " #  # ", "######", "#    #", "#    #"]),
    ('B', ["##### ", "#    #", "##### ", "#    #", "##### "]),
    ('C', [" #### ", "#    #", "#     ", "#    #", " #### "]),
    ('D', ["##### ", "#    #", "#    #", "#    #", "##### "]),
    ('E', ["######", "#     ", "##### ", "#     ", "######"]),
    ('F', ["######", "#     ", "##### ", "#     ", "#     "]),
    ('G', [" #### ", "#     ", "#  ###", "#    #", " #### "]),
    ('H', ["#    #", "#    #", "######", "#    #", "#    #"]),
    ('I', [" #### ", "  ##  ", "  ##  ", "  ##  ", " #### "]),
    ('J', ["    ##", "    ##", "    ##", "#   ##", " #### "]),
    ('K', ["#   ##", "#  ## ", "####  ", "#  ## ", "#   ##"]),
    ('L', ["#     ", "#     ", "#     ", "#     ", "######"]),
    ('M', ["#    #", "##  ##", "# ## #", "#    #", "#    #"]),
    ('N', ["#    #", "##   #", "# #  #", "#  # #", "#   ##"]),
    ('O', [" #### ", "#    #", "#    #", "#    #", " #### "]),
    ('P', ["##### ", "#    #", "##### ", "#     ", "#     "]),
    ('Q', [" #### ", "#    #", "#    #", "#  ## ", " ## ##"]),
    ('R', ["##### ", "#    #", "##### ", "#  ## ", "#   ##"]),
    ('S', [" #####", "#     ", " #### ", "     #", "##### "]),
    ('T', ["######", "  ##  ", "  ##  ", "  ##  ", "  ##  "]),
    ('U', ["#    #", "#    #", "#    #", "#    #", " #### "]),
    ('V', ["#    #", "#    #", "#    #", " #  # ", "  ##  "]),
    ('W', ["#    #", "#    #", "# ## #", "##  ##", "#    #"]),
    ('X', ["#    #", " #  # ", "  ##  ", " #  # ", "#    #"]),
    ('Y', ["#    #", " #  # ", "  ##  ", "  ##  ", "  ##  "]),
    ('Z', ["######", "    # ", "  ##  ", " #    ", "######"]),
    ('0', [" #### ", "#   ##", "# ## #", "##   #", " #### "]),
    ('1', ["  ##  ", " ###  ", "  ##  ", "  ##  ", "######"]),
    ('2', [" #### ", "#    #", "   ## ", " ##   ", "######"]),
    ('3', ["##### ", "     #", "  ### ", "     #", "##### "]),
    ('4', ["#   # ", "#   # ", "######", "    # ", "    # "]),
    ('5', ["######", "#     ", "##### ", "     #", "##### "]),
    ('6', [" #### ", "#     ", "##### ", "#    #", " #### "]),
    ('7', ["######", "    # ", "   #  ", "  #   ", " #    "]),
    ('8', [" #### ", "#    #", " #### ", "#    #", " #### "]),
    ('9', [" #### ", "#    #", " #####", "     #", " #### "]),
    ('.', ["      ", "      ", "      ", "  ##  ", "  ##  "]),
    ('-', ["      ", "      ", " #### ", "      ", "      "]),
];

fn glyph(c: char) -> Option<&'static [&'static str; ROWS]> {
    let c = c.to_ascii_uppercase();
    FONT.iter().find(|(k, _)| *k == c).map(|(_, g)| g)
}

/// Render `text` as five rows, clipped to `max_width` columns.
///
/// Clipping rather than wrapping: a banner that reflows changes the height of
/// everything below it, and a pane that is briefly narrow would then reshuffle
/// the whole dashboard.
pub fn render(text: &str, max_width: usize) -> Vec<String> {
    let mut rows = vec![String::new(); ROWS];
    for ch in text.chars() {
        if ch == ' ' {
            for r in rows.iter_mut() {
                r.push_str("   ");
            }
            continue;
        }
        let Some(g) = glyph(ch) else { continue };
        if rows[0].chars().count() + W + 1 > max_width {
            break;
        }
        for (r, line) in rows.iter_mut().zip(g.iter()) {
            r.push_str(line);
            r.push(' ');
        }
    }
    for r in rows.iter_mut() {
        while r.chars().count() > max_width {
            r.pop();
        }
    }
    rows
}

/// The narrow fallback: when the pane cannot hold letterforms, say the name
/// plainly rather than drawing a fragment of it.
pub fn compact(text: &str) -> String {
    text.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_the_same_width() {
        let rows = render("CRUCIBLE", 200);
        let w = rows[0].chars().count();
        assert!(w > 0);
        for r in &rows {
            assert_eq!(r.chars().count(), w);
        }
    }

    /// A narrow pane must clip, never wrap: wrapping would change the banner's
    /// height and reshuffle everything below it.
    #[test]
    fn a_narrow_pane_clips_and_keeps_its_height() {
        let rows = render("CRUCIBLE", 20);
        assert_eq!(rows.len(), ROWS);
        for r in &rows {
            assert!(r.chars().count() <= 20);
        }
    }

    #[test]
    fn unmapped_characters_are_skipped_not_drawn_as_holes() {
        let a = render("AB", 200);
        let b = render("A\u{263A}B", 200);
        assert_eq!(a, b);
    }

    #[test]
    fn a_zero_width_pane_produces_empty_rows_not_a_panic() {
        let rows = render("CRUCIBLE", 0);
        assert_eq!(rows.len(), ROWS);
        assert!(rows.iter().all(|r| r.is_empty()));
    }

    /// The banner is configurable, so it has to handle a name it was not
    /// designed around.
    #[test]
    fn alternate_names_render() {
        for name in ["BELLOWS", "ANVIL", "FORGE", "FERROSWEEP", "0.26.0"] {
            let rows = render(name, 200);
            assert!(rows[0].chars().count() > 0, "{name}");
        }
    }
}
