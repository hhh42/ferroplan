;; Survival village-builder — a stress test of ferroplan's feature surface:
;;   durative actions, numeric fluents (graph distances + material counts),
;;   ADL: disjunctive preconditions (road OR trail), quantified preconditions
;;   (build-square needs ALL slots built), conditional effects (skilled chopper
;;   gets +1 wood), forall+when effects (fire warms everyone at camp), negation.
(define (domain village)
  (:requirements :typing :adl :durative-actions :numeric-fluents)
  (:types agent location slot - object)
  (:predicates
    (at ?a - agent ?l - location)
    (road ?x ?y - location)
    (trail ?x ?y - location)
    (is-forest ?l - location)
    (is-quarry ?l - location)
    (is-camp ?l - location)
    (has-axe ?a - agent)
    (has-pick ?a - agent)
    (skilled ?a - agent)
    (warm ?a - agent)
    (fire-lit)
    (built ?s - slot)
    (square-built))
  (:functions
    (dist ?x ?y - location)
    (wood) (stone) (sticks)
    (chops-left ?l - location))

  ;; Travel a road OR a trail (disjunctive precond); duration = graph distance.
  (:durative-action travel
    :parameters (?a - agent ?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at ?a ?from))
                    (at start (or (road ?from ?to) (trail ?from ?to))))
    :effect (and (at start (not (at ?a ?from)))
                 (at end (at ?a ?to))))

  ;; Chop wood: need an axe at a non-exhausted forest. Skilled => +1 (conditional).
  (:durative-action chop-wood
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (is-forest ?l))
                    (at start (has-axe ?a)) (at start (> (chops-left ?l) 0)))
    :effect (and (at end (increase (wood) 1))
                 (at end (decrease (chops-left ?l) 1))
                 (at end (when (skilled ?a) (increase (wood) 1)))))

  ;; Mine stone: need a pick at a quarry.
  (:durative-action mine-stone
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?a ?l)) (at start (is-quarry ?l)) (at start (has-pick ?a)))
    :effect (at end (increase (stone) 1)))

  ;; Gather sticks: any forest, no tool.
  (:durative-action gather-sticks
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 1)
    :condition (and (at start (at ?a ?l)) (at start (is-forest ?l)))
    :effect (at end (increase (sticks) 1)))

  ;; Make fire at camp; warms EVERY agent at camp (forall + when over the state).
  (:durative-action make-fire
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (is-camp ?l)) (at start (>= (sticks) 2)))
    :effect (and (at start (decrease (sticks) 2))
                 (at end (fire-lit))
                 (at end (forall (?b - agent) (when (at ?b ?l) (warm ?b))))))

  ;; Build a house in an empty slot (negation) from wood + stone.
  (:durative-action build-house
    :parameters (?a - agent ?l - location ?s - slot)
    :duration (= ?duration 5)
    :condition (and (at start (at ?a ?l)) (at start (is-camp ?l))
                    (at start (>= (wood) 2)) (at start (>= (stone) 1))
                    (at start (not (built ?s))))
    :effect (and (at start (decrease (wood) 2)) (at start (decrease (stone) 1))
                 (at end (built ?s))))

  ;; Build the village square — ONLY once every house slot is built (forall precond).
  (:durative-action build-square
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?a ?l)) (at start (is-camp ?l))
                    (at start (>= (stone) 2))
                    (at start (forall (?s - slot) (built ?s))))
    :effect (and (at start (decrease (stone) 2))
                 (at end (square-built)))))
