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

use super::app::Snapshot;
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
    Quit,
    /// Pause or resume the sweep.
    TogglePause,
    /// Operator-forced suspension: get off the machine now.
    ForceSuspend,
    /// Re-measure the selected item, discarding its current row.
    Rerun,
    ShowDiff,
    FilterLog,
    Search,
    Redraw,
}

/// Vim keys and arrows both, because muscle memory is not negotiable.
pub fn action_for(k: KeyEvent, s: &mut Snapshot) -> Option<Action> {
    if k.modifiers.contains(KeyModifiers::CONTROL) && matches!(k.code, KeyCode::Char('c')) {
        return Some(Action::Quit);
    }
    match k.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => {
            s.move_selection(1);
            Some(Action::Redraw)
        }
        KeyCode::Char('k') | KeyCode::Up => {
            s.move_selection(-1);
            Some(Action::Redraw)
        }
        KeyCode::Char('g') => {
            s.selected = 0;
            Some(Action::Redraw)
        }
        KeyCode::Char('G') => {
            s.selected = s.visible_tracks().len().saturating_sub(1);
            Some(Action::Redraw)
        }
        KeyCode::Enter
        | KeyCode::Char('l')
        | KeyCode::Right
        | KeyCode::Char('h')
        | KeyCode::Left => {
            s.toggle_selected();
            Some(Action::Redraw)
        }
        KeyCode::Char('z') => {
            s.collapse_finished();
            Some(Action::Redraw)
        }
        KeyCode::Esc => {
            s.dismiss_toasts();
            Some(Action::Redraw)
        }
        KeyCode::Char('p') => Some(Action::TogglePause),
        KeyCode::Char('s') => Some(Action::ForceSuspend),
        KeyCode::Char('r') => Some(Action::Rerun),
        KeyCode::Char('d') => Some(Action::ShowDiff),
        KeyCode::Char('f') => Some(Action::FilterLog),
        KeyCode::Char('/') => Some(Action::Search),
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
    N: FnMut() -> Option<Snapshot>,
    A: FnMut(Action, &Snapshot),
{
    let _guard = TerminalGuard::enter()?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let theme = Theme::forge();
    let tick = Duration::from_millis(1000 / fps.clamp(1, 30) as u64);

    let Some(mut snap) = next() else {
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
            match next() {
                Some(s) => snap = s,
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
        Snapshot {
            tracks: vec![
                TrackProgress {
                    name: "A".into(),
                    done: 1,
                    total: 2,
                    solved: 1,
                    delta: None,
                    domains: vec![],
                    expanded: false,
                    finished: true,
                },
                TrackProgress {
                    name: "B".into(),
                    done: 1,
                    total: 2,
                    solved: 1,
                    delta: None,
                    domains: vec![DomainProgress {
                        name: "d".into(),
                        solved: 0,
                        total: 1,
                        regressions: 0,
                    }],
                    expanded: false,
                    finished: false,
                },
            ],
            ..Default::default()
        }
    }

    /// Vim keys and arrows both. Half a keymap is worse than either.
    #[test]
    fn vim_keys_and_arrows_agree() {
        let mut a = snap();
        let mut b = snap();
        action_for(key('j'), &mut a);
        action_for(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut b);
        assert_eq!(a.selected, b.selected);
        assert_eq!(a.selected, 1);
    }

    #[test]
    fn enter_expands_the_selected_track() {
        let mut s = snap();
        s.selected = 1;
        assert_eq!(s.visible_tracks().len(), 2);
        action_for(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut s);
        assert_eq!(s.visible_tracks().len(), 3, "its one domain is now shown");
    }

    #[test]
    fn g_and_shift_g_jump_to_the_ends() {
        let mut s = snap();
        action_for(
            KeyEvent::new(KeyCode::Char('G'), KeyModifiers::NONE),
            &mut s,
        );
        assert_eq!(s.selected, 1);
        action_for(key('g'), &mut s);
        assert_eq!(s.selected, 0);
    }

    /// Esc clears the ordinary toasts. A sticky regression toast is cleared
    /// too -- but only because the operator explicitly asked, which is the
    /// difference between dismissing and never seeing.
    #[test]
    fn esc_dismisses_toasts() {
        let mut s = snap();
        s.toasts.push(Toast {
            text: "x".into(),
            kind: LogKind::Info,
            sticky: false,
            age: Duration::ZERO,
        });
        action_for(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut s);
        assert!(s.toasts.is_empty());
    }

    #[test]
    fn ctrl_c_quits_as_well_as_q() {
        let mut s = snap();
        assert_eq!(
            action_for(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &mut s
            ),
            Some(Action::Quit)
        );
        assert_eq!(action_for(key('q'), &mut s), Some(Action::Quit));
    }

    /// Operator intent is REPORTED, never acted on here -- this file must not
    /// know what a sweep is, or the dashboard could stall one.
    #[test]
    fn control_keys_report_intent_rather_than_acting() {
        let mut s = snap();
        assert_eq!(action_for(key('p'), &mut s), Some(Action::TogglePause));
        assert_eq!(action_for(key('s'), &mut s), Some(Action::ForceSuspend));
        assert_eq!(action_for(key('r'), &mut s), Some(Action::Rerun));
    }

    #[test]
    fn an_unbound_key_does_nothing_at_all() {
        let mut s = snap();
        let before = s.selected;
        assert_eq!(action_for(key('Q'), &mut s), None);
        assert_eq!(s.selected, before);
    }
}
