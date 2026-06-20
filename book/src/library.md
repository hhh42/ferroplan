# Library API

The library returns **typed, `serde`-serializable** structures.

```rust,no_run
use ferroplan::{solve, Mode, Options};

let opts = Options { mode: Mode::Auto, threads: 0 };
let sol = ferroplan::solve(domain, problem, &opts)?;
# let (domain, problem) = ("", "");
```

Key types:

- `Solution { solved, mode, plan: Option<Plan>, statistics, notes }`
- `Plan { steps: Vec<Step>, length, metric: Option<f64> }`
- `Step { index, action, args }`
- `SolveError` (parse / empty-type), via `thiserror`

Everything serializes to JSON, so `solve` doubles as the core of a planning
service. See `examples/json_api.rs`.
