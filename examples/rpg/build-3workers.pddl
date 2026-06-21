;; Three workers: chopping/mining/sawing overlap up to the crew size (makespan ~13).
(define (problem build-3workers) (:domain rpg)
  (:init (= (workers) 3) (= (wood) 0) (= (planks) 0) (= (stone) 0))
  (:goal (house-built)))
