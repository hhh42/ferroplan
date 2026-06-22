;; P5 MED-HARD: multi-package conjunction. 3 distinct package types, each at a
;; different origin, all must reach a common hub h. Star map: a,b,c each --road-- h.
(define (problem log-p5) (:domain logistics)
  (:objects
    a b c h - location
    t1 - truck
    p q r - package)
  (:init
    (is-truck t1)
    (road a h) (road h a) (road b h) (road h b) (road c h) (road h c)
    (= (dist a h) 3) (= (dist h a) 3)
    (= (dist b h) 3) (= (dist h b) 3)
    (= (dist c h) 3) (= (dist h c) 3)
    (at-veh t1 h)
    (= (capacity t1) 4) (= (aboard t1) 0)
    (= (stock a p) 1) (= (stock b q) 1) (= (stock c r) 1)
    (= (stock a q) 0) (= (stock a r) 0)
    (= (stock b p) 0) (= (stock b r) 0)
    (= (stock c p) 0) (= (stock c q) 0)
    (= (stock h p) 0) (= (stock h q) 0) (= (stock h r) 0)
    (= (load t1 p) 0) (= (load t1 q) 0) (= (load t1 r) 0))
  (:goal (and (>= (stock h p) 1) (>= (stock h q) 1) (>= (stock h r) 1))))