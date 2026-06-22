;; P1 EASY: one truck, two adjacent locations, deliver 1 package.
(define (problem log-p1) (:domain logistics)
  (:objects
    a b - location
    t1 - truck
    box - package)
  (:init
    (is-truck t1)
    (road a b) (road b a)
    (= (dist a b) 3) (= (dist b a) 3)
    (at-veh t1 a)
    (= (capacity t1) 4) (= (aboard t1) 0)
    (= (stock a box) 1) (= (stock b box) 0)
    (= (load t1 box) 0))
  (:goal (>= (stock b box) 1)))