"""Falsifiers for the canonical Bash mutation classifier.

Three defects motivated this table. First, three copies of `MUTATING_BASH`
disagreed, so `git push` logged a ledger event but never collapsed the phase
vector. Second, no git subcommand alternation except `rm\\b` carried a trailing
word boundary, so `git merge-base --is-ancestor` and `git branch --show-current`
were classified as repository mutations and blocked a legitimate push. Third
(CE-GALL-35), bare shell redirection (`echo hi > file`) was not classified as
a mutation at all -- only `tee`/`cat ... >` were -- which let a non-editor
agent write a file through `Bash` undetected.
"""

from __future__ import annotations

import json

import pytest

MUTATIONS = [
    'git commit -m "x"',
    "git commit --amend",
    "git push",
    "git push origin main",
    "git merge feature",
    "git rebase main",
    "git reset --hard",
    "git checkout -- .",
    "git checkout-index -a",
    "git add .",
    "git clean -fd",
    "git branch -d old",
    "git branch -D old",
    "git tag v1",
    "git tag -d v1",
    "rm -rf x",
    "mv a b",
    "cp a b",
    "npm publish",
    "cargo publish",
    "gh pr create",
    "echo hi > /tmp/f.txt",
    "echo hi >> /tmp/f.txt",
    "cmd 2> err.log",
    'awk "{print}" foo > bar',
]

NON_MUTATIONS = [
    "git commit --dry-run",
    "git commit-graph verify",
    "git push --dry-run",
    "git push -n",
    "git merge-base main HEAD",
    "git merge-tree a b",
    "git merge-file -p a b c",
    "git rebase --show-current-patch",
    "git add --dry-run .",
    "git add -n .",
    "git clean -n",
    "git clean --dry-run",
    "git branch --show-current",
    "git branch -a",
    "git branch -r",
    "git branch -v",
    "git branch -l",
    "git branch --contains HEAD",
    "git branch --merged",
    "git branch --points-at HEAD",
    "git branch --format=%(refname)",
    "git tag -l",
    "git tag -v v1",
    "git tag -n",
    "git tag --contains HEAD",
    "git tag --points-at HEAD",
    "git status",
    "git log --oneline",
    "git diff",
    "rmdir x",
    "cmd 2>&1",
    "find . -name '*.py' 2>/dev/null",
    "cmd > /dev/null 2>&1",
    "cmd 2>&1 | head -50",
]


@pytest.mark.parametrize("command", MUTATIONS)
def test_is_mutation_true(command):
    import bash_classify

    assert bash_classify.is_mutation(command) is True


@pytest.mark.parametrize("command", NON_MUTATIONS)
def test_is_mutation_false(command):
    import bash_classify

    assert bash_classify.is_mutation(command) is False


@pytest.mark.parametrize(
    "command", ["git push", "gh pr create", "cargo publish", "npm publish"]
)
def test_phase_agrees_with_loop_on_publication_class(command):
    """Regression test for the three-copy divergence.

    `phase.py` was missing all four of these, so a publication-class command
    logged a ledger event without collapsing the phase vector.
    """
    import phase

    assert phase.is_mutation({"tool_name": "Bash", "tool_input": {"command": command}})


def test_protected_boundary():
    import bash_classify

    assert bash_classify.is_protected("git push origin main") is True
    assert bash_classify.is_protected("git push --dry-run") is False
    assert bash_classify.is_protected("git merge-base main HEAD") is False
    assert bash_classify.is_protected("git branch --show-current") is False
    assert bash_classify.is_protected("git reset --hard") is True


def test_read_only_bash_is_not_denied_by_loop_hook(hook_event, run_script, tmp_path):
    """End-to-end: a read-only command must not produce a PreToolUse deny."""
    payload = hook_event(
        "PreToolUse",
        tool_name="Bash",
        tool_input={"command": "git merge-base --is-ancestor main HEAD"},
        cwd=str(tmp_path),
    )
    proc = run_script("loop.py", "hook", stdin=payload)
    assert proc.returncode == 0, proc.stderr
    if proc.stdout.strip():
        emitted = json.loads(proc.stdout)
        decision = emitted.get("hookSpecificOutput", {}).get("permissionDecision")
        assert decision != "deny", proc.stdout


def test_event_summary_counts_git_push_as_mutating(hook_event, run_script, tmp_path):
    """Regression test for the third divergent copy: event-summary.py.

    Before this fix, event-summary.py's own MUTATING_BASH pattern (unlike
    loop.py's) had no `git push` alternative, so a batch containing a `git
    push` call was summarized with zero mutation candidates.
    """
    payload = hook_event(
        "PostToolBatch",
        tool_calls=[{"tool_name": "Bash", "tool_input": {"command": "git push origin main"}}],
        cwd=str(tmp_path),
    )
    proc = run_script("event-summary.py", stdin=payload, cwd=tmp_path)
    assert proc.returncode == 0, proc.stderr
    emitted = json.loads(proc.stdout)
    context = emitted["hookSpecificOutput"]["additionalContext"]
    assert "1 mutation candidate(s)" in context, context
