# ferroplan-wasm

WebAssembly bindings for [ferroplan](../ferroplan) — run the PDDL planner entirely
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
  JSON-serialized `Solution` (or `{"error": "..."}`). `mode` ∈ auto | ff | pddl3 | partition.
- `version() -> string`.

Not published to crates.io (it's a build target). WASM has no threads here, so the
planner runs single-threaded (identical results, just not parallel).
