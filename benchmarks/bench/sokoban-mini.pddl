;; sokoban-mini: a hand-rolled 5x2 instance of the vendored IPC-2008
;; sokoban-sequential domain (benchmarks/bench/sokoban-mini-domain.pddl).
;; Golden byte-identity member of the 0.22 Phase 7 MCV battery: push-*
;; carries three location parameters over MOVE-DIR statics — the exact
;; join shape the sitting re-attributed sokoban-t's grounding wall to —
;; so with FF_MCV_THRESHOLD=1 forced, the MCV join order must reproduce
;; this task byte-for-byte (RawOp stream, fact-intern order, plan).
;;
;; Layout (x-y, y=1 top row):   p  s1 .  G  .
;;                              .  s2 .  G  .
;; Plan: push s1 right twice; walk around; push s2 right twice.
(define (problem sokoban-mini)
  (:domain sokoban-sequential)
  (:objects
    player-01 - player
    stone-01 stone-02 - stone
    dir-down dir-left dir-right dir-up - direction
    pos-1-1 pos-2-1 pos-3-1 pos-4-1 pos-5-1
    pos-1-2 pos-2-2 pos-3-2 pos-4-2 pos-5-2 - location)
  (:init
    (= (total-cost) 0)
    (IS-GOAL pos-4-1)
    (IS-GOAL pos-4-2)
    (IS-NONGOAL pos-1-1) (IS-NONGOAL pos-2-1) (IS-NONGOAL pos-3-1)
    (IS-NONGOAL pos-5-1)
    (IS-NONGOAL pos-1-2) (IS-NONGOAL pos-2-2) (IS-NONGOAL pos-3-2)
    (IS-NONGOAL pos-5-2)
    (MOVE-DIR pos-1-1 pos-2-1 dir-right) (MOVE-DIR pos-2-1 pos-1-1 dir-left)
    (MOVE-DIR pos-2-1 pos-3-1 dir-right) (MOVE-DIR pos-3-1 pos-2-1 dir-left)
    (MOVE-DIR pos-3-1 pos-4-1 dir-right) (MOVE-DIR pos-4-1 pos-3-1 dir-left)
    (MOVE-DIR pos-4-1 pos-5-1 dir-right) (MOVE-DIR pos-5-1 pos-4-1 dir-left)
    (MOVE-DIR pos-1-2 pos-2-2 dir-right) (MOVE-DIR pos-2-2 pos-1-2 dir-left)
    (MOVE-DIR pos-2-2 pos-3-2 dir-right) (MOVE-DIR pos-3-2 pos-2-2 dir-left)
    (MOVE-DIR pos-3-2 pos-4-2 dir-right) (MOVE-DIR pos-4-2 pos-3-2 dir-left)
    (MOVE-DIR pos-4-2 pos-5-2 dir-right) (MOVE-DIR pos-5-2 pos-4-2 dir-left)
    (MOVE-DIR pos-1-1 pos-1-2 dir-down) (MOVE-DIR pos-1-2 pos-1-1 dir-up)
    (MOVE-DIR pos-2-1 pos-2-2 dir-down) (MOVE-DIR pos-2-2 pos-2-1 dir-up)
    (MOVE-DIR pos-3-1 pos-3-2 dir-down) (MOVE-DIR pos-3-2 pos-3-1 dir-up)
    (MOVE-DIR pos-4-1 pos-4-2 dir-down) (MOVE-DIR pos-4-2 pos-4-1 dir-up)
    (MOVE-DIR pos-5-1 pos-5-2 dir-down) (MOVE-DIR pos-5-2 pos-5-1 dir-up)
    (at player-01 pos-1-1)
    (at stone-01 pos-2-1)
    (at stone-02 pos-2-2)
    (clear pos-3-1) (clear pos-4-1) (clear pos-5-1)
    (clear pos-1-2) (clear pos-3-2) (clear pos-4-2) (clear pos-5-2))
  (:goal (and (at-goal stone-01) (at-goal stone-02)))
  (:metric minimize (total-cost)))
