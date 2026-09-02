#!/usr/bin/env python3
"""Keep the release notes short in BOTH places they accumulate.

1. CHANGELOG.md -> `[Unreleased]` plus the newest KEEP releases; everything
   older moves verbatim to CHANGELOG-ARCHIVE.md.
2. README.md -> the newest KEEP "What's new in X" blockquotes between the
   WHATSNEW markers; older ones are dropped with a link to the changelog.

Why: the changelog had grown to 22 releases / 1,919 lines and the README
carried SIXTEEN "What's new" blocks — ~308 lines, 45% of the front page — so
the thing a reader actually wants (what is this, is it any good, what changed
lately) was buried under a year of history. Changelog history is not deleted,
it moves; the README blocks are curated summaries whose fuller record is the
changelog entry for the same version, so those are dropped and linked.

Run it as part of the release process (RELEASING.md), after the new sections
are written and before publishing. Idempotent: running it twice is a no-op.

    python3 scripts/release-notes-roll.py [--keep N] [--check]

--check exits non-zero if a roll is needed, for a pre-flight gate.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LIVE = os.path.join(ROOT, "CHANGELOG.md")
ARCHIVE = os.path.join(ROOT, "CHANGELOG-ARCHIVE.md")
KEEP = int(sys.argv[sys.argv.index("--keep") + 1]) if "--keep" in sys.argv else 2
CHECK = "--check" in sys.argv

ARCHIVE_HEADER = """# Changelog archive

Releases older than the two most recent. The live
[`CHANGELOG.md`](CHANGELOG.md) keeps `[Unreleased]` plus the newest two;
everything before that lands here, newest first, verbatim and unedited.

