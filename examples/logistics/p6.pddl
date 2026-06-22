;; P6 HARD: full transshipment WITH cross deliveries. Two cities, each with a
;; local depot and a rail-depot. Packages cross via the train AND swap dests.
;;   a1 --road-- railA ==rail== railB --road-- b1
;; box pA starts at a1, must reach b1.  box pB starts at b1, must reach a1.
(define (problem log-p6) (:domain logistics)
  (:objects
    a1 railA railB b1 - location
    ta tb - truck
    tn - train
    pa pb - package)
  (:init
    (is-truck ta) (is-truck tb) (is-train tn)
    (rail-depot railA) (rail-depot railB)
    (road a1 railA) (road railA a1)
    (road b1 railB) (road railB b1)
    (rail railA railB) (rail railB railA)
    (= (dist a1 railA) 3) (= (dist railA a1) 3)
    (= (dist b1 railB) 3) (= (dist railB b1) 3)
    (= (dist railA railB) 6) (= (dist railB railA) 6)
    (at-veh ta a1) (at-veh tb b1) (at-veh tn railA)
    (= (capacity ta) 4) (= (aboard ta) 0)
    (= (capacity tb) 4) (= (aboard tb) 0)
    (= (capacity tn) 8) (= (aboard tn) 0)
    (= (stock a1 pa) 1) (= (stock b1 pb) 1)
    (= (stock a1 pb) 0) (= (stock b1 pa) 0)
    (= (stock railA pa) 0) (= (stock railA pb) 0)
    (= (stock railB pa) 0) (= (stock railB pb) 0)
    (= (load ta pa) 0) (= (load ta pb) 0)
    (= (load tb pa) 0) (= (load tb pb) 0)
    (= (load tn pa) 0) (= (load tn pb) 0))
  (:goal (and (>= (stock b1 pa) 1) (>= (stock a1 pb) 1))))