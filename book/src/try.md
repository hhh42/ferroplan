# Try it in your browser

ferroplan compiles to **WebAssembly** and runs **entirely client-side** — no
server, no install, nothing leaves your machine.

> ### [▶ Open the live planner demo](./demo/index.html)

**Pick a built-in example** from the dropdown — gripper, numeric resources, ADL,
derived axioms, PDDL3 preferences, temporal/durative, logistics, a job shop, or one
of the RPG-world scenarios (including a **border** example that shows where a
monolithic goal stops solving in one shot and must be decomposed) — or **paste your
own** PDDL **domain** + **problem**.
Choose a mode (`auto` routes by problem features), hit **Plan**, and the plan is
computed in your browser by the same Rust planner core compiled to WASM. It runs the
gripper example on load so you see a plan immediately.

## The visual GUI, in your browser

The full Bevy GUI — graph visualizer, plan animation, and the color-coded block
editor — also runs in the browser (it's a larger download; give it a moment).

> ### [▶ Open the visualizer & block editor](./gui/index.html)

Keys: **E** toggles the editor, **Tab** switches problem/domain, **S** solves,
**Space** plays the plan; drag nodes, scroll to zoom, click to inspect.

For everything-else (the CLI, the library, install), see the rest of the docs.
