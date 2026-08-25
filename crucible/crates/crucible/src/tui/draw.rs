//! Drawing one frame.
//!
//! Everything here is a pure function of a `Snapshot` and a `Rect`. That is
//! what makes the layout testable without a terminal, and it is also the
//! cheapest thing to do at 4 fps for three days: no retained widget state, no
//! diffing beyond what ratatui already does, no work proportional to anything
//! but what is actually on screen.
//!
//! The screen degrades rather than fails. A pane narrowed past the minimum gets
//! a compact rendering; a pane narrowed past THAT gets a single honest line. It
//! never panics, because the one thing worse than a small dashboard is a
//! supervisor that died because someone dragged a divider.

use super::app::{Level, LogKind, Snapshot};
use super::theme::Theme;
use super::widget;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Below this the full layout stops being readable and the compact one takes
/// over.
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 16;

/// Vertical budget, top to bottom. The log gets whatever is left, because it is
/// the only region that is useful at any size.
struct Rows {
    header: u16,
    sweep: u16,
    body: u16,
    log: u16,
    keys: u16,
}

fn rows(area: Rect, banner_lines: u16) -> Rows {
    let header = banner_lines + 1;
    let sweep = 3;
    let keys = 1;
    let fixed = header + sweep + keys;
    let rest = area.height.saturating_sub(fixed);
    // The body (tracks and slots) gets two thirds of what is left, the log one
    // third, with the log never smaller than three lines -- a log you cannot
    // read is not a log.
    let log = (rest / 3).max(3).min(rest);
    Rows {
        header,
        sweep,
        body: rest.saturating_sub(log),
        log,
        keys,
    }
}

pub fn draw(f: &mut Frame, s: &Snapshot, th: &Theme, banner_text: &str) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(th.ground)), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        return draw_compact(f, s, th, area);
    }

    let banner = super::banner::render(banner_text, area.width.saturating_sub(28) as usize);
    let r = rows(area, banner.len() as u16);
    let chunks = Layout::vertical([
        Constraint::Length(r.header),
        Constraint::Length(r.sweep),
        Constraint::Length(r.body),
        Constraint::Length(r.log),
        Constraint::Length(r.keys),
    ])
    .split(area);

    draw_header(f, chunks[0], s, th, &banner);
    draw_sweep(f, chunks[1], s, th);
    let body = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(chunks[2]);
    draw_tracks(f, body[0], s, th);
    draw_slots(f, body[1], s, th);
    draw_log(f, chunks[3], s, th);
    draw_keys(f, chunks[4], th);
    draw_toasts(f, area, s, th);
}

fn level_style(th: &Theme, l: Level) -> Style {
    let c = match l {
        Level::Full => th.solved,
        Level::Polite => th.amber,
        Level::Suspended => th.alarm,
    };
    Style::default().fg(c).add_modifier(Modifier::BOLD)
}

