;; One worker: the whole crafting chain serializes (makespan ~19).
(define (problem build-1worker) (:domain rpg)
  (:init (= (workers) 1) (= (wood) 0) (= (planks) 0) (= (stone) 0))
  (:goal (house-built)))
