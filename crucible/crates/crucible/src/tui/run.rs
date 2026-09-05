//! Owning the terminal, and giving it back.
//!
//! Two properties matter more than anything this file draws:
//!
//! 1. **The terminal is always restored.** Raw mode and the alternate screen
//!    are process-global state. If crucible panics -- or is killed -- and does
//!    not put them back, the operator is left with a shell that does not echo.
//!    So restoration happens in a `Drop` guard AND in a panic hook, not at the
//!    end of a happy path.
//! 2. **The UI can never stall the sweep.** It reads a snapshot and draws. It
//!    holds nothing the scheduler wants. If the terminal wedges, measurements
//!    carry on and the screen simply stops moving.
//!
//! Render budget: a fixed tick, 4 fps by default, and a redraw only when the
//! tick fires or state actually changed. This program is watching a benchmark;
//! a dashboard that perturbs its own measurement is worse than no dashboard.

use super::app::{Snapshot, View};
use super::{draw, theme::Theme};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;
use std::io;
use std::time::{Duration, Instant};

/// Restores the terminal however we leave.
pub struct TerminalGuard;

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen, crossterm::cursor::Hide)?;
        // A panic must not leave a shell without echo. The hook runs before
        // unwinding reaches any Drop, so it is the belt to Drop's braces.
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = restore();
            prev(info);
        }));
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore();
    }
}

fn restore() -> io::Result<()> {
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show)?;
    disable_raw_mode()
}

/// How long an ordinary toast stays on screen.
pub const TOAST_DWELL: Duration = Duration::from_secs(4);

/// What a keypress asked for. The loop translates keys; the caller decides what
/// they mean, so this file never has to know what a sweep is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Stop the sweep the way ^C would: the running child is cancelled and
    /// reaped, everything banked stays banked.
    Quit,
    /// The selection or the view changed; nothing for the sweep to do.
    Redraw,
    /// The instance view wants its detail read from the database.
    NeedDetail,
}

/// Keys to intent. There is deliberately no re-run key (`crucible-spec.md`
/// R2.4): if the automatic retry is right there is nothing to press; if it
/// is wrong the fix is the referee.
pub fn action_for(k: KeyEvent, s: &mut Snapshot) -> Option<Action> {
    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
        return Some(Action::Quit);
    }
    let detail = |s: &Snapshot| {
        if s.view == View::Instance {
            Action::NeedDetail
        } else {
            Action::Redraw
        }
    };
    match k.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            s.move_selection(1);
            Some(detail(s))
        }
        KeyCode::Char('k') | KeyCode::Up => {
            s.move_selection(-1);
            Some(detail(s))
        }
        KeyCode::Char('g') => {
            s.jump(false);
            Some(detail(s))
        }
        KeyCode::Char('G') => {
            s.jump(true);
            Some(detail(s))
        }
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
            s.enter();
            Some(detail(s))
        }
        KeyCode::Char('b') | KeyCode::Char('h') | KeyCode::Left => {
            s.back();
            Some(Action::Redraw)
        }
        KeyCode::Esc => {
            if s.toasts.is_empty() {
                s.back();
            } else {
                s.dismiss_toasts();
            }
            Some(Action::Redraw)
        }
        KeyCode::Char('t') => {
            s.toggle_timeline();
            Some(Action::Redraw)
        }
        KeyCode::Char('o') => {
            s.cycle_sort();
            Some(Action::Redraw)
        }
        _ => None,
    }
}

