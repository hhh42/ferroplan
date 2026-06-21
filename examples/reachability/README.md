# Derived predicates (axioms) — reachability over a map

Shows ferroplan's `:derived` support. `reachable` is defined by a rule (the
transitive closure of `link`), not by action effects:

```pddl
(:derived (reachable ?a ?b - poi)
  (or (link ?a ?b)
      (exists (?c - poi) (and (link ?a ?c) (reachable ?c ?b)))))
```

Because the map (`link`) is **static**, ferroplan computes the full `reachable`
closure once at grounding and folds it into the problem's init — no per-state
axiom evaluation, no hand-written reachable pairs. This is the "explore → build a
graph → can I get there?" primitive for a game world.

```sh
ff -o examples/reachability/domain.pddl -f examples/reachability/problem.pddl
# travel camp -> cave directly (reachable camp cave is derived)
```

**Scope:** static derived predicates (body over facts no action changes) are
supported, recursion included. A derived predicate whose body depends on facts an
action *changes* (a truly per-state axiom) currently returns a clear error rather
than mis-planning.
