//! What a board leaves behind on disk.
//!
//! A sweep writes three files per board and each answers a different question
//! later: the `.jsonl` raw is the per-instance evidence
//! (`crucible_publish::write_row`), `conditions` is what else the machine was
//! doing while that evidence was collected, and the `.md` is the human summary
//! whose last line the sweep drivers parse.
//!
//! They live together because they are read together. A coverage number is only
//! evidence alongside the conditions it was measured under, and this project's
//! expensive mistakes have all been cases of one of the three being trusted
//! without the others.

pub mod board_md;
pub mod conditions;
