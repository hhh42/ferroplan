;; P7 HARD: classic IPC-logistics shape. 2 cities, rail hubs, MANY packages each
;; with its own origin/dest spread across both cities. 5 packages, mixed
;; cross/local deliveries -> big conjunction + routing + transshipment.
(define (problem log-p7) (:domain logistics)
  (:objects
    c1a c1b railA railB c2a c2b - location
    t1 t2 - truck
    tn - train
    p1 p2 p3 p4 p5 - package)
  (:init
    (is-truck t1) (is-truck t2) (is-train tn)
    (rail-depot railA) (rail-depot railB)
    (road c1a railA) (road railA c1a)
    (road c1b railA) (road railA c1b)
    (road c2a railB) (road railB c2a)
    (road c2b railB) (road railB c2b)
    (rail railA railB) (rail railB railA)
    (= (dist c1a railA) 2) (= (dist railA c1a) 2)
    (= (dist c1b railA) 2) (= (dist railA c1b) 2)
    (= (dist c2a railB) 2) (= (dist railB c2a) 2)
    (= (dist c2b railB) 2) (= (dist railB c2b) 2)
    (= (dist railA railB) 6) (= (dist railB railA) 6)
    (at-veh t1 railA) (at-veh t2 railB) (at-veh tn railA)
    (= (capacity t1) 5) (= (aboard t1) 0)
    (= (capacity t2) 5) (= (aboard t2) 0)
    (= (capacity tn) 10) (= (aboard tn) 0)
    (= (stock c1a p1) 1) (= (stock c1b p2) 1)
    (= (stock c2a p3) 1) (= (stock c2b p4) 1) (= (stock c1a p5) 1)
    (= (stock c1a p2) 0) (= (stock c1a p3) 0) (= (stock c1a p4) 0)
    (= (stock c1b p1) 0) (= (stock c1b p3) 0) (= (stock c1b p4) 0) (= (stock c1b p5) 0)
    (= (stock c2a p1) 0) (= (stock c2a p2) 0) (= (stock c2a p4) 0) (= (stock c2a p5) 0)
    (= (stock c2b p1) 0) (= (stock c2b p2) 0) (= (stock c2b p3) 0) (= (stock c2b p5) 0)
    (= (stock railA p1) 0) (= (stock railA p2) 0) (= (stock railA p3) 0) (= (stock railA p4) 0) (= (stock railA p5) 0)
    (= (stock railB p1) 0) (= (stock railB p2) 0) (= (stock railB p3) 0) (= (stock railB p4) 0) (= (stock railB p5) 0)
    (= (load t1 p1) 0) (= (load t1 p2) 0) (= (load t1 p3) 0) (= (load t1 p4) 0) (= (load t1 p5) 0)
    (= (load t2 p1) 0) (= (load t2 p2) 0) (= (load t2 p3) 0) (= (load t2 p4) 0) (= (load t2 p5) 0)
    (= (load tn p1) 0) (= (load tn p2) 0) (= (load tn p3) 0) (= (load tn p4) 0) (= (load tn p5) 0))
  (:goal (and (>= (stock c2a p1) 1)
              (>= (stock c2b p2) 1)
              (>= (stock c1a p3) 1)
              (>= (stock c1b p4) 1)
              (>= (stock c2b p5) 1))))