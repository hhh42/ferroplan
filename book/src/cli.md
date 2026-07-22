# Command line (`ff`)

The `ff` binary is a drop-in for Metric-FF's `ff -o domain -f problem`.

| flag | meaning |
|---|---|
| `-o, --domain <FILE>` | PDDL domain |
| `-f, --problem <FILE>` | PDDL problem |
| `--json` | emit a structured JSON `Solution` instead of classic text |
| `--json-request <FILE>` | self-contained `{domain, problem, options}` JSON job (`-` = stdin) |
| `--mode <auto\|ff\|partition\|pddl3\|temporal\|portfolio>` | planning strategy (default `auto`) |
| `--search <auto\|ehc\|best-first\|ehc-then-best-first>` | search strategy |
| `--weight-g <W>` / `--weight-h <W>` | best-first path-length / heuristic weights |
| `--max-evaluated <N>` | cap on nodes evaluated before giving up |
| `--satisfice` | PDDL3: stop at the first plan instead of optimizing the metric |
| `--decompose` | split a too-big temporal goal into ordered contracts and stitch them |
| `--validate <PLAN>` | replay a plan file under ferroplan's own apply semantics (exit 0/1) |
| `--threads <N>` | worker threads (`0` = auto) |
| `--ipc` | IPC time-stamped plan format (text mode) |

`auto` routes by problem features: classic FF for classical/numeric, the PDDL3
metric optimizer when the problem has preferences, and the decision-epoch temporal
search when the domain has `:durative-action`s. `partition` selects the
SGPlan-style partition-and-resolve mode; `portfolio` runs the budget-aware
sequential strategy ladder on one shared eval pool. Run `ff --help` for the
full list.

```sh
ff -o domain.pddl -f problem.pddl --json
ff -o temporal-domain.pddl -f problem.pddl --mode temporal --decompose
ff -o domain.pddl -f problem.pddl --validate plan.txt
echo '{"domain":"...","problem":"...","options":{"mode":"ff"}}' | ff --json-request -
```
