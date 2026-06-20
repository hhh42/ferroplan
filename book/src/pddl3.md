# PDDL3 preferences

ferroplan compiles soft-goal preferences away (Keyder & Geffner, JAIR 2009) and
minimizes the `:metric` with anytime branch-and-bound over `total-cost`.

- **Goal preferences**, including `(forall (?x) (preference p phi))`, are
  expanded into one instance per binding; `(is-violated p)` counts violated
  instances.
- **Precondition preferences** become satisfied/violated action variants, so a
  violation is charged exactly once per application.
- The metric must be linear in `(is-violated …)` and `(total-cost)` (the IPC-5
  *simple-preferences* shape); maximize / negative / scaled metrics fall back to
  a satisficing plan with a clear note.

On large IPC-5 instances exact optimization may return a best-found plan
(flagged *not proven optimal*) within the time bound.