`publish.sh` reads release notes from BOTH files, so archiving a version
never breaks `--release-only <old-version>`.
"""

SECTION = re.compile(r"^## \[", re.M)


def split_sections(text):
    """-> (preamble, [(header_line, body_including_header), ...])"""
    starts = [m.start() for m in SECTION.finditer(text)]
    if not starts:
        return text, []
    preamble = text[: starts[0]]
    out = []
    for i, s in enumerate(starts):
        e = starts[i + 1] if i + 1 < len(starts) else len(text)
        chunk = text[s:e]
        out.append((chunk.splitlines()[0], chunk))
    return preamble, out


GH_BLOB = "https://github.com/hhh42/ferroplan/blob/main/"
README = os.path.join(ROOT, "README.md")
WN_BEGIN = "<!-- WHATSNEW:BEGIN"
WN_END = "<!-- WHATSNEW:END -->"
WN_BLOCK = re.compile(r"^> \*\*What's new in ", re.M)


def roll_readme(keep, check):
    """Trim README's What's-new blockquotes to the newest `keep`."""
    if not os.path.exists(README):
        return 0
    text = open(README).read()
    i, j = text.find(WN_BEGIN), text.find(WN_END)
    if i < 0 or j < 0:
        return 0
    head_end = text.find("-->", i) + 3
    region = text[head_end:j]
    starts = [m.start() for m in WN_BLOCK.finditer(region)]
    if len(starts) <= keep:
        print(f"README: {len(starts)} What's-new block(s), keep={keep} — nothing to trim")
        return 0
    if check:
        print(f"README: would drop {len(starts) - keep} What's-new block(s)")
        return 1
    kept = region[:starts[keep]].rstrip()
    # Absolute: README ships as the crate README, and crates.io resolves
    # relative links against crates/ferroplan/ (see standings.py).
    tail = ("\n\nEarlier releases are summarised in the "
            f"[changelog]({GH_BLOB}CHANGELOG.md) and its "
            f"[archive]({GH_BLOB}CHANGELOG-ARCHIVE.md).\n")
    with open(README, "w") as f:
        f.write(text[:head_end] + "\n" + kept + tail + text[j:])
    print(f"README: dropped {len(starts) - keep} What's-new block(s), kept {keep}")
    return 0


REL_LINK = re.compile(r"\[[^\]]*\]\((?!https?:|#|mailto:)([^)]+)\)")


def check_absolute_links():
    """README and the newest changelog section must use ABSOLUTE links.

    Both get lifted OUT of the repo, where relative links break in ways that
    are invisible from GitHub: README ships as the crate README
    (`readme = "../../README.md"`), and crates.io/docs.rs resolve relatives
    against crates/ferroplan/ — so `STANDINGS.md` 404s as
    crates/ferroplan/STANDINGS.md. Changelog sections become GitHub Release
    bodies, which do not resolve repo-relative links either. Caught in the
    wild once; a gate is cheaper than another 404.
    """
    bad = []
    if os.path.exists(README):
        for m in REL_LINK.finditer(open(README).read()):
            bad.append(("README.md", m.group(1)))
    text = open(LIVE).read()
    _, sections = split_sections(text)
    newest = next((b for h, b in sections
                   if not h.startswith("## [Unreleased]")), "")
    for m in REL_LINK.finditer(newest):
        bad.append(("CHANGELOG.md (newest release)", m.group(1)))
    if bad:
        print("RELATIVE LINKS — these break on crates.io / GitHub Releases:")
        for where, target in bad:
            print(f"  {where}: {target}")
        return 1
    print("links: README + newest changelog section are all absolute")
    return 0


STATUS_LINE = re.compile(r"^> Status: \*\*v(\d+\.\d+\.\d+)\*\*", re.M)
WN_VERSION = re.compile(r"^> \*\*What's new in (\d+\.\d+\.\d+)", re.M)


def check_readme_current(released):
    """The README must already carry the newest changelog release.

    Trimming to the newest two says nothing about whether the newest one was
    ever WRITTEN: 0.25.0 shipped with its changelog section in place and the
    README still reading "Status: v0.24.0" over a 0.24.0 What's-new block,
    and `--check` passed, because two blocks is two blocks. So the gate now
    reads the version off the newest `## [X.Y.Z]` section and demands the
    README's Status line and its first What's-new block both name it.
    """
    if not released or not os.path.exists(README):
        return 0
    want = released[0][0].split("]")[0].split("[")[1]
    text = open(README).read()
    bad = []
    m = STATUS_LINE.search(text)
    if not m or m.group(1) != want:
        bad.append(f"Status line says v{m.group(1) if m else '?'}, changelog's newest is {want}")
    m = WN_VERSION.search(text)
    if not m or m.group(1) != want:
        bad.append(f"first What's-new block is {m.group(1) if m else 'missing'}, "
                   f"changelog's newest is {want}")
    if bad:
        print("README IS BEHIND THE CHANGELOG — write the What's-new block and "
              "bump the Status line before rolling:")
        for b in bad:
            print(f"  {b}")
        return 1
    print(f"README: Status line and newest What's-new both say {want}")
    return 0


def main():
    live = open(LIVE).read()
    preamble, sections = split_sections(live)

    unreleased = [s for s in sections if s[0].startswith("## [Unreleased]")]
    released = [s for s in sections if not s[0].startswith("## [Unreleased]")]

    rc_readme = roll_readme(KEEP, CHECK) or check_readme_current(released)
    rc_links = check_absolute_links()

    if len(released) <= KEEP:
        print(f"CHANGELOG: {len(released)} released section(s), keep={KEEP} — nothing to roll")
        return rc_readme or rc_links

    keep, move = released[:KEEP], released[KEEP:]

    if CHECK:
        print(f"CHANGELOG: {len(move)} section(s) would move to "
              f"{os.path.basename(ARCHIVE)}")
        for h, _ in move:
            print(f"  {h}")
        return 1

    # Prepend the moved sections to the archive, newest first, so the archive
    # stays in the same order as the live file.
    if os.path.exists(ARCHIVE):
        old = open(ARCHIVE).read()
        _, old_sections = split_sections(old)
    else:
        old_sections = []
    have = {h for h, _ in old_sections}
    merged = [s for s in move if s[0] not in have] + old_sections

    with open(ARCHIVE, "w") as f:
        f.write(ARCHIVE_HEADER)
        for _, body in merged:
            f.write("\n" + body.rstrip() + "\n")

    link = (f"\n---\n\nOlder releases: "
            f"[`CHANGELOG-ARCHIVE.md`](CHANGELOG-ARCHIVE.md) "
            f"({len(merged)} earlier releases, {merged[-1][0].split(']')[0].split('[')[1]}"
            f"–{merged[0][0].split(']')[0].split('[')[1]}).\n")
    with open(LIVE, "w") as f:
        f.write(preamble.rstrip() + "\n")
        for _, body in unreleased + keep:
            f.write("\n" + body.rstrip() + "\n")
        f.write(link)

    print(f"moved {len(move)} section(s) to {os.path.basename(ARCHIVE)}:")
    for h, _ in move:
        print(f"  {h}")
    print(f"CHANGELOG.md now keeps Unreleased + {len(keep)}")
    return rc_readme or rc_links


if __name__ == "__main__":
    sys.exit(main())
