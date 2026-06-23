;; ============================================================================
;; villagers — the GENERIC, recipe-driven planning model used by a live game
;; (the `sim_core` villager planner; domain renamed `village`→`villagers` here to
;; avoid clashing with the durative `examples/village` survival-builder).
;;
;; CONTRAST WITH `rpg-world` (see ../rpg-world + this dir's README):
;;  * rpg-world is SPECIFIC + DURATIVE: ~100 hand-written `:durative-action`s, deep
;;    multi-input recipe DAGs, real makespan/concurrency.
;;  * villagers is GENERIC + NUMERIC: THREE actions (walk/gather/craft) cover any
;;    map and any single-input recipe via data in `:init` (source/workshop/recipe +
;;    numeric tables). Durations are a `(total-time)` COST the `:metric` minimizes —
;;    instantaneous + action-costs, not durative. This is the data-table encoding,
;;    and it's what a real app embeds: build the problem from world state, solve,
;;    map steps back to game verbs.
;;
;; Exercises ferroplan's: ADL (a conditional `when` — laden walking costs more),
;; numeric fluents (per-kind carried counts, agent load, recipe quantities,
;; per-edge walk-times), and numeric `:metric` optimization (minimize total-time).
;; ============================================================================
(define (domain villagers)
  (:requirements :typing :adl :numeric-fluents :action-costs)
  (:types agent location itemtype - object)
  (:predicates
    (at ?a - agent ?l - location)
    (path ?from - location ?to - location)
    (source ?l - location ?k - itemtype)
    (workshop ?l - location ?out - itemtype)
    (recipe ?out - itemtype ?in - itemtype))
  (:functions
    (carried ?a - agent ?k - itemtype)
    (load ?a - agent)
    (walk-time ?from - location ?to - location)
    (gather-time)
    (craft-time ?out - itemtype)
    (recipe-qty ?out - itemtype)
    (total-time))
  (:action walk
    :parameters (?a - agent ?from - location ?to - location)
    :precondition (and (at ?a ?from) (path ?from ?to))
    :effect (and (not (at ?a ?from)) (at ?a ?to)
                 (increase (total-time) (walk-time ?from ?to))
                 (when (> (load ?a) 5) (increase (total-time) 2))))
  (:action gather
    :parameters (?a - agent ?l - location ?k - itemtype)
    :precondition (and (at ?a ?l) (source ?l ?k))
    :effect (and (increase (carried ?a ?k) 1) (increase (load ?a) 1)
                 (increase (total-time) (gather-time))))
  (:action craft
    :parameters (?a - agent ?l - location ?out - itemtype ?in - itemtype)
    :precondition (and (at ?a ?l) (workshop ?l ?out) (recipe ?out ?in)
                       (>= (carried ?a ?in) (recipe-qty ?out)))
    :effect (and (decrease (carried ?a ?in) (recipe-qty ?out))
                 (decrease (load ?a) (recipe-qty ?out))
                 (increase (carried ?a ?out) 1) (increase (load ?a) 1)
                 (increase (total-time) (craft-time ?out)))))
