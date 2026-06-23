# cabin — raise a log cabin from standing trees

A deliberately **deep, linear crafting chain**: a single goal (`cabin-finished`)
drags out one long, ordered sequence of work. It's the "whole sequence of shit"
example — chop, process, then build, in order.

```
fell-tree ─┬─ saw-planks ──┐
           ├─ hew-beam ─────┤
           └─ split-shingles┤
mine-ore ── smelt-ingot ── forge-nails ┤   ← everything feeds the build…
dig-sand ── fire-glass ────────────────┤
quarry-stone ──────────────────────────┤
                                        ▼
  lay-foundation → raise-walls → frame-roof → lay-floor
      → build-door → hang-door → set-window-frames → glaze-windows → finish-cabin
```

The build stages are a **strict linear chain** (each needs the previous one done),
so the plan is a forced sequence the planner can follow. The full cabin is a
**~52-step plan**: fell ~a dozen trees, mill them into planks/beams/shingles,
forge nails from ore, fire window glass from sand, quarry stone, then build.

## Why classical (not durative)?

This is modeled as a **numeric classical** domain (instantaneous actions; each
adds its time to `(total-time)`), not `:durative-actions`. ferroplan's metric/FF
search handles a long ~50-step numeric plan; the temporal decision-epoch search
**can't** — it exhausts around ~20 steps on a chain this deep (it's tuned for
shorter, more-concurrent durative problems like [`../rpg-world`](../rpg-world)).
So the lesson this example also teaches: pick the encoding to the solver's
strength — long sequential numeric builds → classical; concurrent durative work →
temporal.

## Run it

```sh
# the shell — foundation, walls, roof (~26 steps, instant)
ff -o examples/cabin/domain.pddl -f examples/cabin/raise-frame.pddl

# the whole cabin — ~52 steps end to end (a few seconds)
ff -o examples/cabin/domain.pddl -f examples/cabin/raise-cabin.pddl
```

In the web demo, "The whole log cabin" is flagged slow — run it in **Web Worker**
mode so the page stays responsive while it solves (~7s).

## Files
- `domain.pddl` — the cabin domain (harvest + mill + smith + glass + masonry + the
  9-stage linear build).
- `raise-frame.pddl` — goal `roof-on`: the weather-tight shell.
- `raise-cabin.pddl` — goal `cabin-finished`: the complete cabin, door and windows.
