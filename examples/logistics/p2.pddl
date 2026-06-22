;; P2 EASY-MED: linear road corridor a-b-c-d, one truck, deliver 1 box a->d.
(define (problem log-p2) (:domain logistics)
  (:objects
    a b c d - location
    t1 - truck
    box - package)
  (:init
    (is-truck t1)
    (road a b) (road b a) (road b c) (road c b) (road c d) (road d c)
    (= (dist a b) 3) (= (dist b a) 3)
    (= (dist b c) 3) (= (dist c b) 3)
    (= (dist c d) 3) (= (dist d c) 3)
    (at-veh t1 a)
    (= (capacity t1) 4) (= (aboard t1) 0)
    (= (stock a box) 1) (= (stock b box) 0) (= (stock c box) 0) (= (stock d box) 0)
    (= (load t1 box) 0))
  (:goal (>= (stock d box) 1)))