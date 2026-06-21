# ferroplan (Python)

Python bindings for the [ferroplan](../ferroplan) PDDL planner, via
[pyo3](https://pyo3.rs) + [maturin](https://maturin.rs). Builds a portable
**abi3** wheel (one wheel works on CPython 3.8+).

## Install / build

```
pip install maturin
cd crates/ferroplan-py
maturin develop --release          # build + install into the active venv
# or: maturin build --release       # produce a wheel in target/wheels/
```

On a Python newer than your pyo3 knows about, prefix with
`PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`.

## Use

```python
import ferroplan, json

domain  = open("domain.pddl").read()
problem = open("problem.pddl").read()

sol = json.loads(ferroplan.plan(domain, problem))   # mode="auto", threads=auto
if sol["solved"]:
    print(sol["plan"]["length"], "steps,", sol["statistics"]["evaluated_states"], "states")
    for step in sol["plan"]["steps"]:
        print(step["index"], step["action"], *step["args"])

print("ferroplan", ferroplan.version())
```

`plan(domain, problem, mode=None, threads=None)` returns a JSON string of the
`Solution` (or `{"error": "..."}`). `mode` ∈ `auto | ff | pddl3 | partition`.
