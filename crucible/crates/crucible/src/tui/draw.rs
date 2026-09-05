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

use super::app::{strip, Cell, Level, LogKind, Snapshot, View};
use super::theme::Theme;
use super::widget;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

/// Below this the full layout stops being readable and the compact one takes
/// over.
pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 16;

/// Vertical budget, top to bottom. The body gets whatever is left after the
/// fixed rows; the log keeps a floor of three lines, because a log you cannot
/// read is not a log.
struct Rows {
    header: u16,
    sweep: u16,
    body: u16,
    slots: u16,
    log: u16,
    keys: u16,
}

fn rows(area: Rect, banner_lines: u16, slots: u16) -> Rows {
    let header = banner_lines + 1;
    let sweep = 2;
    let keys = 1;
    let slots = slots.min(4) + 1;
    let fixed = header + sweep + keys + slots;
    let rest = area.height.saturating_sub(fixed);
    let log = (rest / 4).clamp(3, 8).min(rest);
    Rows {
        header,
        sweep,
        body: rest.saturating_sub(log),
        slots,
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
    let r = rows(area, banner.len() as u16, s.slots.len().max(1) as u16);
    let chunks = Layout::vertical([
        Constraint::Length(r.header),
        Constraint::Length(r.sweep),
        Constraint::Length(r.body),
        Constraint::Length(r.slots),
        Constraint::Length(r.log),
        Constraint::Length(r.keys),
    ])
    .split(area);

    draw_header(f, chunks[0], s, th, &banner);
    draw_sweep(f, chunks[1], s, th);
    match s.view {
        View::Grid => draw_grid(f, chunks[2], s, th),
        View::Board => draw_board(f, chunks[2], s, th),
        View::Instance => draw_instance(f, chunks[2], s, th),
        View::Timeline => draw_timeline(
            f,
            chunks[2],
            s,
            th,
            &s.timeline,
            "TIMELINE -- the whole sweep",
        ),
    }
    draw_slots(f, chunks[3], s, th);
    draw_log(f, chunks[4], s, th);
    draw_keys(f, chunks[5], s, th);
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
            format!(" \u{2014} {}", widget::ellipsize(r, 40)),
            Style::default().fg(th.dim),
        ));
    }
    if let Some(c) = s.canary {
        let slow = c > 1.15;
        status.push(Span::styled(
            format!("   canary {c:.2}x"),
            Style::default().fg(if slow { th.alarm } else { th.dim }),
        ));
    }
    if let Some(l) = lines.first_mut() {
        l.spans.push(Span::raw("  "));
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
    if !s.competitors.is_empty() {
        let top: Vec<String> = s
            .competitors
            .iter()
            .take(3)
            .map(|(n, p)| format!("{} {p:.0}%", widget::ellipsize(n, 18)))
            .collect();
        sub.push_str(&format!(" \u{00b7} foreign: {}", top.join(", ")));
    }
    lines.push(Line::from(Span::styled(
        widget::ellipsize(&sub, area.width as usize),
        Style::default().fg(th.dim),
    )));
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_sweep(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let w = area.width.saturating_sub(50).clamp(10, 40) as usize;
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
            format!(
                "  {:>3.0}%  {}/{} banked",
                p.frac() * 100.0,
                p.done,
                p.total
            ),
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
    if p.owed > 0 {
        second.push(Span::styled(
            format!("   owed {}", p.owed),
            Style::default().fg(th.amber),
        ));
    }
    f.render_widget(
        Paragraph::new(vec![Line::from(top), Line::from(second)]),
        area,
    );
}

/// The glyph and colour of one strip column.
fn cell_span(th: &Theme, cell: Cell, regression: bool, gain: bool) -> Span<'static> {
    let (uni, asc, colour) = match cell {
        Cell::Queued => ("\u{00b7}", ".", th.dim),
        Cell::Running => ("\u{25b6}", ">", th.ember),
        Cell::SolvedClean => ("\u{2588}", "#", th.solved),
        Cell::SolvedDirty => ("\u{2593}", "=", th.solved),
        Cell::TimeoutBanked => ("\u{2592}", "-", th.structure),
        Cell::Owed => ("\u{2591}", "~", th.amber),
        Cell::Error => ("\u{2716}", "x", th.alarm),
    };
    let mut style = Style::default().fg(colour);
    if regression {
        style = style.add_modifier(Modifier::UNDERLINED).fg(th.alarm);
    } else if gain {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    Span::styled(th.glyph(uni, asc).to_string(), style)
}

/// THE GRID. One row per board, one cell per instance, worst-of-k when the
/// strip is narrower than the board.
fn draw_grid(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(
            format!(" BOARDS {} ", s.boards.len()),
            Style::default().fg(th.structure),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width < 40 || inner.height == 0 {
        return;
    }
    // Columns: id (20) banked/total (9) owed (5) delta (5) strip (rest).
    let label_w = 20usize;
    let strip_w = (inner.width as usize).saturating_sub(label_w + 9 + 5 + 6 + 4);
    let mut lines = Vec::new();
    let first = s
        .sel_board
        .saturating_sub(inner.height as usize / 2)
        .min(s.boards.len().saturating_sub(inner.height as usize));
    for (i, b) in s.boards.iter().enumerate().skip(first) {
        if lines.len() >= inner.height as usize {
            break;
        }
        let sel = i == s.sel_board;
        let base = if sel {
            Style::default().fg(th.ground).bg(th.structure)
        } else {
            Style::default().fg(th.text)
        };
        let mark = if b.done() {
            th.glyph("\u{2714}", "+")
        } else if b.running() {
            th.glyph("\u{25b6}", ">")
        } else {
            " "
        };
        let delta = match b.prev_solved() {
            Some(p) => format!("{:>+5}", b.solved() as i64 - p as i64),
            None => "     ".into(),
        };
        let mut spans = vec![Span::styled(
            format!(
                "{mark} {:<label_w$} {:>4}/{:<4}{:>4} {delta} ",
                widget::ellipsize(&b.id, label_w),
                b.banked(),
                b.total(),
                b.owed(),
            ),
            base,
        )];
        spans.push(Span::styled(
            th.glyph("\u{2595}", "|").to_string(),
            Style::default().fg(th.dim),
        ));
        for col in strip(&b.cells, strip_w) {
            spans.push(cell_span(th, col.cell, col.regression, col.gain));
        }
        spans.push(Span::styled(
            th.glyph("\u{258f}", "|").to_string(),
            Style::default().fg(th.dim),
        ));
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

/// One board: every instance, sortable, with this run beside the predecessor.
fn draw_board(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let Some(b) = s.board() else {
        return;
    };
    let title = format!(
        " {} ({}, {}s{}) \u{00b7} {}/{} banked, {} owed, {} solved{} \u{00b7} sort: {} ",
        b.id,
        b.label,
        b.budget_secs,
        if b.threads > 1 {
            format!(", {} threads", b.threads)
        } else {
            String::new()
        },
        b.banked(),
        b.total(),
        b.owed(),
        b.solved(),
        match b.prev_solved() {
            Some(p) => format!(
                " ({:+} vs prev: {} gained, {} lost)",
                b.solved() as i64 - p as i64,
                b.gains(),
                b.regressions()
            ),
            None => String::new(),
        },
        s.sort.label()
    );
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(title, Style::default().fg(th.structure)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 3 {
        return;
    }
    // Histogram strip on the right when there is room.
    let (table_area, hist_area) = if inner.width >= 100 {
        let h = Layout::horizontal([Constraint::Min(70), Constraint::Length(30)]).split(inner);
        (h[0], Some(h[1]))
    } else {
        (inner, None)
    };

    let mut lines = vec![Line::from(Span::styled(
        format!(
            " {:<3}{:<26} {:>8} {:>8} {:>7} {:>5} {:>3}  {}",
            "", "instance", "this", "prev", "delta", "rho", "att", "verdict"
        ),
        Style::default().fg(th.dim),
    ))];
    let order = s.sorted_instances();
    let rows_avail = table_area.height.saturating_sub(1) as usize;
    let first = s
        .sel_inst
        .saturating_sub(rows_avail / 2)
        .min(order.len().saturating_sub(rows_avail));
    for (row, &i) in order.iter().enumerate().skip(first) {
        if lines.len() > rows_avail {
            break;
        }
        let c = &b.cells[i];
        let sel = row == s.sel_inst;
        let secs = |v: Option<f64>| v.map_or(format!("{:>8}", "-"), |x| format!("{x:>7.2}s"));
        let delta = c
            .delta_secs()
            .map_or("      -".to_string(), |d| format!("{d:>+7.2}"));
        let rho = c.rho.map_or("    -".to_string(), |r| format!("{r:>5.2}"));
        let verdict = c.verdict.clone().unwrap_or_default();
        let text = format!(
            "{:<26} {} {} {delta} {rho} {:>3}  {verdict}",
            widget::ellipsize(&format!("{}/{}", c.variant, c.label), 26),
            secs(c.this_secs),
            secs(c.prev_secs),
            c.attempt
        );
        let colour = if c.regression() {
            th.alarm
        } else if c.gain() {
            th.solved
        } else {
            th.text
        };
        let style = if sel {
            Style::default().fg(th.ground).bg(th.structure)
        } else {
            Style::default().fg(colour)
        };
        lines.push(Line::from(vec![
            Span::raw(" "),
            cell_span(th, c.cell, c.regression(), c.gain()),
            Span::raw("  "),
            Span::styled(
                widget::ellipsize(&text, table_area.width.saturating_sub(4) as usize),
                style,
            ),
        ]));
    }
    f.render_widget(Paragraph::new(lines), table_area);

    if let Some(h) = hist_area {
        draw_histogram(f, h, s, th);
    }
}

/// Solve times against the wall, in eight bins; the near-wall band counted.
fn draw_histogram(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let Some(b) = s.board() else {
        return;
    };
    let secs = b.solve_secs();
    let budget = b.budget_secs.max(1) as f64;
    let bins = 8usize;
    let mut counts = vec![0usize; bins];
    for t in &secs {
        let k = ((t / budget) * bins as f64).floor() as usize;
        counts[k.min(bins - 1)] += 1;
    }
    let max = *counts.iter().max().unwrap_or(&1).max(&1);
    let mut lines = vec![Line::from(Span::styled(
        format!(" solve times vs {}s wall", b.budget_secs),
        Style::default().fg(th.dim),
    ))];
    let bar_w = (area.width as usize).saturating_sub(14).max(4);
    for (k, n) in counts.iter().enumerate() {
        let lo = budget * k as f64 / bins as f64;
        let near = k * 4 >= bins * 3;
        let bar = widget::bar(th, *n as f64 / max as f64, bar_w);
        lines.push(Line::from(vec![
            Span::styled(format!(" {lo:>4.0}s "), Style::default().fg(th.dim)),
            Span::styled(
                bar,
                Style::default().fg(if near { th.amber } else { th.solved }),
            ),
            Span::styled(format!(" {n}"), Style::default().fg(th.text)),
        ]));
    }
    lines.push(Line::from(Span::styled(
        format!(" near the wall (\u{2265}75%): {}", b.near_wall()),
        Style::default().fg(th.amber),
    )));
    f.render_widget(Paragraph::new(lines), area);
}

/// One instance: every attempt, the box across the latest window.
fn draw_instance(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let (Some(b), Some(c)) = (s.board(), s.selected_instance()) else {
        return;
    };
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(
            format!(
                " {} \u{00b7} {}/{} \u{00b7} {} ",
                b.id,
                c.variant,
                c.label,
                c.cell.label()
            ),
            Style::default().fg(th.structure),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let mut lines = vec![Line::from(vec![
        Span::styled(" this ", Style::default().fg(th.dim)),
        Span::styled(
            c.this_secs.map_or("-".into(), |t| format!("{t:.2}s")),
            Style::default().fg(th.text),
        ),
        Span::styled("   prev ", Style::default().fg(th.dim)),
        Span::styled(
            c.prev_secs.map_or("-".into(), |t| format!("{t:.2}s")),
            Style::default().fg(th.text),
        ),
        Span::styled("   verdict ", Style::default().fg(th.dim)),
        Span::styled(
            c.verdict.clone().unwrap_or_else(|| "-".into()),
            Style::default().fg(if c.cell == Cell::Owed {
                th.amber
            } else {
                th.text
            }),
        ),
        Span::styled(
            if c.regression() {
                "   REGRESSION"
            } else if c.gain() {
                "   gain"
            } else {
                ""
            },
            Style::default()
                .fg(if c.regression() { th.alarm } else { th.solved })
                .add_modifier(Modifier::BOLD),
        ),
    ])];
    match &s.detail {
        None => lines.push(Line::from(Span::styled(
            " reading the database\u{2026}",
            Style::default().fg(th.dim),
        ))),
        Some(d) => {
            lines.push(Line::from(Span::styled(
                format!(
                    " {:>3} {:>7} {:>8} {:>8} {:>8} {:>6} {:>8} {:<8} verdict",
                    "att", "solved", "time", "wall", "cpu", "rho", "rss", "timing"
                ),
                Style::default().fg(th.dim),
            )));
            for a in &d.attempts {
                lines.push(Line::from(Span::styled(
                    format!(
                        " {:>3} {:>7} {:>8} {:>8} {:>8} {:>6} {:>8} {:<8} {}",
                        a.attempt,
                        if a.solved { "yes" } else { "no" },
                        a.secs.map_or("-".into(), |t| format!("{t:.2}s")),
                        a.wall_ms
                            .map_or("-".into(), |w| format!("{:.1}s", w as f64 / 1000.0)),
                        a.cpu_ms
                            .map_or("-".into(), |w| format!("{:.1}s", w as f64 / 1000.0)),
                        a.rho().map_or("-".into(), |r| format!("{r:.3}")),
                        a.peak_rss.map_or("-".into(), |r| format!("{}M", r >> 20)),
                        a.timing,
                        a.verdict.clone().unwrap_or_default()
                    ),
                    Style::default().fg(th.text),
                )));
            }
            let mut box_line = String::from(" across the latest window:");
            if let Some(cm) = d.canary_max {
                box_line.push_str(&format!("  canary max {cm:.2}x"));
            }
            if let Some(sg) = d.swap_growth_mb {
                box_line.push_str(&format!("  swap {sg:+.0} MB"));
            }
            if !d.competitors.is_empty() {
                let top: Vec<String> = d
                    .competitors
                    .iter()
                    .take(4)
                    .map(|(n, p)| format!("{} {p:.0}%", widget::ellipsize(n, 18)))
                    .collect();
                box_line.push_str(&format!("  foreign: {}", top.join(", ")));
            }
            lines.push(Line::from(Span::styled(
                widget::ellipsize(&box_line, inner.width as usize),
                Style::default().fg(th.dim),
            )));
        }
    }
    let used = lines.len() as u16;
    f.render_widget(Paragraph::new(lines), inner);
    if let Some(d) = &s.detail {
        if inner.height > used + 3 {
            let rest = Rect {
                x: inner.x,
                y: inner.y + used,
                width: inner.width,
                height: inner.height - used,
            };
            draw_timeline(
                f,
                rest,
                s,
                th,
                &d.timeline,
                "the box across this instance's window",
            );
        }
    }
}

/// The box over a span: foreign load and the canary as sparklines, throttle
/// windows as a band, runs as marks.
fn draw_timeline(
    f: &mut Frame,
    area: Rect,
    _s: &Snapshot,
    th: &Theme,
    t: &super::app::Timeline,
    title: &str,
) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(th.structure),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height < 2 || t.points.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " no samples in this span",
                Style::default().fg(th.dim),
            ))),
            inner,
        );
        return;
    }
    let w = (inner.width as usize).saturating_sub(12).max(8);
    let n = t.points.len();
    // Resample to the width: max within each bucket, so a burst is not lost.
    let bucket = |get: &dyn Fn(&super::app::TimelinePoint) -> Option<f64>| -> Vec<f64> {
        let per = n.div_ceil(w).max(1);
        t.points
            .chunks(per)
            .map(|ch| ch.iter().filter_map(get).fold(0.0f64, f64::max))
            .collect()
    };
    let foreign = bucket(&|p| p.foreign);
    let canary = bucket(&|p| p.canary.map(|c| (c - 1.0).max(0.0) * 100.0));
    let swap = bucket(&|p| p.swap_mb);
    let span_s = t.points.last().map(|p| p.at).unwrap_or(0.0)
        - t.points.first().map(|p| p.at).unwrap_or(0.0);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" foreign %  ", Style::default().fg(th.dim)),
            Span::styled(
                widget::spark(th, &foreign, w),
                Style::default().fg(th.amber),
            ),
            Span::styled(
                format!(" max {:.0}", foreign.iter().cloned().fold(0.0, f64::max)),
                Style::default().fg(th.dim),
            ),
        ]),
        Line::from(vec![
            Span::styled(" canary +%  ", Style::default().fg(th.dim)),
            Span::styled(widget::spark(th, &canary, w), Style::default().fg(th.ember)),
            Span::styled(
                format!(" max {:.0}", canary.iter().cloned().fold(0.0, f64::max)),
                Style::default().fg(th.dim),
            ),
        ]),
        Line::from(vec![
            Span::styled(" swap MB    ", Style::default().fg(th.dim)),
            Span::styled(
                widget::spark(th, &swap, w),
                Style::default().fg(th.structure),
            ),
            Span::styled(
                format!(" max {:.0}", swap.iter().cloned().fold(0.0, f64::max)),
                Style::default().fg(th.dim),
            ),
        ]),
    ];
    // Throttle band and runs, as one row each of marks.
    if let (Some(first), Some(last)) = (t.points.first(), t.points.last()) {
        let (a, z) = (first.at, last.at.max(first.at + 1.0));
        let col = |ts: f64| {
            (((ts - a) / (z - a)) * (w as f64 - 1.0))
                .round()
                .clamp(0.0, w as f64 - 1.0) as usize
        };
        let mut band = vec![' '; w];
        for (st, en, level) in &t.windows {
            let ch = match level.as_str() {
                "suspended" => 'S',
                "polite" => 'p',
                _ => '-',
            };
            let (lo, hi) = (col(*st), col(en.unwrap_or(z)));
            band.iter_mut().take(hi + 1).skip(lo).for_each(|c| *c = ch);
        }
        let mut runs = vec![' '; w];
        for (st, en, banked) in &t.runs {
            let ch = if *banked { '=' } else { '~' };
            let (lo, hi) = (col(*st), col(*en));
            runs.iter_mut().take(hi + 1).skip(lo).for_each(|c| *c = ch);
        }
        let critical = t
            .points
            .iter()
            .filter(|p| p.mem_pressure.is_some_and(|l| l >= 4))
            .count();
        lines.push(Line::from(vec![
            Span::styled(" throttle   ", Style::default().fg(th.dim)),
            Span::styled(
                band.into_iter().collect::<String>(),
                Style::default().fg(th.alarm),
            ),
            Span::styled("  S suspended, p polite", Style::default().fg(th.dim)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" runs       ", Style::default().fg(th.dim)),
            Span::styled(
                runs.into_iter().collect::<String>(),
                Style::default().fg(th.solved),
            ),
            Span::styled("  = banked, ~ owed", Style::default().fg(th.dim)),
        ]));
        lines.push(Line::from(Span::styled(
            format!(
                " {} samples over {}, {} under critical memory pressure",
                n,
                widget::duration(span_s.max(0.0) as u64),
                critical,
            ),
            Style::default().fg(th.dim),
        )));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_slots(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(th.dim))
        .title(Span::styled(
            format!(
                " RUNNING{:>w$} P-core ",
                s.p_cores,
                w = (area.width as usize).saturating_sub(18)
            ),
            Style::default().fg(th.structure),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();
    for slot in s.slots.iter().take(inner.height as usize) {
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
                let clock = if r.suspended {
                    "suspended".to_string()
                } else {
                    format!(
                        "{:>6.1}s/{:.0}s",
                        r.effective.as_secs_f64(),
                        r.budget.as_secs_f64()
                    )
                };
                let rho = r.rho.map_or(String::new(), |x| format!("  rho {x:.2}"));
                let rss = r.rss_mb.map_or(String::new(), |x| format!("  {x:.0} MB"));
                let err = r
                    .last_stderr
                    .as_ref()
                    .map_or(String::new(), |e| format!("  {e}"));
                Line::from(Span::styled(
                    widget::ellipsize(
                        &format!(
                            " {} {glyph} {:<20} {:<24} {clock}{rho}{rss}{err}",
                            slot.index,
                            widget::ellipsize(&r.board, 20),
                            format!("{}/{}", widget::ellipsize(&r.variant, 18), r.instance),
                        ),
                        inner.width as usize,
                    ),
                    Style::default().fg(colour),
                ))
            }
        };
        lines.push(line);
    }
    if !s.throughput.is_empty() && inner.height as usize > lines.len() {
        let w = (inner.width as usize).saturating_sub(24).clamp(6, 30);
        lines.push(Line::from(vec![
            Span::styled(" throughput ", Style::default().fg(th.dim)),
            Span::styled(
                widget::spark(th, &s.throughput, w),
                Style::default().fg(th.ember),
            ),
            Span::styled(
                format!(" {:.1}/min", s.throughput.last().copied().unwrap_or(0.0)),
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

fn draw_keys(f: &mut Frame, area: Rect, s: &Snapshot, th: &Theme) {
    let keys = match s.view {
        View::Grid => " j/k move   \u{21b5} board   t timeline   esc dismiss   q quit (stops the sweep; nothing banked is lost)",
        View::Board => " j/k move   \u{21b5} instance   o sort   b/esc back   q quit",
        View::Instance => " j/k next instance   b/esc back   q quit",
        View::Timeline => " t/b/esc back to the grid   q quit",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(keys, Style::default().fg(th.dim)))),
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
            "{} {}  {}/{} {:.0}%  cov {}  owed {}",
            super::banner::compact("crucible"),
            s.level.level.label(),
            p.done,
            p.total,
            p.frac() * 100.0,
            p.solved,
            p.owed
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
