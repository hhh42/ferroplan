//! The measuring half of `crucible`: process supervision, contention, and the
//! work queue.
//!
//! Split from `crucible-publish` so the pure transforms -- the ones with a
//! Python oracle to be checked against -- can be tested on any box in seconds
//! without linking a database, a terminal, or a platform.

pub mod artifact;
pub mod corpus;
pub mod db;
pub mod exec;
pub mod monitor;
pub mod platform;
pub mod sched;
pub mod sweep;
pub mod validate;