fn draw_header(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme, banner: &[String]) {
    let dot = th.glyph("\u{25cf}", "*");
    let mut lines: Vec<Line> = banner
        .iter()
        .map(|b| Line::from(Span::styled(b.clone(), Style::default().fg(th.ember))))
        .collect();

    let mut status = vec![
        Span::styled(format!("  {} ", s.engine_ver), Style::default().fg(th.text)),
        Span::styled(
            format!("[{}]  ", s.engine_hash),
            Style::default().fg(th.dim),
        ),
        Span::styled(
            format!("{dot} {}", s.level.level.label()),
            level_style(th, s.level.level),
        ),
    ];
    if let Some(r) = &s.level.reason {
        status.push(Span::styled(
            format!(" \u{2014} {r}"),
            Style::default().fg(th.dim),
        ));
    }
    if let Some(l) = lines.first_mut() {
        // The status rides on the banner's first row so the header stays as
        // short as the letterforms demand and no shorter.
        let pad = " ".repeat(2);
        l.spans.push(Span::raw(pad));
        l.spans.extend(status);
    }

    let mut sub = format!(
        "  ferroplan sweep \u{00b7} uptime {}",
        widget::duration(s.uptime.as_secs())
    );
    if let Some(q) = s.quiet_in {
        sub.push_str(&format!(
            " \u{00b7} quiet hours in {}",
            widget::until(q.as_secs())
        ));
    }
    lines.push(Line::from(Span::styled(sub, Style::default().fg(th.dim))));
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_sweep(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let w = area.width.saturating_sub(46).clamp(10, 40) as usize;
    let p = &s.sweep;
    let mut top = vec![
        Span::styled(
            "  SWEEP  ",
            Style::default()
                .fg(th.structure)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(widget::bar(th, p.frac(), w), Style::default().fg(th.ember)),
        Span::styled(
            format!("  {:>3.0}%  {}/{}", p.frac() * 100.0, p.done, p.total),
            Style::default().fg(th.text),
        ),
    ];
    if let Some(eta) = p.eta {
        top.push(Span::styled(
            format!("  ETA {}", widget::until(eta.as_secs())),
            Style::default().fg(th.dim),
        ));
    }

    let mut second = vec![Span::styled(
        format!("  coverage {}", p.solved),
        Style::default().fg(th.solved),
    )];
    if let Some(d) = p.delta {
        // A delta only means something against a release measured on the same
        // box; the label carries which one.
        second.push(Span::styled(
            format!(" ({d:+} vs {})", p.delta_vs),
            Style::default().fg(if d < 0 { th.alarm } else { th.dim }),
        ));
    }
    if p.regressions > 0 {
        second.push(Span::styled(
            format!(
                "   regressions {} {}",
                p.regressions,
                th.glyph("\u{26a0}", "!")
            ),
            Style::default().fg(th.alarm).add_modifier(Modifier::BOLD),
        ));
    }
    if p.dirty > 0 {
        // Kept, not lost -- but owed. The board cannot bank until these are
        // re-measured on a quiet box.
        second.push(Span::styled(
            format!("   dirty {} (re-run owed)", p.dirty),
            Style::default().fg(th.amber),
        ));
    }
    f.render_widget(
        Paragraph::new(vec![Line::from(top), Line::from(second)]),
        area,
    );
}

fn draw_tracks(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(" TRACKS", Style::default().fg(th.structure)));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let bw = (inner.width as usize).saturating_sub(34).clamp(6, 12);
    let mut lines = Vec::new();
    for (row, (ti, di)) in s.visible_tracks().into_iter().enumerate() {
        if lines.len() >= inner.height as usize {
            break;
        }
        let t = &s.tracks[ti];
        let sel = row == s.selected;
        let mark = |c: Color| {
            if sel {
                Style::default().fg(th.ground).bg(c)
            } else {
                Style::default().fg(c)
            }
        };
        match di {
            None => {
                let (glyph, colour) = if t.finished {
                    (th.glyph("\u{2714}", "+"), th.solved)
                } else if t.done > 0 {
                    (th.glyph("\u{25b6}", ">"), th.ember)
                } else {
                    (th.glyph("\u{00b7}", "."), th.dim)
                };
                let delta = match t.delta {
                    Some(d) => format!(" {d:+}"),
                    None => String::new(),
                };
                let body = if t.finished {
                    // Collapsed to one line with its final coverage.
                    format!(" {glyph} {:<10} {}/{}{delta}", t.name, t.solved, t.total)
                } else if t.done == 0 {
                    // Queued: a dim single line. A bar pinned at zero reads as
                    // "running and getting nowhere", a different and worse claim.
                    format!(" {glyph} {:<10} {}/{}  [queued]", t.name, t.done, t.total)
                } else {
                    format!(
                        " {glyph} {:<10} {}/{}{delta}  {} {:>3.0}%",
                        t.name,
                        t.done,
                        t.total,
                        widget::bar(th, t.done as f64 / t.total.max(1) as f64, bw),
                        100.0 * t.done as f64 / t.total.max(1) as f64
                    )
                };
                lines.push(Line::from(Span::styled(
                    widget::ellipsize(&body, inner.width as usize),
                    mark(colour),
                )));
            }
            Some(j) => {
                let d = &t.domains[j];
                let warn = if d.regressions > 0 {
                    format!(" {}", th.glyph("\u{26a0}", "!"))
                } else {
                    String::new()
                };
                let body = format!(
                    "      {:<14} {} {}/{}{warn}",
                    widget::ellipsize(&d.name, 14),
                    // The bar tracks SOLVED, not "ran": coverage is the metric,
                    // and a full bar beside "12/30" says the opposite of the truth.
                    widget::bar(th, d.solved as f64 / d.total.max(1) as f64, bw),
                    d.solved,
                    d.total
                );
                let colour = if d.regressions > 0 { th.alarm } else { th.text };
                lines.push(Line::from(Span::styled(
                    widget::ellipsize(&body, inner.width as usize),
                    mark(colour),
                )));
            }
        }
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_slots(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let block = Block::default().title(Span::styled(
        format!(
            " SLOTS{:>w$} P-core ",
            s.p_cores,
            w = (area.width as usize).saturating_sub(16)
        ),
        Style::default().fg(th.structure),
    ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();
    for slot in &s.slots {
        let line = match &slot.what {
            None => Line::from(Span::styled(
                format!(" {} {} idle", slot.index, th.glyph("\u{00b7}", ".")),
                Style::default().fg(th.dim),
            )),
            Some(r) => {
                let (glyph, colour) = if r.suspended {
                    (th.glyph("\u{23f8}", "="), th.amber)
                } else {
                    (th.glyph("\u{25b6}", ">"), th.ember)
                };
                // Effective time, not wall: a suspended run is not about to
                // time out, and showing wall here would suggest it was.
                let clock = if r.suspended {
                    "suspended".to_string()
                } else {
                    format!(
                        "{:>6.1}s/{:.0}s",
                        r.effective.as_secs_f64(),
                        r.budget.as_secs_f64()
                    )
                };
                Line::from(Span::styled(
                    widget::ellipsize(
                        &format!(
                            " {} {glyph} {:<22} {} {clock}",
                            slot.index,
                            format!("{}/{}", widget::ellipsize(&r.variant, 16), r.instance),
                            r.tier
                        ),
                        inner.width as usize,
                    ),
                    Style::default().fg(colour),
                ))
            }
        };
        lines.push(line);
    }
    if !s.throughput.is_empty() && inner.height as usize > lines.len() + 1 {
        lines.push(Line::from(""));
        let w = (inner.width as usize).saturating_sub(24).clamp(6, 24);
        lines.push(Line::from(vec![
            Span::styled(" throughput ", Style::default().fg(th.dim)),
            Span::styled(
                widget::spark(th, &s.throughput, w),
                Style::default().fg(th.ember),
            ),
            Span::styled(
                format!(" {:.0}/min", s.throughput.last().copied().unwrap_or(0.0)),
                Style::default().fg(th.dim),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn kind_colour(th: &Theme, k: LogKind) -> Color {
    match k {
        LogKind::Info => th.text,
        LogKind::Good => th.solved,
        LogKind::Warn => th.amber,
        LogKind::Regression => th.alarm,
    }
}

fn draw_log(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let n = inner.height as usize;
    let lines: Vec<Line> = s
        .log
        .iter()
        .rev()
        .take(n)
        .rev()
        .map(|l| {
            let g = match l.kind {
                LogKind::Info => th.glyph("\u{00b7}", "."),
                LogKind::Good => th.glyph("\u{2714}", "+"),
                LogKind::Warn => th.glyph("\u{26a0}", "!"),
                LogKind::Regression => th.glyph("\u{2716}", "x"),
            };
            Line::from(vec![
                Span::styled(format!(" {}  ", l.at), Style::default().fg(th.dim)),
                Span::styled(
                    format!("{g}  "),
                    Style::default().fg(kind_colour(th, l.kind)),
                ),
                Span::styled(
                    widget::ellipsize(&l.text, inner.width.saturating_sub(14) as usize),
                    Style::default().fg(kind_colour(th, l.kind)),
                ),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_keys(f: &mut Frame, area: Rect, th: &Theme) {
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " j/k move   \u{21b5} expand   z collapse done   d diff   f filter   \
             p pause   s suspend   r re-run   esc dismiss   q quit",
            Style::default().fg(th.dim),
        ))),
        area,
    );
}

fn draw_toasts(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    if s.toasts.is_empty() {
        return;
    }
    let w = 46u16.min(area.width.saturating_sub(4));
    let h = (s.toasts.len() as u16).min(area.height.saturating_sub(4));
    if w < 12 || h == 0 {
        return;
    }
    let rect = Rect {
        x: area.width.saturating_sub(w + 2),
        y: area.height.saturating_sub(h + 3),
        width: w,
        height: h,
    };
    let lines: Vec<Line> = s
        .toasts
        .iter()
        .take(h as usize)
        .map(|t| {
            Line::from(Span::styled(
                widget::ellipsize(&format!(" {} ", t.text), w as usize),
                Style::default()
                    .fg(th.ground)
                    .bg(kind_colour(th, t.kind))
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    f.render_widget(Paragraph::new(lines), rect);
}

/// The fallback for a pane too small for the real layout. One honest line
/// beats a scrambled dashboard, and it must never panic -- a supervisor that
/// died because someone dragged a divider would be a bad joke.
fn draw_compact(f: &mut Frame, s: &Snapshot, th: &Theme, area: Rect) {
    let p = &s.sweep;
    // The loud thing goes FIRST, because this line is about to be truncated
    // and whatever is last is what disappears. A regression that scrolled off
    // the right edge of a narrow pane is a regression nobody saw.
    let text = if p.regressions > 0 {
        format!(
            "REGRESSIONS {}  {} {}/{} {:.0}%",
            p.regressions,
            s.level.level.label(),
            p.done,
            p.total,
            p.frac() * 100.0
        )
    } else {
        format!(
            "{} {}  {}/{} {:.0}%  cov {}",
            super::banner::compact("crucible"),
            s.level.level.label(),
            p.done,
            p.total,
            p.frac() * 100.0,
            p.solved
        )
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            widget::ellipsize(&text, area.width as usize),
            Style::default().fg(if p.regressions > 0 { th.alarm } else { th.text }),
        ))),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::*;
    use ratatui::backend::TestBackend;
    use std::time::Duration;

    fn demo() -> Snapshot {
        Snapshot {
            engine_ver: "ff 0.26.0".into(),
            engine_hash: "2989d528b05e".into(),
            level: LevelState {
                level: Level::Polite,
                reason: Some("Mail 34%".into()),
            },
            uptime: Duration::from_secs(3 * 86400 + 4 * 3600 + 720),
            quiet_in: Some(Duration::from_secs(9660)),
            sweep: SweepProgress {
                done: 1284,
                total: 2103,
                solved: 1102,
                delta: Some(7),
                delta_vs: "0.25.0".into(),
                regressions: 1,
                dirty: 43,
                eta: Some(Duration::from_secs(51_720)),
            },
            tracks: vec![
                TrackProgress {
                    name: "IPC5".into(),
                    done: 86,
                    total: 86,
                    solved: 86,
                    delta: Some(0),
                    domains: vec![],
                    expanded: true,
                    finished: true,
                },
                TrackProgress {
                    name: "IPC6".into(),
                    done: 412,
                    total: 598,
                    solved: 400,
                    delta: Some(5),
                    domains: vec![
                        DomainProgress {
                            name: "openstacks".into(),
                            solved: 30,
                            total: 30,
                            regressions: 0,
                        },
                        DomainProgress {
                            name: "pathways".into(),
                            solved: 12,
                            total: 30,
                            regressions: 1,
                        },
                    ],
                    expanded: true,
                    finished: false,
                },
            ],
            slots: vec![
                Slot {
                    index: 0,
                    what: Some(SlotRun {
                        variant: "rovers".into(),
                        instance: "p12".into(),
                        tier: 'A',
                        effective: Duration::from_millis(4200),
                        suspended: false,
                        budget: Duration::from_secs(60),
                    }),
                },
                Slot {
                    index: 1,
                    what: Some(SlotRun {
                        variant: "storage".into(),
                        instance: "p18".into(),
                        tier: 'C',
                        effective: Duration::from_secs(12),
                        suspended: true,
                        budget: Duration::from_secs(60),
                    }),
                },
                Slot {
                    index: 2,
                    what: None,
                },
            ],
            p_cores: 4,
            log: vec![
                LogLine {
                    at: "14:02:11".into(),
                    kind: LogKind::Warn,
                    text: "POLITE -- foreign CPU 34% (Mail) -- demoted to E-cores".into(),
                },
                LogLine {
                    at: "14:04:02".into(),
                    kind: LogKind::Regression,
                    text: "REGRESSION IPC6/pathways/p07 -- solved in 0.25.0, timeout".into(),
                },
            ],
            toasts: vec![Toast {
                text: "REGRESSION pathways/p07".into(),
                kind: LogKind::Regression,
                sticky: true,
                age: Duration::ZERO,
            }],
            throughput: vec![1.0, 3.0, 8.0, 14.0, 22.0, 14.0, 8.0, 3.0, 1.0],
            selected: 1,
        }
    }

    fn render(w: u16, h: u16) -> ratatui::buffer::Buffer {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        let th = Theme::forge();
        term.draw(|f| draw(f, &demo(), &th, "CRUCIBLE")).unwrap();
        term.backend().buffer().clone()
    }

    fn text(buf: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn a_full_size_frame_shows_the_things_that_matter() {
        let t = text(&render(120, 34));
        assert!(t.contains("POLITE"), "the throttle level is always visible");
        assert!(t.contains("1284/2103"));
        assert!(t.contains("REGRESSION"), "a regression must be on screen");
        assert!(t.contains("dirty 43"), "work owed is shown, not hidden");
    }

    /// A suspended run shows "suspended", never a wall clock creeping toward
    /// its budget -- the whole point is that it is NOT about to time out.
    #[test]
    fn a_suspended_slot_does_not_show_a_countdown() {
        let t = text(&render(120, 34));
        assert!(t.contains("suspended"));
    }

    /// The screen must survive any size a person can drag a pane to. A
    /// supervisor that panicked on a resize would be a bad joke.
    #[test]
    fn every_plausible_terminal_size_renders_without_panicking() {
        for (w, h) in [
            (1, 1),
            (2, 3),
            (10, 5),
            (40, 10),
            (59, 15),
            (60, 16),
            (80, 24),
            (120, 34),
            (400, 100),
            (200, 8),
        ] {
            let _ = render(w, h);
        }
    }

    /// Below the minimum the layout collapses to one honest line rather than a
    /// scrambled one -- and it still says the loud thing.
    #[test]
    fn a_tiny_pane_still_reports_a_regression() {
        let t = text(&render(50, 6));
        assert!(t.contains("REGRESSION"), "got: {t:?}");
    }

    /// A queued track says so rather than drawing a bar pinned at zero.
    #[test]
    fn a_queued_track_reads_as_queued() {
        let mut s = demo();
        s.tracks.push(TrackProgress {
            name: "IPC7".into(),
            done: 0,
            total: 419,
            solved: 0,
            delta: None,
            domains: vec![],
            expanded: false,
            finished: false,
        });
        let mut term = Terminal::new(TestBackend::new(120, 34)).unwrap();
        let th = Theme::forge();
        term.draw(|f| draw(f, &s, &th, "CRUCIBLE")).unwrap();
        assert!(text(term.backend().buffer()).contains("[queued]"));
    }

    /// A finished track occupies one line even though it is marked expanded.
    #[test]
    fn a_finished_track_does_not_crowd_out_the_active_one() {
        let t = text(&render(120, 34));
        assert!(t.contains("IPC5"));
        assert!(
            t.contains("openstacks"),
            "the ACTIVE track's domains are visible"
        );
    }
}
