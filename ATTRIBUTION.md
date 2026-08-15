# Code attribution

ferroplan is MIT OR Apache-2.0. This file records code adopted into the
tree from elsewhere — the companion to
[`benchmarks/ATTRIBUTION.md`](benchmarks/ATTRIBUTION.md), which records
the vendored benchmark corpus.

## ferroplan-sat (absorbed from varisat)

`crates/ferroplan-sat` is ferroplan's in-tree CDCL SAT solver, absorbed
in the 0.24 cycle from **varisat 0.2.2** by Jannis Harder
(https://github.com/jix/varisat, `master` @ `33e876937c5d`, crates
`varisat` and `varisat-formula`):

- License: **MIT OR Apache-2.0** — ferroplan's exact dual license, so
  the absorption carries the license through unchanged (`LICENSE-MIT`,
  `LICENSE-APACHE` at the repo root apply). Copyright (c) 2017-2019
  Jannis Harder; every absorbed file names the origin, the upstream
  revision, and the copyright in its header comment.
- The absorption is adoption, not vendoring: from the absorption commit
  onward the code is ferroplan's, carried forward on ferroplan's
  roadmap. The proof/DRAT/checker layers, the CLI, and the sampling and
  tuning surfaces were stripped behind the `tests/satdiff.rs`
  differential battery; the `partial_ref`/`vec_mut_scan`/
  `ordered-float`/`rustc-hash`/`log`/`serde` dependencies were replaced
  with plain std Rust — the crate has zero external dependencies.
- Not absorbed: `varisat-dimacs` (replaced by a small ferroplan-original
  reader in `src/dimacs.rs`), `varisat-checker`,
  `varisat-internal-proof`, `varisat-internal-macros`, `varisat-lrat`,
  `varisat-cli`.

The upstream reference checkout lives at `.solver-refs/varisat`
(gitignored) for differential comparison.
