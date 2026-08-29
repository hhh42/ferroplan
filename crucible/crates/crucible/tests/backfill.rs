//! `crucible backfill` end to end (0.26 F6 Part 2): a tag in a real git repo,
//! a worktree under crucible's own prefix, the working tree's manifest as the
//! instrument, the tag's planner as the engine -- and the stage under
//! `benchmarks/air-<ver>/`, never the set's own.
//!
//! The RED fixture was `crucible backfill --tag v0.18.0 --set cut25 --dry-run`
//! exiting with clap's "unrecognized subcommand"; this is the same shape on a
//! toy repo. The tag's planner is the fake one, pre-placed in the worktree so
//! the build step (a real `cargo build` in the tag) is not exercised here.

use std::path::{Path, PathBuf};
use std::process::Command;

const MANIFEST: &str = r#"
schema = 1
[corpus]
root = ".ipc-corpus"
domain_shared = "domain.pddl"
domain_per_instance = "domains/domain-{first}.pddl"
[defaults]
timeout_secs = 60
jobs = 2
threads = 1
mode = "auto"
mem_gb = 0.0
[track.toy]
ipcs = ["ipc-test"]
include = "-satisficing$"
[[board]]
id = "toy-board"
raw = "toy-board.jsonl"
md = "toy-board.md"
label = "toy"
competition = "test"
budget_secs = 5
track = "toy"
timeout_secs = 5
rebaselined_on = ["m5-air"]
[[board]]
id = "toy-proof"
raw = "toy-proof.jsonl"
md = "toy-proof.md"
label = "toy proof"
competition = "optimal"
budget_secs = 5
track = "toy"
mode = "optimal"
timeout_secs = 5
proof_track = true
rebaselined_on = ["m5-air"]
[[set]]
name = "toyset"
stage = "benchmarks/airtest"
requires_version = "9.9.9-candidate"
boards = ["toy-board", "toy-proof"]
"#;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?}: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("crucible-backfill-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A git repo holding the instrument, tagged; crucible's worktree prefix
/// beside it; the fake planner pre-placed as the tag's build.
fn make_repo(dir: &Path, tag: &str) -> PathBuf {
    let v = dir.join("benchmarks/.ipc-corpus/ipc-test/domains/toy-satisficing");
    std::fs::create_dir_all(v.join("instances")).unwrap();
    std::fs::write(v.join("domain.pddl"), "(define (domain toy))").unwrap();
    for n in [1u32, 2] {
        std::fs::write(
            v.join("instances").join(format!("instance-{n}.pddl")),
            format!("(define (problem p{n}))"),
        )
        .unwrap();
    }
    std::fs::write(dir.join("benchmarks/manifest.toml"), MANIFEST).unwrap();
    std::fs::write(dir.join(".gitignore"), "benchmarks/.ipc-corpus/\ntarget/\n").unwrap();
    git(dir, &["init", "-q"]);
    git(
        dir,
        &["-c", "user.email=t@t", "-c", "user.name=t", "add", "-A"],
    );
    git(
        dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "instrument",
        ],
    );
    git(dir, &["tag", tag]);

    let worktrees = dir.join("worktrees");
    std::fs::write(
        dir.join("config.toml"),
        format!(
            "[db]\ndir = {:?}\n[repo]\nworktree_dir = {:?}\n",
            dir.join("db").display().to_string(),
            worktrees.display().to_string()
        ),
    )
    .unwrap();
    // The worktree crucible would create, created here so the tag's "build"
    // can be the fake planner instead of a real cargo build.
    let wt = worktrees.join(tag);
    std::fs::create_dir_all(&worktrees).unwrap();
    git(
        dir,
        &[
            "worktree",
            "add",
            "--detach",
            &wt.display().to_string(),
            tag,
        ],
    );
    let bin = wt.join("target/release/ff");
    std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
    let mut src = std::env::current_exe().unwrap();
    src.pop();
    src.pop();
    src.push("fakeff");
    std::fs::copy(&src, &bin).unwrap_or_else(|e| panic!("{}: {e}", src.display()));
    // The candidate slot stays EMPTY: a backfill must never touch it.
    wt
}

fn crucible(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_crucible"))
        .arg("--repo")
        .arg(repo)
        .args(args)
        .env("CRUCIBLE_CONFIG", repo.join("config.toml"))
        .env("FAKEFF_VERSION", "0.18.0")
        .env_remove("CRUCIBLE_NO_DB")
        .output()
        .expect("crucible runs")
}

#[test]
fn a_backfill_plans_the_tag_under_its_own_stage_and_skips_the_version_gate() {
    let repo = tmp("plan");
    let wt = make_repo(&repo, "v0.18.0");

    let out = crucible(
        &repo,
        &[
            "backfill",
            "--tag",
            "v0.18.0",
            "--set",
            "toyset",
            "--dry-run",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    // The set demands 9.9.9-candidate; the tag reports 0.18.0; no gate fired.
    assert!(stdout.contains("engine  ff 0.18.0"), "{stdout}");
    assert!(
        stdout.contains(&format!("tag     v0.18.0 at {}", wt.display())),
        "{stdout}"
    );
    assert!(
        stdout.contains(&format!(
            "stage   {}",
            repo.join("benchmarks/air-0.18.0").display()
        )),
        "the stage is the version's, not the set's:\n{stdout}"
    );
    assert!(!stdout.contains("airtest"), "{stdout}");
    assert!(stdout.contains("toy-board"), "{stdout}");
    assert!(stdout.contains("dry run -- nothing measured"), "{stdout}");
    // Nothing built or written in the candidate's slot.
    assert!(!repo.join("target/release/ff").exists());
}

#[test]
fn a_missing_tag_is_refused_by_name() {
    let repo = tmp("notag");
    make_repo(&repo, "v0.18.0");
    let out = crucible(
        &repo,
        &[
            "backfill",
            "--tag",
            "v0.17.0",
            "--set",
            "toyset",
            "--dry-run",
        ],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no tag \"v0.17.0\""), "{stderr}");
}
