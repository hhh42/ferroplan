# Command line (`ff`)

The `ff` binary is a drop-in for Metric-FF's `ff -o domain -f problem`.

| flag | meaning |
|---|---|
| `-o, --domain <FILE>` | PDDL domain |
| `-f, --problem <FILE>` | PDDL problem |
| `--json` | emit a structured JSON `Solution` instead of classic text |
| `--json-request <FILE>` | self-contained `{domain, problem, options}` JSON job (`-` = stdin) |
| `--mode <auto\|ff\|partition\|pddl3>` | planning strategy (default `auto`) |
| `--threads <N>` | worker threads (`0` = auto) |
| `--ipc` | IPC time-stamped plan format (text mode) |

`auto` uses classic FF for classical/numeric problems and the PDDL3 metric mode
when the problem has preferences.

```sh
ff -o domain.pddl -f problem.pddl --json
echo '{"domain":"...","problem":"...","options":{"mode":"ff"}}' | ff --json-request -
```
