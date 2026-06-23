;; A township errand worth watching: two crafting chains over a 7-node map.
;;   grove --(gather)--> wood --(mill)--> plank
;;   mine  --(gather)--> ore  --(smelter)--> bar --(forge)--> tool
;; Goal: 2 tools AND 3 planks. The planner must gather the right raws (4 ore, 3
;; wood), route the map, sequence three workshops, and keep an eye on the laden-
;; walk penalty (carrying >5 makes every walk cost +2) — all while minimizing
;; total-time. One generic `craft` action drives all three recipes from `:init`.
(define (problem township)
  (:domain villagers)
  (:objects
    home grove mine mill smelter forge market - location
    wood ore plank bar tool - itemtype
    a - agent)
  (:init
    (at a home)
    ;; map (bidirectional edges)
    (path home grove) (path grove home) (path home mine) (path mine home)
    (path grove mill) (path mill grove) (path mine smelter) (path smelter mine)
    (path mill forge) (path forge mill) (path smelter forge) (path forge smelter)
    (path forge market) (path market forge) (path grove mine) (path mine grove)
    (path home market) (path market home)
    ;; sources & workshops (data — the generic actions read these)
    (source grove wood) (source mine ore)
    (workshop mill plank) (workshop smelter bar) (workshop forge tool)
    ;; single-input recipes
    (recipe plank wood) (recipe bar ore) (recipe tool bar)
    ;; numeric fluents (all must be initialized)
    (= (load a) 0) (= (total-time) 0) (= (gather-time) 1)
    (= (carried a wood) 0) (= (carried a ore) 0) (= (carried a plank) 0)
    (= (carried a bar) 0) (= (carried a tool) 0)
    (= (recipe-qty plank) 1) (= (recipe-qty bar) 2) (= (recipe-qty tool) 1)
    (= (craft-time plank) 1) (= (craft-time bar) 2) (= (craft-time tool) 3)
    (= (walk-time home grove) 2) (= (walk-time grove home) 2)
    (= (walk-time home mine) 3) (= (walk-time mine home) 3)
    (= (walk-time grove mill) 1) (= (walk-time mill grove) 1)
    (= (walk-time mine smelter) 1) (= (walk-time smelter mine) 1)
    (= (walk-time mill forge) 2) (= (walk-time forge mill) 2)
    (= (walk-time smelter forge) 1) (= (walk-time forge smelter) 1)
    (= (walk-time forge market) 2) (= (walk-time market forge) 2)
    (= (walk-time grove mine) 2) (= (walk-time mine grove) 2)
    (= (walk-time home market) 5) (= (walk-time market home) 5))
  (:goal (and (>= (carried a tool) 2) (>= (carried a plank) 3)))
  (:metric minimize (total-time)))
