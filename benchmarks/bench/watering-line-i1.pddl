;; Tap at the origin, plants fanned out at staggered distances; capacity 4
;; forces shuttle trips. Under the first-wins charge, watering the near
;; plant re-points the priced gap at a farther one — h jumps up exactly on
;; progress (the ext-plant-watering i7 shape; the exact jump is pinned at
;; unit level in heuristic.rs). Under the summed charge every plant keeps
;; its own term and arrivals retire them smoothly — mode-auto solves on
;; the gradient; this instance is the end-to-end witness.
(define (problem watering-line-1)
    (:domain watering-line)
    (:objects a1 - agent t1 - tap p1 p2 p3 p4 p5 - plant)
    (:init
        (= (x a1) 0) (= (y a1) 0)
        (= (x t1) 0) (= (y t1) 0)
        (= (x p1) 2) (= (y p1) 7)
        (= (x p2) 5) (= (y p2) 2)
        (= (x p3) 7) (= (y p3) 6)
        (= (x p4) 9) (= (y p4) 1)
        (= (x p5) 11) (= (y p5) 9)
        (= (carrying a1) 0)
        (= (max_carry a1) 4)
        (= (poured p1) 0) (= (poured p2) 0) (= (poured p3) 0)
        (= (poured p4) 0) (= (poured p5) 0)
        (= (maxx) 12) (= (maxy) 10))
    (:goal (and
        (= (poured p1) 4)
        (= (poured p2) 4)
        (= (poured p3) 4)
        (= (poured p4) 4)
        (= (poured p5) 4)))
)
