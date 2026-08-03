;; Goal (done) alone: fire costs 1, but its charge >= 4 precondition
;; forces 4 charge-units first — an engine relaxing numeric
;; preconditions in EXPANSION (not just in h, where relaxing is
;; admissible) certifies 1. True optimum 5 = bigpump(4) + fire(1).
(define (problem numopt-p03)
  (:domain numopt)
  (:init (= (charge) 0) (= (total-cost) 0))
  (:goal (done))
  (:metric minimize (total-cost)))
