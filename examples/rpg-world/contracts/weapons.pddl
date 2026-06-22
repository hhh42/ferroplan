(define (problem wa-contract-1)
  (:domain rpg-world)
  (:objects
    smith1 - agent
    armory - location)
  (:init
    (at smith1 armory)
    ;; station cluster
    (has-anvil armory)
    (has-fletching-bench armory)
    ;; tool + roles
    (has-hammer smith1)
    (smith smith1)
    (fletcher smith1)
    ;; pre-delivered raw inputs
    (= (ingots) 8)
    (= (planks) 6)
    ;; reachability for the lone node (self-link keeps reachable axiom happy if needed)
    (link armory armory)
    (= (dist armory armory) 0)
    ;; every fluent this subsystem references, initialized
    (= (swords) 0)
    (= (spearheads) 0)
    (= (arrowheads) 0)
    (= (shields) 0)
    (= (arrows) 0))
  (:goal (and
    (>= (swords) 1)
    (>= (spearheads) 1)
    (>= (shields) 1)
    (>= (arrows) 8))))