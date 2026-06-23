;; ============================================================================
;; cabin-crew — the DURATIVE / PARALLEL twin of the `cabin` domain. Same job (fell,
;; mill, smith, fire glass, quarry, then build in order), but actions take TIME and
;; the planner's SCHEDULING PHASE packs them onto a crew of workers — one job per
;; worker at a time. Independent chains (wood / metal / stone / glass) then run
;; concurrently, so MORE WORKERS ⇒ SHORTER MAKESPAN. Solve with --mode temporal and
;; FF_TCONC=1 (the concurrent scheduler), and compare crew-solo / crew-pair /
;; crew-trio.
;;
;; Workers are INTERCHANGEABLE and the domain is LOCKLESS (no per-worker "busy"
;; flag): so the causal search treats the crew as one (symmetric — no blow-up), and
;; the scheduler decides who-does-what-when. Kept small (a one-room cabin) to stay
;; inside the temporal search's horizon.
;; ============================================================================
(define (domain cabin-crew-skilled)
  (:requirements :typing :durative-actions :numeric-fluents)
  (:types worker location - object)
  (:predicates
    (at ?w - worker ?l - location)
    (is-site ?l - location)
    (smith ?w - worker)        ; can smelt + forge
    (sawyer ?w - worker)       ; can mill (saw/hew/split)
    (foundation-laid) (walls-up) (roof-on)
    (door-hung) (windows-glazed) (cabin-finished))
  (:functions
    (logs) (planks) (beams) (shingles)
    (ore) (ingots) (nails)
    (sand) (panes)
    (stone))

  ;; ---------------- harvest ----------------
  (:durative-action fell-tree
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)))
    :effect (at end (increase (logs) 1)))
  (:durative-action mine-ore
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)))
    :effect (at end (increase (ore) 1)))
  (:durative-action quarry-stone
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)))
    :effect (at end (increase (stone) 1)))
  (:durative-action dig-sand
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)))
    :effect (at end (increase (sand) 1)))

  ;; ---------------- process ----------------
  (:durative-action saw-planks
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (sawyer ?w)) (at start (>= (logs) 1)))
    :effect (and (at start (decrease (logs) 1)) (at end (increase (planks) 3))))
  (:durative-action hew-beam
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (sawyer ?w)) (at start (>= (logs) 1)))
    :effect (and (at start (decrease (logs) 1)) (at end (increase (beams) 2))))
  (:durative-action split-shingles
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (sawyer ?w)) (at start (>= (logs) 1)))
    :effect (and (at start (decrease (logs) 1)) (at end (increase (shingles) 4))))
  (:durative-action smelt-ingot
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (smith ?w)) (at start (>= (ore) 1)))
    :effect (and (at start (decrease (ore) 1)) (at end (increase (ingots) 1))))
  (:durative-action forge-nails
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (smith ?w)) (at start (>= (ingots) 1)))
    :effect (and (at start (decrease (ingots) 1)) (at end (increase (nails) 10))))
  (:durative-action fire-glass
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (>= (sand) 2)))
    :effect (and (at start (decrease (sand) 2)) (at end (increase (panes) 1))))

  ;; ---------------- build (strict linear order) ----------------
  (:durative-action lay-foundation
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 5)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l))
                    (at start (>= (stone) 2)) (at start (>= (beams) 1)))
    :effect (and (at start (decrease (stone) 2)) (at start (decrease (beams) 1))
                 (at end (foundation-laid))))
  (:durative-action raise-walls
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 6)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (foundation-laid))
                    (at start (>= (planks) 4)) (at start (>= (beams) 2)) (at start (>= (nails) 3)))
    :effect (and (at start (decrease (planks) 4)) (at start (decrease (beams) 2)) (at start (decrease (nails) 3))
                 (at end (walls-up))))
  (:durative-action frame-roof
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 6)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (walls-up))
                    (at start (>= (beams) 2)) (at start (>= (shingles) 4)) (at start (>= (nails) 2)))
    :effect (and (at start (decrease (beams) 2)) (at start (decrease (shingles) 4)) (at start (decrease (nails) 2))
                 (at end (roof-on))))
  (:durative-action fit-door
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (walls-up))
                    (at start (>= (planks) 3)) (at start (>= (nails) 2)))
    :effect (and (at start (decrease (planks) 3)) (at start (decrease (nails) 2))
                 (at end (door-hung))))
  (:durative-action fit-windows
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l)) (at start (walls-up))
                    (at start (>= (planks) 2)) (at start (>= (panes) 2)) (at start (>= (nails) 2)))
    :effect (and (at start (decrease (planks) 2)) (at start (decrease (panes) 2)) (at start (decrease (nails) 2))
                 (at end (windows-glazed))))
  (:durative-action finish-cabin
    :parameters (?w - worker ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?w ?l)) (at start (is-site ?l))
                    (at start (roof-on)) (at start (door-hung)) (at start (windows-glazed)))
    :effect (at end (cabin-finished))))
