;; Simplest errand: a blacksmith gathers ore and forges a nail (1 nail = 2 ore).
;; The planner walks home→mine, gathers twice, walks to the forge, crafts.
(define (problem errand)
  (:domain villagers)
  (:objects home mine forge - location  ore nail - itemtype  a - agent)
  (:init
    (at a home)
    (path home mine) (path mine home) (path mine forge) (path forge mine)
    (path home forge) (path forge home)
    (source mine ore) (workshop forge nail) (recipe nail ore)
    (= (load a) 0) (= (gather-time) 1) (= (total-time) 0)
    (= (carried a ore) 0) (= (carried a nail) 0)
    (= (walk-time home mine) 3) (= (walk-time mine home) 3)
    (= (walk-time mine forge) 2) (= (walk-time forge mine) 2)
    (= (walk-time home forge) 4) (= (walk-time forge home) 4)
    (= (recipe-qty nail) 2) (= (craft-time nail) 5))
  (:goal (>= (carried a nail) 1))
  (:metric minimize (total-time)))
