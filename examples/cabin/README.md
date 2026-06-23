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

## Parallel crew — `crew.pddl` (makespan drops with more workers)

`crew.pddl` is the **durative** twin: the same job, but actions take time and the
planner's **scheduling phase** packs them onto a crew of workers (one job per worker
at a time). Independent work — chopping, mining, digging, firing glass — then
overlaps, so **more workers finish sooner**. Same 34-step job, different makespan:

```sh
ff -o examples/cabin/crew.pddl -f examples/cabin/crew-solo.pddl --mode temporal   # 1 worker  -> makespan 109
ff -o examples/cabin/crew.pddl -f examples/cabin/crew-pair.pddl --mode temporal   # 2 workers -> makespan 63
ff -o examples/cabin/crew.pddl -f examples/cabin/crew-trio.pddl --mode temporal   # 3 workers -> makespan 47
```

This needs the concurrent scheduler, which is gated: set `FF_TDEMAND=1 FF_TCONC=1`
(or, in the web demo, the example carries flags `tdemand,tconc`). Why a separate
phase? ferroplan's temporal *search* is guided by action count, not makespan, so on
its own it lays actions out sequentially (makespan = the serial sum, regardless of
crew size). The scheduler (`crate::tsched`) searches a single-actor reduction for
*what* to do, then repacks it across the crew for *who does what, when* — validated,
and only kept if it's genuinely shorter. The crew domain is **lockless** (workers
interchangeable) so the search stays small and the scheduler owns the parallelism.

## Files
- `domain.pddl` — the cabin domain (harvest + mill + smith + glass + masonry + the
  9-stage linear build), classical/numeric, solo.
- `raise-frame.pddl` — goal `roof-on`: the weather-tight shell.
- `raise-cabin.pddl` — goal `cabin-finished`: the complete cabin, door and windows.
- `crew.pddl` — the durative twin; `crew-{solo,pair,trio}.pddl` — 1/2/3-worker crews
  for the makespan comparison (run with `FF_TDEMAND=1 FF_TCONC=1`).
