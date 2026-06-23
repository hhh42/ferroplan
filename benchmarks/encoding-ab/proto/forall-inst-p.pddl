(define (problem chain-k3-n2) (:domain craft-forall)
  (:init
    (= (stock r0) 2) (= (stock r1) 0) (= (stock r2) 0) (= (stock r3) 0)
    ;; need[recipe][resource]
    (= (need c0 r0) 1) (= (need c0 r1) 0) (= (need c0 r2) 0) (= (need c0 r3) 0)
    (= (need c1 r0) 0) (= (need c1 r1) 1) (= (need c1 r2) 0) (= (need c1 r3) 0)
    (= (need c2 r0) 0) (= (need c2 r1) 0) (= (need c2 r2) 1) (= (need c2 r3) 0)
    ;; make[recipe][resource]
    (= (make c0 r0) 0) (= (make c0 r1) 1) (= (make c0 r2) 0) (= (make c0 r3) 0)
    (= (make c1 r0) 0) (= (make c1 r1) 0) (= (make c1 r2) 1) (= (make c1 r3) 0)
    (= (make c2 r0) 0) (= (make c2 r1) 0) (= (make c2 r2) 0) (= (make c2 r3) 1))
  (:goal (>= (stock r3) 2)))
