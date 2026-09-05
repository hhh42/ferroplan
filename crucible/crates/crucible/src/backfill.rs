//! `crucible backfill` -- sweep a TAG with the working tree's instrument.
//!
//! The rule `repo.rs::worktree_for` has carried since the harness was
//! written, now driven: the instrument (manifest, corpus, classifier) is
//! ALWAYS the working tree's, and only the ENGINE comes from the tag.
//! `backfill-air.sh` says why: checking out the old `benchmarks/` too "would
//! vary the INSTRUMENT as well as the engine, and then the delta means
//! nothing." So a backfill builds the tagged binary in a detached worktree
//! under crucible's own prefix and points the CURRENT sweep at it.
//!
//! What differs from a candidate sweep, and nothing else:
//! - the version gate is SKIPPED (a backfill is exactly the case where the
//!   version is old);
//! - the engine carries `tag = Some(..)`, so the database's engine facts can
//!   find it by name;
//! - the stage defaults to `benchmarks/air-<ver>/` (the `air-0.18.0/`,
//!   `air-0.19.0/`, `air-0.21.0/` convention), never the set's own stage --
//!   an old engine must not write where the candidate stages;
//! - a board the tag cannot run (`--mode optimal` before 0.19) is SKIPPED
//!   with a `feature-absent` pass row and zero measured rows, which the
//!   runner already does for any engine.

use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Opts<'a> {
    pub tag: &'a str,
    pub set: &'a str,
    /// Stage override; `None` is `benchmarks/air-<ver>/`.
    pub stage: Option<PathBuf>,
    pub dry_run: bool,
    pub max_passes: Option<u32>,
    pub no_db: bool,
}

/// `benchmarks/air-<ver>/` from `ff 0.18.0` -- the convention the hand-made
/// backfills already follow, so `crucible-differential.py --only air-0.19`
/// reads a driven backfill the way it reads the committed ones.
pub fn stage_for(ver: &str) -> PathBuf {
    let v = ver.trim().strip_prefix("ff ").unwrap_or(ver.trim());
    PathBuf::from("benchmarks").join(format!("air-{v}"))
}

fn git(repo: &Path, args: &[&str]) -> anyhow::Result<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

/// The tag must exist by name in the sweep repo; anything else is refused
/// with the name, not a stack of git noise.
fn verify_tag(repo: &Path, tag: &str) -> anyhow::Result<()> {
    let out = git(
        repo,
        &["rev-parse", "-q", "--verify", &format!("refs/tags/{tag}")],
    )?;
    if !out.status.success() {
        anyhow::bail!(
            "no tag {tag:?} in {} -- a backfill measures a tag that exists",
            repo.display()
        );
    }
    Ok(())
}

/// The detached worktree for `tag`, created if absent. Worktrees get their
/// own `target/`, so the candidate at `repo/target/release/ff` is untouched.
fn ensure_worktree(repo: &Path, worktree_dir: &Path, tag: &str) -> anyhow::Result<PathBuf> {
    let wt = crate::repo::worktree_for(worktree_dir, tag);
    if wt.join(".git").exists() {
        return Ok(wt);
    }
    std::fs::create_dir_all(worktree_dir)?;
    let out = git(
        repo,
        &[
            "worktree",
            "add",
            "--detach",
            &wt.display().to_string(),
            tag,
        ],
    )?;
    if !out.status.success() {
        anyhow::bail!(
            "git worktree add for {tag} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(wt)
}

/// Build the tag's planner in its worktree unless a binary is already there
/// (a rebuilt worktree is the same bytes; a pre-placed one is a test's).
fn ensure_binary(wt: &Path) -> anyhow::Result<PathBuf> {
    let bin = crate::repo::candidate_path(wt);
    if bin.exists() {
        return Ok(bin);
    }
    println!(
        "build   cargo build --release -p ferroplan-cli in {}",
        wt.display()
    );
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "ferroplan-cli"])
        .current_dir(wt)
        .status()
        .context("running cargo build in the tag worktree")?;
    if !status.success() {
        anyhow::bail!("the tag did not build ({status}); nothing measured");
    }
    Ok(bin)
}

/// Keep the newest `keep` worktrees under crucible's OWN prefix; never touch
/// anything outside it (the operator's `~/ferroplan-backfill-*` checkouts
/// share no prefix with it by construction, `worktrees_live_under_their_own_prefix`).
fn gc_worktrees(repo: &Path, worktree_dir: &Path, keep: usize, current: &str) {
    let Ok(rd) = std::fs::read_dir(worktree_dir) else {
        return;
    };
    let mut wts: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.path().join(".git").exists())
        .filter(|e| e.file_name() != current)
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    // Newest first; the current tag is always kept and not counted.
    wts.sort_by_key(|w| std::cmp::Reverse(w.0));
    for (_, p) in wts.into_iter().skip(keep.saturating_sub(1)) {
        if !p.starts_with(worktree_dir) {
            continue;
        }
        println!("gc      worktree {}", p.display());
        let _ = git(
            repo,
            &["worktree", "remove", "--force", &p.display().to_string()],
        );
    }
}

pub fn run(repo: &Path, cfg: &crate::config::Config, o: Opts<'_>) -> anyhow::Result<()> {
    verify_tag(repo, o.tag)?;
    let manifest = crate::load_manifest(repo)?;
    manifest
        .set(o.set)
        .with_context(|| format!("no set {:?} in the manifest", o.set))?;
    let wt = ensure_worktree(repo, &cfg.repo.worktree_dir, o.tag)?;
    let bin = ensure_binary(&wt)?;
    let mut engine = crate::repo::Engine::probe(&bin)?;
    engine.tag = Some(o.tag.to_string());
    let stage = o
        .stage
        .clone()
        .unwrap_or_else(|| repo.join(stage_for(&engine.ver)));
    println!("tag     {} at {}", o.tag, wt.display());
    println!("stage   {}", stage.display());
    let result = crate::sweep::run_engine(
        repo,
        cfg,
        crate::sweep::Opts {
            headless: false,
            set: o.set,
            require_version: None,
            quiet_only: false,
            dry_run: o.dry_run,
            max_passes: o.max_passes,
            no_db: o.no_db,
        },
        &manifest,
        engine,
        Some(stage),
    );
    if !o.dry_run {
        gc_worktrees(repo, &cfg.repo.worktree_dir, cfg.repo.keep_tags, o.tag);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_follows_the_hand_made_convention() {
        assert_eq!(
            stage_for("ff 0.18.0"),
            PathBuf::from("benchmarks/air-0.18.0")
        );
        assert_eq!(
            stage_for("0.21.0\n"),
            PathBuf::from("benchmarks/air-0.21.0")
        );
    }

    #[test]
    fn a_missing_tag_is_refused_by_name() {
        let dir = std::env::temp_dir().join("crucible-backfill-notag");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(&dir)
            .status()
            .unwrap()
            .success());
        let err = verify_tag(&dir, "v9.9.9").unwrap_err().to_string();
        assert!(err.contains("v9.9.9"), "{err}");
    }
}
