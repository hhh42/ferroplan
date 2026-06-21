;; A small explored map: camp -> spring -> ridge -> cave (one direction).
;; Goal: be at the cave. `reachable camp cave` is derived (closure), so the planner
;; can travel straight there.
(define (problem reach1) (:domain reachability)
  (:objects camp spring ridge cave - poi)
  (:init (at camp) (link camp spring) (link spring ridge) (link ridge cave))
  (:goal (at cave)))
