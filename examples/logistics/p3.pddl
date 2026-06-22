;; P3 MED: transshipment. Truck-only ends, train in the middle.
;;   a --road-- railA ==rail== railB --road-- d
;; one truck on each side, one train. Deliver box a->d (must use all three).
(define (problem log-p3) (:domain logistics)
  (:objects
    a railA railB d - location
    tw - truck   tr - truck
    tn - train
    box - package)
  (:init
    (is-truck tw) (is-truck tr) (is-train tn)
    (rail-depot railA) (rail-depot railB)
    (road a railA) (road railA a)
    (road railB d) (road d railB)
    (rail railA railB) (rail railB railA)
    (= (dist a railA) 3) (= (dist railA a) 3)
    (= (dist railB d) 3) (= (dist d railB) 3)
    (= (dist railA railB) 5) (= (dist railB railA) 5)
    (at-veh tw a) (at-veh tr railB) (at-veh tn railA)
    (= (capacity tw) 4) (= (aboard tw) 0)
    (= (capacity tr) 4) (= (aboard tr) 0)
    (= (capacity tn) 8) (= (aboard tn) 0)
    (= (stock a box) 1) (= (stock railA box) 0) (= (stock railB box) 0) (= (stock d box) 0)
    (= (load tw box) 0) (= (load tr box) 0) (= (load tn box) 0))
  (:goal (>= (stock d box) 1)))