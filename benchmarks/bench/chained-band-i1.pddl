;; Everything at zero: 12 make-a + 5 make-b + 4 make-c + finish = 22 steps.
(define (problem chained-band-1) (:domain chained-band)
    (:init (= (stock-a) 0) (= (stock-b) 0) (= (stock-c) 0)
           (= (drift-x) 0) (= (drift-y) 0) (= (drift-z) 0))
    (:goal (saved)))
