# ferroplan-sat

`ferroplan`'s in-tree CDCL SAT solver: watched literals, 1UIP clause
learning, VSIDS decisions with phase saving, restarts, clause-database
reduction, and an incremental-assumptions interface, with a
per-solve conflict budget (`Interrupted` as an honest third verdict
alongside SAT/UNSAT). Zero external dependencies.

Absorbed in ferroplan's 0.24 cycle from
[varisat 0.2.2](https://github.com/jix/varisat) by Jannis Harder — see
[`ATTRIBUTION.md`](https://github.com/hhh42/ferroplan/blob/main/ATTRIBUTION.md)
at the repo root for the full absorption record (what was kept, what
was stripped, and why). From the absorption commit onward this is
ferroplan's own code, carried forward on ferroplan's roadmap — not a
vendored copy.

This crate is a building block for `ferroplan`'s SAT-compiled planning
wing (bounded-layer ∃-step encoding for classical/temporal PDDL). It
is a general-purpose CDCL solver in its own right and has no
planning-specific dependencies, but its API and internals are shaped
by that one caller — treat it as ferroplan-internal infrastructure
rather than a general-audience SAT library, at least for now.

## License

MIT OR Apache-2.0, matching the rest of the `ferroplan` workspace —
the same dual license varisat itself used, so the absorption carries
it through unchanged.
