;; A small RPG crafting/economy domain for ferroplan's temporal + resource engine.
;;
;; It exercises exactly the primitives a live game needs of a low-level planner:
;;   - DURATIVE actions     (chopping/sawing/mining/building take in-world time)
;;   - a RENEWABLE resource  (workers): consumed at-start, released at-end, so a
;;                           limited crew forces a schedule (more workers -> more
;;                           parallelism -> shorter makespan)
;;   - CONSUMABLE resources  (wood -> planks; planks + stone -> a house)
;;   - a CRAFTING CHAIN      (chop -> saw -> build; mine -> build)
;;
;; ferroplan schedules this respecting the worker pool and the material flow:
;;   ff -o examples/rpg/domain.pddl -f examples/rpg/build-1worker.pddl   ; serialized
;;   ff -o examples/rpg/domain.pddl -f examples/rpg/build-3workers.pddl  ; parallel
(define (domain rpg)
  (:requirements :typing :durative-actions :numeric-fluents)
  (:predicates (house-built))
  (:functions (workers) (wood) (planks) (stone))

  ;; A worker is busy for 3, yields 1 wood.
  (:durative-action chop-tree
    :parameters ()
    :duration (= ?duration 3)
    :condition (at start (>= (workers) 1))
    :effect (and (at start (decrease (workers) 1))
                 (at end (increase (workers) 1))
                 (at end (increase (wood) 1))))

  ;; Saw 1 wood into 2 planks; worker busy 2.
  (:durative-action saw-planks
    :parameters ()
    :duration (= ?duration 2)
    :condition (and (at start (>= (workers) 1)) (at start (>= (wood) 1)))
    :effect (and (at start (decrease (workers) 1)) (at start (decrease (wood) 1))
                 (at end (increase (workers) 1)) (at end (increase (planks) 2))))

  ;; Mine stone; worker busy 4, yields 1 stone.
  (:durative-action mine-stone
    :parameters ()
    :duration (= ?duration 4)
    :condition (at start (>= (workers) 1))
    :effect (and (at start (decrease (workers) 1))
                 (at end (increase (workers) 1)) (at end (increase (stone) 1))))

  ;; Build the house: needs 4 planks + 1 stone, worker busy 5.
  (:durative-action build-house
    :parameters ()
    :duration (= ?duration 5)
    :condition (and (at start (>= (workers) 1)) (at start (>= (planks) 4)) (at start (>= (stone) 1)))
    :effect (and (at start (decrease (workers) 1)) (at start (decrease (planks) 4)) (at start (decrease (stone) 1))
                 (at end (increase (workers) 1)) (at end (house-built)))))
