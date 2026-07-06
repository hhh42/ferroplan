# Library API

The library returns **typed, `serde`-serializable** structures. Every knob lives
on one `Options` struct; every field is optional via `Default`.

```rust,no_run
use ferroplan::{solve, Mode, Options};

let domain = std::fs::read_to_string("domain.pddl").unwrap();
let problem = std::fs::read_to_string("problem.pddl").unwrap();

let opts = Options { mode: Mode::Auto, ..Default::default() };
let sol = solve(&domain, &problem, &opts).unwrap();

if let Some(plan) = sol.plan {
    for step in &plan.steps {
        println!("{} {}", step.action, step.args.join(" "));
    }
    println!("metric: {:?}, makespan: {:?}", plan.metric, plan.makespan);
}
```

## The public surface

- **`solve(domain, problem, &Options)`** → `Result<Solution, SolveError>` — plan a
  domain+problem. `Mode::Auto` routes by features (temporal → decision-epoch,
  preferences → PDDL3 metric optimizer, else classical FF).
- **`parse(src)`** → `ParseReport` — syntax-check and summarize a domain *or*
  problem without grounding or solving (fast authoring feedback).
- **`decompose(domain, problem, &Options)`** → `Result<Decomposition, SolveError>`
  — split a too-big temporal goal into ordered, individually-solved contracts and
  stitch them into one validated plan (falls back to a monolithic solve when a goal
  can't be split). See [`examples/decompose.rs`](https://github.com/hhh42/ferroplan/blob/main/crates/ferroplan/examples/decompose.rs).
- **`Session::new(domain, problem, &Options)`** — ground once, then `replan` many
  times as the world changes each tick (classical domains). See
  [`examples/replan.rs`](https://github.com/hhh42/ferroplan/blob/main/crates/ferroplan/examples/replan.rs).
- **`plan::validate_plan(&domain, &problem, &plan)`** — independently replay a plan
  under ferroplan's own apply semantics. See
  [`examples/validate_plan.rs`](https://github.com/hhh42/ferroplan/blob/main/crates/ferroplan/examples/validate_plan.rs).

## Key types

- `Solution { solved, mode, plan: Option<Plan>, statistics, notes }`
- `Plan { steps: Vec<Step>, length, metric: Option<f64>, makespan: Option<f64> }`
- `Step { index, action, args, time }` — `time` is set on temporal plans.
- `SolveError` — `DomainParse` / `ProblemParse` / `EmptyType` / `Derived` /
  `Unsupported`, via `thiserror`.

Everything serializes to JSON, so `solve` doubles as the core of a planning
service. See [`examples/json_api.rs`](https://github.com/hhh42/ferroplan/blob/main/crates/ferroplan/examples/json_api.rs).
