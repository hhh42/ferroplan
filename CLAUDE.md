# Working agreements

- **Finish in main.** Every completed phase/cycle push goes to `main` as
  well as the working branch — `publish.sh` runs from `main`, and a
  release isn't "ready" until `main` has it. Push the working branch,
  then fast-forward `main` (`git push origin <branch>:main`).
- Cycle discipline lives in `docs/roadmap-0.N.md`: measured win or
  recorded negative, fixtures first, scoreboards defend themselves.
- Full pre-flight before any cut: latest stable, `fmt --check`, clippy
  `--all-targets --all-features -D warnings`, `test --all` (release),
  doc `-D warnings`, `bench --no-run`, ferroplan-py version+re-lock,
  `publish -p ferroplan --dry-run`, build-check `ferroplan-mcp`, and
  the maturin wheel build (0.14+). See `RELEASING.md`.