/// Drive the dashboard until the operator quits or `next` says the sweep ended.
///
/// `next` is called once per tick and hands back the current snapshot; it is
/// where the caller reads whatever the scheduler last published. `on_action`
/// receives operator intent. Neither is allowed to block for long -- if the
/// caller wants to do work, it should do it elsewhere and let the next tick
/// pick up the result.
pub fn run<N, A>(fps: u32, banner_text: &str, mut next: N, mut on_action: A) -> io::Result<()>
where
    N: FnMut(&Snapshot) -> Option<Snapshot>,
    A: FnMut(Action, &Snapshot),
{
    let _guard = TerminalGuard::enter()?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let theme = Theme::forge();
    let tick = Duration::from_millis(1000 / fps.clamp(1, 30) as u64);

    let Some(mut snap) = next(&Snapshot::default()) else {
        return Ok(());
    };
    let mut last_draw = Instant::now();
    term.draw(|f| draw::draw(f, &snap, &theme, banner_text))?;

    loop {
        // Wait for a key, but never past the next tick -- so the clock, the
        // slot timers and the throughput sparkline keep moving with no input.
        let wait = tick.saturating_sub(last_draw.elapsed());
        let mut dirty = false;
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == event::KeyEventKind::Press => {
                    match action_for(k, &mut snap) {
                        Some(Action::Quit) => return Ok(()),
                        Some(Action::Redraw) => dirty = true,
                        Some(a) => {
                            on_action(a, &snap);
                            dirty = true;
                        }
                        None => {}
                    }
                }
                Event::Resize(_, _) => dirty = true,
                _ => {}
            }
        }

        if last_draw.elapsed() >= tick {
            match next(&snap) {
                Some(mut s) => {
                    // The feed knows nothing about the cursor; navigation
                    // state carries across refreshes.
                    s.view = snap.view;
                    s.sel_board = snap.sel_board;
                    s.sel_inst = snap.sel_inst;
                    s.sort = snap.sort;
                    if s.detail.is_none() {
                        s.detail = snap.detail.take();
                    }
                    snap = s;
                }
                // The sweep is over and there is nothing left to watch.
                None => return Ok(()),
            }
            // Ordinary toasts dwell for four seconds. A REGRESSION toast is
            // sticky and does not expire at all -- it is the one thing on this
            // screen nobody may miss, so it has to be dismissed rather than
            // waited out.
            snap.expire_toasts(TOAST_DWELL);
            dirty = true;
            last_draw = Instant::now();
        }
        if dirty {
            term.draw(|f| draw::draw(f, &snap, &theme, banner_text))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::*;

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn snap() -> Snapshot {
        let cells = |n: usize| (0..n).map(|_| InstanceCell::default()).collect();
        Snapshot {
            boards: vec![
                BoardRow {
                    id: "a".into(),
                    cells: cells(3),
                    ..Default::default()
                },
                BoardRow {
                    id: "b".into(),
                    cells: cells(2),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn vim_keys_and_arrows_agree() {
        let mut a = snap();
        let mut b = snap();
        action_for(key('j'), &mut a);
        action_for(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut b);
        assert_eq!(a.sel_board, b.sel_board);
        assert_eq!(a.sel_board, 1);
    }

    #[test]
    fn enter_drills_and_back_climbs() {
        let mut s = snap();
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut s),
            Some(Action::Redraw)
        );
        assert_eq!(s.view, View::Board);
        assert_eq!(
            action_for(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut s),
            Some(Action::NeedDetail),
            "the instance view asks for its detail"
        );
        assert_eq!(s.view, View::Instance);
        action_for(key('b'), &mut s);
        assert_eq!(s.view, View::Board);
        action_for(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut s);
        assert_eq!(s.view, View::Grid);
    }

    #[test]
    fn g_and_shift_g_jump_to_the_ends() {
        let mut s = snap();
        action_for(key('G'), &mut s);
        assert_eq!(s.sel_board, 1);
        action_for(key('g'), &mut s);
        assert_eq!(s.sel_board, 0);
    }

    #[test]
    fn t_toggles_the_timeline_and_o_cycles_the_sort() {
        let mut s = snap();
        action_for(key('t'), &mut s);
        assert_eq!(s.view, View::Timeline);
        action_for(key('t'), &mut s);
        assert_eq!(s.view, View::Grid);
        action_for(key('o'), &mut s);
        assert_eq!(s.sort, Sort::Corpus, "sort is a board-view thing");
        s.enter();
        action_for(key('o'), &mut s);
        assert_eq!(s.sort, Sort::Time);
    }

    /// Esc clears the ordinary toasts first; with none showing it climbs.
    #[test]
    fn esc_dismisses_toasts_before_it_climbs() {
        let mut s = snap();
        s.enter();
        s.toasts.push(Toast {
            text: "resumed".into(),
            kind: LogKind::Info,
            sticky: false,
            age: Duration::ZERO,
        });
        action_for(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut s);
        assert!(s.toasts.is_empty());
        assert_eq!(s.view, View::Board);
        action_for(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut s);
        assert_eq!(s.view, View::Grid);
    }

    #[test]
    fn ctrl_c_quits_as_well_as_q() {
        let mut s = snap();
        assert_eq!(action_for(key('q'), &mut s), Some(Action::Quit));
        assert_eq!(
            action_for(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &mut s
            ),
            Some(Action::Quit)
        );
    }

    /// The re-run key is gone by decision, and an unbound key does nothing.
    #[test]
    fn there_is_no_rerun_key_and_unbound_keys_do_nothing() {
        let mut s = snap();
        assert_eq!(action_for(key('r'), &mut s), None);
        assert_eq!(action_for(key('x'), &mut s), None);
        assert_eq!(s.view, View::Grid);
    }
}
