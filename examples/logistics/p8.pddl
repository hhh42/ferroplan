;; P8 HARDEST: capacity-forced multi-trip transshipment. 6 boxes at a1 must all
;; reach b1 across the rail bridge, but every vehicle is small (cap 2), so the
;; train and trucks must each make multiple loaded round trips.
(define (problem log-p8) (:domain logistics)
  (:objects
    a1 railA railB b1 - location
    ta tb - truck
    tn - train
    box - package)
  (:init
    (is-truck ta) (is-truck tb) (is-train tn)
    (rail-depot railA) (rail-depot railB)
    (road a1 railA) (road railA a1)
    (road b1 railB) (road railB b1)
    (rail railA railB) (rail railB railA)
    (= (dist a1 railA) 2) (= (dist railA a1) 2)
    (= (dist b1 railB) 2) (= (dist railB b1) 2)
    (= (dist railA railB) 4) (= (dist railB railA) 4)
    (at-veh ta a1) (at-veh tb railB) (at-veh tn railA)
    (= (capacity ta) 2) (= (aboard ta) 0)
    (= (capacity tb) 2) (= (aboard tb) 0)
    (= (capacity tn) 2) (= (aboard tn) 0)
    (= (stock a1 box) 6) (= (stock railA box) 0) (= (stock railB box) 0) (= (stock b1 box) 0)
    (= (load ta box) 0) (= (load tb box) 0) (= (load tn box) 0))
  (:goal (>= (stock b1 box) 6)))