# PPDDL probabilistic planning

Ferroplan's core crate supports PPDDL 1.0 as a separate stochastic policy API. The existing deterministic `solve` path remains unchanged.

## Semantics

The PPDDL rail compiles a domain and problem into a bounded explicit Markov decision process:

- `(probabilistic p1 e1 ... pn en)` creates a distribution over effects.
- Missing probability mass is an implicit no-op.
- Nested probabilistic effects in `and`, `when`, and instantiated `forall` are independent choices and compose by Cartesian product.
- Conditions are evaluated in the source state, matching the deterministic ADL kernel.
- Probabilistic initial conditions are resolved before the first policy decision.
- Goal states are absorbing.
- `(increase (reward) e)` and `(decrease (reward) e)` are transition rewards.
- `:goal-reward` accepts a ground numeric expression and is awarded once when the absorbing goal is reached.
- Without rewards, the default objective is maximum goal-achievement probability. With `:rewards` or `:mdp`, the default is maximum expected reward.
- Explicit `:metric` supports maximize/minimize of `goal-achieved`, `reward`, or a ground numeric expression, including finite-horizon `total-time`.

The stochastic compiler reuses the existing grounded `PackedTask` transition kernel. Each normalized outcome receives a hidden marker, is grounded as a deterministic operator, and is regrouped into one stochastic action. Marker facts and the reward accumulator are removed from canonical successor states.

## API

```rust
use ferroplan::{solve_ppddl, ProbabilisticOptions};

let solution = solve_ppddl(domain, problem, &ProbabilisticOptions::default())?;
for decision in solution.policy {
    println!("state {} -> {}", decision.state, decision.action);
}
# Ok::<(), ferroplan::PpddlError>(())
```

Use `horizon: Some(h)` for exact finite-horizon backward induction. Use `horizon: None` for value iteration. Infinite expected-reward planning requires `discount < 1`. Because `total-time` is trajectory-time dependent, it is admitted only for finite-horizon metrics and goal rewards.

Additional receipts:

- `parse_ppddl` validates and summarizes the normalized stochastic surface.
- `validate_ppddl_policy` recompiles the MDP and checks action identity, transition probability, successor identity, reward, Bellman optimality, and policy closure.
- `simulate_ppddl` executes the synthesized policy with a deterministic seed.

## Boundedness and exclusions

Explicit construction is bounded by independent limits for normalized outcomes, initial outcomes, states, transitions, finite-horizon value cells, and policy entries. Exceeding a bound is a typed refusal, never an approximation.

PPDDL 1.0 is discrete-time. The PPDDL API rejects durative actions, timed initial literals, PDDL3 trajectory constraints, and derived predicates. Those deterministic extensions remain available through their existing Ferroplan modes but are not silently mixed into PPDDL semantics.
