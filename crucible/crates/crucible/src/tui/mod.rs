//! The dashboard.
//!
//! Render budget: a fixed tick, 4 fps, redrawing only on a state change or a
//! tick. The target is well under one percent of one core -- this program is
//! watching a benchmark, and a dashboard that perturbs its own measurement is
//! worse than no dashboard at all. Everything here is therefore cheap: no
//! gradients, no easing, no animation that is not a single character changing.
//!
//! The UI is a pure CONSUMER of state snapshots. It never holds a lock the
//! scheduler wants and never blocks a run; the worst a wedged terminal can do
//! is stop repainting.

pub mod app;
pub mod banner;
pub mod draw;
pub mod run;
pub mod theme;
pub mod widget;
