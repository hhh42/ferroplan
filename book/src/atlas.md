# Architecture atlas

An interactive, zoomable map of the whole planner. It covers the front doors, the solve pipeline, heuristics, search, planning modes, the Session, the SAT solver, data structures and the measurement/release process. Every box opens in place to show its parts, down to write-ups that link into the source.

**[Open the atlas full-screen →](./atlas.html)**

<iframe src="./atlas.html" title="ferroplan architecture atlas" style="width:100%;height:80vh;border:1px solid #c9c5bc;border-radius:3px"></iframe>

The atlas is generated: content lives in `book/atlas/content/*.json`, and
`python3 book/atlas/build.py` writes `book/src/atlas.html`.
