//! Watching the box: what else is running, and what to do about it.

pub mod games;
pub mod sample;
pub mod throttle;

pub use games::{GameRules, Proc};
pub use sample::{Sample, SAMPLE_CLEAN_PCPU};
pub use throttle::{Config, GameState, Level, Reason, Throttle, Transition};
