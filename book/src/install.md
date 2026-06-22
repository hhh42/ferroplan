# Install & quick start

```sh
git clone https://github.com/hhh42/ferroplan
cd ferroplan
cargo build --release      # -> target/release/ff
```

Solve a problem:

```sh
./target/release/ff -o domain.pddl -f problem.pddl
```

Or from code — add `ferroplan` as a path/git dependency and:

```rust,no_run
let domain  = std::fs::read_to_string("domain.pddl").unwrap();
let problem = std::fs::read_to_string("problem.pddl").unwrap();
let sol = ferroplan::solve(&domain, &problem, &ferroplan::Options::default()).unwrap();
println!("{:?}", sol.plan.map(|p| p.length));
```
