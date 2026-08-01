# ferroplan-wasm

WebAssembly bindings for [ferroplan](https://github.com/seanchatmangpt/ferroplan/tree/main/crates/ferroplan) — run the PDDL planner entirely
in the browser, no server or install.

## Build

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli          # match the wasm-bindgen crate version
./build.sh                              # -> web/pkg/
```

## Try it

```
python3 -m http.server -d web 8000
# open http://localhost:8000
```

Paste a PDDL domain + problem, hit **Plan** — everything runs client-side.

## API

- `plan(domain: string, problem: string, mode?: string) -> string` — returns a
  JSON-serialized `Solution` (or `{"error": "..."}`). `mode` ∈ auto | ff | pddl3 | partition | temporal.
- `explain(domain, problem, plan_json) -> string` — plan introspection
  (0.18): causal links (classical), invariant spans (temporal), preference
  breakdown (PDDL3), as an `Explanation` JSON. `plan_json` is a
  `Solution`'s `plan` field.
- `WasmSession` — the live `Session` surface: `fork`, `set_goal`,
  `restrict_prefix_claims` / `restrict_contains`, `think`, `valid` /
  `plan_valid_json`, `apply_start`, `elapse`, `set_fact` / `set_fluent`,
  `fact` / `fluent`, `observe`, `goal_met` — what the live pages drive.
- `version() -> string`.

## The live pages

- `web/index.html` — the solver demo; **Explain this plan** renders the
  introspection views for any solved instance.
- `web/bazaar-live.html` — the living bazaar (0.15): claims + fog.
- `web/village-live.html` — the living village (0.18): the tick-loop
  economy over `benchmarks/village/pair.pddl`, workers hired by goal
  contract, map + economy timeline + disruption buttons. Its PDDL rides
  in `web/village-data.js`, generated from the canonical fixtures:

  ```
  python3 - <<'PY'
  import json
  dom = open('benchmarks/village/domain.pddl').read()
  prb = open('benchmarks/village/pair.pddl').read()
  open('crates/ferroplan-wasm/web/village-data.js', 'w').write(
      "// Generated from benchmarks/village/{domain,pair}.pddl — the canonical\n"
      "// fixtures stay the source of truth. Regenerate after editing them:\n"
      "//   python3 - <<'PY'\n"
      "//   (see crates/ferroplan-wasm/README.md, village live page section)\n"
      "//   PY\n"
      f"export const VILLAGE_DOMAIN = {json.dumps(dom)};\n"
      f"export const VILLAGE_PROBLEM = {json.dumps(prb)};\n")
  PY
  ```

Not published to crates.io (it's a build target). WASM has no threads here, so the
planner runs single-threaded (identical results, just not parallel).
