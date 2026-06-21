# Try it in your browser

ferroplan compiles to **WebAssembly** and runs **entirely client-side** — no
server, no install, nothing leaves your machine.

> ### [▶ Open the live planner demo](./demo/index.html)

Paste a PDDL **domain** + **problem**, choose a mode (`auto` / `ff` / `pddl3` /
`partition`), and hit **Plan** — the plan is computed in your browser by the same
Rust planner core compiled to WASM. It comes prefilled with a gripper example so
you can see a plan immediately.

## The visual GUI, in your browser

The full Bevy GUI — graph visualizer, plan animation, and the color-coded block
editor — also runs in the browser (it's a larger download; give it a moment).

> ### [▶ Open the visualizer & block editor](./gui/index.html)

Keys: **E** toggles the editor, **Tab** switches problem/domain, **S** solves,
**Space** plays the plan; drag nodes, scroll to zoom, click to inspect.

For everything-else (the CLI, the library, install), see the rest of the docs.
