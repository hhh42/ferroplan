#!/usr/bin/env python3
"""Apply one admitted plan step inside an isolated Git worktree.

Gall Checkpoint 11 ("Isolated Source Manufacture") requires that manufacture:

* creates a worktree at an exact, recorded base commit;
* refuses a diff that touches paths outside the admitted scope;
* never mutates the main working tree;
* cleans up the worktree deterministically, win or lose;
* leaves a real commit behind for a caller-driven fast-forward merge.

This script deliberately does NOT merge the result back into the caller's
branch itself. Merging is a real repository mutation, and Checkpoint 4/5's
`PostToolUse` hook only fires when the harness's own `Bash` tool runs the
merge -- not when a `git merge` subprocess is spawned from inside this
script. So "mutation emits a new observation candidate" and "advanced
standing collapses after manufacture" are proven by the caller running
`git merge --ff-only <branch>` themselves as an actual tool call, using the
branch this script leaves behind. This script's job stops at: isolate,
apply, scope-check, build/test, commit, report, clean up.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

DIFF_PATH_RE = re.compile(r"^\+\+\+ b/(?P<path>.+)$", re.MULTILINE)


def run(cmd: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=str(cwd) if cwd else None,
        capture_output=True,
        text=True,
    )


def repo_root() -> Path:
    result = run(["git", "rev-parse", "--show-toplevel"])
    if result.returncode != 0:
        raise SystemExit(f"not a git repository: {result.stderr.strip()}")
    return Path(result.stdout.strip())


def resolve_base(root: Path, base: str | None) -> str:
    result = run(["git", "rev-parse", base or "HEAD"], cwd=root)
    if result.returncode != 0:
        raise SystemExit(f"cannot resolve --base `{base}`: {result.stderr.strip()}")
    return result.stdout.strip()


def diff_paths(diff_text: str) -> list[str]:
    paths = DIFF_PATH_RE.findall(diff_text)
    if not paths:
        raise SystemExit("diff declares no `+++ b/...` target paths; nothing to scope-check")
    return paths


def refuse_out_of_scope(paths: list[str], allow_globs: list[str]) -> None:
    if not allow_globs:
        raise SystemExit(
            "at least one --allow-path is required; manufacture without an "
            "admitted scope is refused by construction"
        )
    violations = [
        path
        for path in paths
        if not any(fnmatch.fnmatch(path, pattern) for pattern in allow_globs)
    ]
    if violations:
        raise SystemExit(
            "manufacture refused: diff touches paths outside the admitted scope: "
            + ", ".join(sorted(violations))
        )


def clean_worktree(root: Path, worktree_dir: Path) -> None:
    run(["git", "worktree", "remove", "--force", str(worktree_dir)], cwd=root)
    if worktree_dir.exists():
        shutil.rmtree(worktree_dir, ignore_errors=True)


def apply_in_worktree(args: argparse.Namespace) -> int:
    root = repo_root()
    diff_path = Path(args.diff).resolve()
    if not diff_path.is_file():
        raise SystemExit(f"--diff `{diff_path}` does not exist")
    diff_text = diff_path.read_text(encoding="utf-8")

    paths = diff_paths(diff_text)
    refuse_out_of_scope(paths, args.allow_path)

    before_status = run(["git", "status", "--porcelain"], cwd=root)
    base_commit = resolve_base(root, args.base)

    branch = args.branch
    existing_branch = run(["git", "rev-parse", "--verify", "--quiet", branch], cwd=root)
    if existing_branch.returncode == 0:
        raise SystemExit(
            f"branch `{branch}` already exists; pick a fresh --branch name "
            "(manufacture never overwrites an existing ref)"
        )

    worktree_dir = Path(tempfile.mkdtemp(prefix="manufacture-worktree-"))
    worktree_dir.rmdir()  # `git worktree add` requires the target not to exist yet.

    manifest: dict[str, object] = {
        "schema": "urn:chatman:manufacture-worktree-report:v1",
        "base_commit": base_commit,
        "branch": branch,
        "diff": str(diff_path),
        "diff_paths": paths,
        "allow_path": args.allow_path,
        "test_cmd": args.test_cmd,
        "worktree_dir": str(worktree_dir),
        "started_at_unix_ms": int(time.time() * 1000),
    }

    added = run(["git", "worktree", "add", "--detach", str(worktree_dir), base_commit], cwd=root)
    if added.returncode != 0:
        manifest.update(status="worktree-create-failed", error=added.stderr.strip())
        print(json.dumps(manifest, sort_keys=True, indent=2))
        return 2
    manifest["worktree_created"] = True

    try:
        checkout = run(["git", "checkout", "-b", branch], cwd=worktree_dir)
        if checkout.returncode != 0:
            manifest.update(status="branch-create-failed", error=checkout.stderr.strip())
            print(json.dumps(manifest, sort_keys=True, indent=2))
            return 2

        applied = run(["git", "apply", "--index", str(diff_path)], cwd=worktree_dir)
        if applied.returncode != 0:
            manifest.update(status="apply-failed", error=applied.stderr.strip())
            print(json.dumps(manifest, sort_keys=True, indent=2))
            return 2

        commit = run(
            ["git", "commit", "-m", f"manufacture: {branch}"],
            cwd=worktree_dir,
        )
        if commit.returncode != 0:
            manifest.update(status="commit-failed", error=commit.stderr.strip())
            print(json.dumps(manifest, sort_keys=True, indent=2))
            return 2
        commit_sha = run(["git", "rev-parse", "HEAD"], cwd=worktree_dir).stdout.strip()
        manifest["commit"] = commit_sha

        if args.test_cmd:
            tested = run(["bash", "-lc", args.test_cmd], cwd=worktree_dir)
            manifest["test_exit_code"] = tested.returncode
            manifest["test_stdout"] = tested.stdout[-4000:]
            manifest["test_stderr"] = tested.stderr[-4000:]
            if tested.returncode != 0:
                manifest.update(status="test-failed")
                print(json.dumps(manifest, sort_keys=True, indent=2))
                return 1

        after_status = run(["git", "status", "--porcelain"], cwd=root)
        manifest["main_tree_untouched"] = before_status.stdout == after_status.stdout
        manifest.update(status="ready-to-merge")
        print(json.dumps(manifest, sort_keys=True, indent=2))
        return 0
    finally:
        clean_worktree(root, worktree_dir)
        manifest["worktree_cleaned_up"] = not worktree_dir.exists()


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    sub = root.add_subparsers(dest="command", required=True)
    apply_cmd = sub.add_parser(
        "apply", help="apply a diff inside an isolated worktree and report the result"
    )
    apply_cmd.add_argument("--diff", required=True, help="unified diff (git diff format) to apply")
    apply_cmd.add_argument(
        "--allow-path",
        action="append",
        default=[],
        help="fnmatch glob a changed path must match; repeatable; required",
    )
    apply_cmd.add_argument("--branch", required=True, help="new branch name for the manufactured commit")
    apply_cmd.add_argument("--base", help="base commit/ref; defaults to HEAD")
    apply_cmd.add_argument("--test-cmd", help="shell command to run inside the worktree after commit")
    return root


def main() -> int:
    args = parser().parse_args()
    if args.command == "apply":
        return apply_in_worktree(args)
    raise SystemExit(f"unsupported command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
