# ferroplan — alternative logo concepts

Four distinct directions exploring different angles on "ferroplan" (a fast PDDL
planner in Rust). Each concept ships a wordmark (`concept-N.svg`) and a square
favicon-style mark (`concept-N-mark.svg`). All SVGs are fully self-contained:
no external fonts (`system-ui, sans-serif`) and no external images. They are
designed to read on both light and dark backgrounds — marks use solid fills and
the wordmarks avoid pure-black/white glyphs.

To compare against the current brand, see `../logo.svg` and `../logo-mark.svg`.

## Concept 1 — Forged iron (anvil + spark)

**Direction:** the "ferro" = iron/metal angle, literally. An angular anvil
silhouette gets struck, throwing a spark — the moment a plan is forged. The
spark doubles as the goal/output accent.
**Rationale:** the most concrete, memorable take; instantly says "metal" and
"made/forged" without needing to explain graph search. Bold weight reinforces
solidity and speed.
**Palette (new):** steel slate `#64748b` + forge-spark amber `#f59e0b`. A
deliberate departure from the indigo brand to commit fully to the metal theme;
the warm amber pops on both backgrounds.

## Concept 2 — Solution route

**Direction:** planning as a route. Faint pruned candidate branches fan out in
grey; one bold indigo path threads from a start node through a waypoint to the
goal, highlighting the solution the planner found.
**Rationale:** communicates "this finds a path through possibilities" at a
glance — the core value of a planner — while staying clean and minimal. The
hollow goal node reads as a target/flag.
**Palette (brand riff):** indigo `#6c5ce7` route, slate `#94a3b8` pruned
branches, emerald `#10b981` goal (a slightly punchier green than the original
`#a8d24a`, for stronger contrast on light backgrounds).

## Concept 3 — "fp" monogram

**Direction:** a minimal, geometric `fp` monogram set in a rounded-square tile.
The `f` and `p` are stroke-built and share a baseline for a tight, modern lockup.
**Rationale:** the most flexible and scalable option — a single-color tile that
works as an app icon, terminal favicon, or social avatar at tiny sizes where a
graph mark would muddy. Quiet and confident.
**Palette (brand):** indigo `#6c5ce7` tile with white glyphs. The tile fill
guarantees contrast on any background; can be recolored without touching the
glyphs.

## Concept 4 — State-space search

**Direction:** an explicit state-space graph. A small lattice of states with
dim "frontier" edges; one highlighted indigo branch is expanded from the root
(square = explored) down to the green goal state.
**Rationale:** the most technically literal — it depicts search itself, which
will resonate with the PDDL / automated-planning audience. Closest in spirit to
the existing node-graph mark but reframed around *search* (root, frontier, goal)
rather than a generic graph.
**Palette (brand):** indigo `#6c5ce7` expanded path, grey `#9aa0a6` frontier,
green `#a8d24a` goal — the original three-color palette, kept for continuity.
