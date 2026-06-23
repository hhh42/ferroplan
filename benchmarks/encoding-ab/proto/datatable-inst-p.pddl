(define (problem chain-k3-n2) (:domain craft-data-table)
  (:init
    (recipe c0 r0 r1) (recipe c1 r1 r2) (recipe c2 r2 r3)
    (= (stock r0) 2) (= (stock r1) 0) (= (stock r2) 0) (= (stock r3) 0))
  (:goal (>= (stock r3) 2)))
