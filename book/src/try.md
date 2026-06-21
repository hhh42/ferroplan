# Try it in your browser

ferroplan compiles to **WebAssembly** and runs **entirely client-side** — no
server, no install, nothing leaves your machine.

> ### [▶ Open the live planner demo](./demo/index.html)

Paste a PDDL **domain** + **problem**, choose a mode (`auto` / `ff` / `pddl3` /
`partition`), and hit **Plan** — the plan is computed in your browser by the same
Rust planner core compiled to WASM. It comes prefilled with a gripper example so
you can see a plan immediately.

For everything-else (the CLI, the library, the GUI), see the rest of the docs.
