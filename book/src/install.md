# Install & quick start

Both crates are on crates.io:

```sh
cargo install ferroplan-cli    # -> the `ff` binary
```

or build from source:

```sh
git clone https://github.com/hhh42/ferroplan
cd ferroplan
cargo build --release      # -> target/release/ff
```

Solve a problem:

```sh
ff -o domain.pddl -f problem.pddl
```

Or from code — `cargo add ferroplan` and:

```rust,no_run
let domain  = std::fs::read_to_string("domain.pddl").unwrap();
let problem = std::fs::read_to_string("problem.pddl").unwrap();
let sol = ferroplan::solve(&domain, &problem, &ferroplan::Options::default()).unwrap();
println!("{:?}", sol.plan.map(|p| p.length));
```
