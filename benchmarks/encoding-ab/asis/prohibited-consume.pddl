;; A minimal WORKING instance of the existing generic `Prohibited` domain
;; (planner/src/resources/simple.pddl). The shipped planner/bin/simple_problem.pddl
;; is stale (its goal simplifies to FALSE), so this provides a runnable
;; generic-instantaneous data point for the as-is comparison. The domain is
;; unchanged; only this problem is new.
;;
;; Goal: eat 3 units of food. Plan = claim -> pick-up -> consume x3, all via the
;; single generic `consume` action gated by the (consumable food eat) data fact.
(define (problem prohibited-consume) (:domain Prohibited)
  (:objects
    ace - Agent
    s0 - Location
    res1 - Resource)
  (:init
    (at ace s0)
    (at res1 s0)
    (active res1)
    (kind res1 food)
    (consumable food eat)
    (= (quantity res1) 5)
    (= (consumed-resources ace eat) 0))
  (:goal (>= (consumed-resources ace eat) 3)))
