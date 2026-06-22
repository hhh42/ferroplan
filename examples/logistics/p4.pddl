;; P4 MED: capacity batching. One truck cap 3, deliver 3 boxes a->c on a-b-c.
(define (problem log-p4) (:domain logistics)
  (:objects
    a b c - location
    t1 - truck
    box - package)
  (:init
    (is-truck t1)
    (road a b) (road b a) (road b c) (road c b)
    (= (dist a b) 3) (= (dist b a) 3) (= (dist b c) 3) (= (dist c b) 3)
    (at-veh t1 a)
    (= (capacity t1) 3) (= (aboard t1) 0)
    (= (stock a box) 3) (= (stock b box) 0) (= (stock c box) 0)
    (= (load t1 box) 0))
  (:goal (>= (stock c box) 3)))