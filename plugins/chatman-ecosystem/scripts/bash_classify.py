#!/usr/bin/env python3
"""Canonical Bash-command classification for the Chatman ecosystem plugin.

This module is the single authority on whether a Bash command mutates the
repository (`is_mutation`) and whether it is a protected actuation
(`is_protected`). Three divergent copies of these patterns used to exist in
`loop.py`, `phase.py`, and `event-summary.py`; they disagreed, so `git push`
logged a ledger event but never collapsed the phase vector.

It imports only `re`, and must keep importing only the standard library:
`loop.py` is on the hook path and the hooks are this plugin's only mechanical
authority. Precedent: `_standing.py` imports only `enum`.

Word boundaries after every git subcommand alternation are load-bearing. Without
them, `git merge-base --is-ancestor` and `git branch --show-current` were
classified as repository mutations and blocked a legitimate push. A plain `\\b`
is not enough, because `-` is a non-word character and `commit\\b` therefore
matches `commit-graph`; the boundary used here is `(?![\\w-])`. `git
checkout-index` is genuinely mutating, so it gets its own explicit branch rather
than riding on `checkout`.
"""

from __future__ import annotations

import re

_SEGMENT_START = r"(?:^|[;&|]\s*)"

#: git subcommands that mutate the repository. Each is followed by
#: `(?![\w-])` so that `merge-base`, `merge-tree`, `merge-file`, and
#: `commit-graph` are not swept up as mutations.
_GIT_MUTATING = r"(?:add|commit|push|merge|rebase|reset|clean|checkout|switch|branch|tag)"

MUTATING_BASH = re.compile(
    _SEGMENT_START + r"(?:"
    r"git\s+checkout-index(?![\w-])|"
    rf"git\s+{_GIT_MUTATING}(?![\w-])|"
    r"gh\s+pr\s+(?:create|merge|close|edit)|"
    r"cargo\s+(?:fmt|fix|update|publish|install)|"
    r"npm\s+(?:publish|version|install)|"
    r"(?:rm|mv|cp|mkdir|touch|chmod|chown)\b|"
    r"(?:sed\s+-i|perl\s+-pi)|"
    r"(?:tee\s+|cat\s+[^|;&]*>)|"
    r"python(?:3)?\s+[^|;&]*(?:write|generate|update|patch))",
    re.IGNORECASE,
)

#: Bare file redirection (`>`, `>>`, `2>`, `1>`) is a mutation regardless of
#: command position, so it is not folded into `MUTATING_BASH`'s
#: `_SEGMENT_START`-anchored alternation: `echo hi > file` redirects after the
#: command's own arguments, not at a command boundary, so an anchored
#: alternative would never see it. Excluded: fd duplication (`2>&1`, `>&2`,
#: which redirect a stream to another stream, not to a file) and `2>/dev/null`
#: (the standard idiom for discarding stderr, not a real write) -- both are
#: read-only in effect despite the `>` in the text.
_BARE_REDIRECT = re.compile(r"\d*>>?(?!&)(?!\s*/dev/null\b)")

PROTECTED_BASH = re.compile(
    _SEGMENT_START + r"(?:"
    rf"git\s+(?:push|merge|rebase)(?![\w-])|"
    r"git\s+(?:reset\s+--hard|clean\s+-[a-z]*f)|"
    r"gh\s+pr\s+(?:create|merge|close)|"
    r"cargo\s+publish|npm\s+publish|"
    r"rm\s+-[^\n;&|]*r[^\n;&|]*f|"
    r"curl\b[^\n;&|]*(?:-X\s*(?:POST|PUT|PATCH|DELETE)|--request\s*(?:POST|PUT|PATCH|DELETE)))",
    re.IGNORECASE,
)

#: Flags that make any matched git subcommand read-only. Case-sensitive on
#: purpose: `-n` is a no-op preview, `-N` is not, and `git branch -D` must stay
#: a mutation while `git branch -d`'s neighbours in the listing set do not.
#: `--show-current` is matched as a prefix so that `git rebase
#: --show-current-patch` is covered too.
_READ_ONLY_ANY = re.compile(r"(?:^|\s)(?:--show-current|(?:--dry-run|-n)(?=[\s=]|$))")

#: Additional listing/query flags that are read-only for `branch` and `tag`.
_READ_ONLY_LISTING = re.compile(
    r"(?:^|\s)(?:-a|-r|-v|-l|--list|--contains|--no-contains|--merged|--no-merged"
    r"|--points-at|--format|--sort|--verify)(?=[\s=]|$)"
)

#: `git <subcommand>` extracted from a matched span, minus `checkout-index`,
#: which is genuinely mutating and never exempt.
_MATCHED_GIT_SUBCOMMAND = re.compile(rf"git\s+({_GIT_MUTATING}|push|merge|rebase)(?![\w-])", re.I)


def _segment(command: str, start: int) -> str:
    """The command text from `start` up to the next shell separator."""
    end = re.search(r"[;&|]", command[start:])
    return command[start : start + end.start()] if end else command[start:]


def _read_only_git(command: str, match: re.Match[str]) -> bool:
    """True when a matched mutating git subcommand is carrying read-only flags."""
    found = _MATCHED_GIT_SUBCOMMAND.search(match.group(0))
    if found is None:
        return False
    subcommand = found.group(1).lower()
    segment = _segment(command, match.start())
    if _READ_ONLY_ANY.search(segment):
        return True
    return subcommand in {"branch", "tag"} and bool(_READ_ONLY_LISTING.search(segment))


def _matches(pattern: re.Pattern[str], command: str) -> bool:
    for match in pattern.finditer(command):
        if not _read_only_git(command, match):
            return True
    return False


def is_mutation(command: str) -> bool:
    """True when `command` mutates the repository or the working tree."""
    return _matches(MUTATING_BASH, command) or bool(_BARE_REDIRECT.search(command))


def is_protected(command: str) -> bool:
    """True when `command` is a protected actuation (publication-class)."""
    return _matches(PROTECTED_BASH, command)
