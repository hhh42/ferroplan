;; done AND charge >= 6. The cost-5 plan [bigpump fire] ends at charge 5:
;; an engine skipping the numeric goal conjunct certifies 5. True optimum
;; 6 = bigpump(4) + pump(1) + fire(1).
(define (problem numopt-p01)
  (:domain numopt)
  (:init (= (charge) 0) (= (total-cost) 0))
  (:goal (and (done) (>= (charge) 6)))
  (:metric minimize (total-cost)))
