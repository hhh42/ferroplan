;; Numeric over-all invariant GAP fixture (0.15 Phase 2). `run` needs
;; (>= (level) 1) over its whole interval; `dip` (-2) is only possible
;; WHILE the run is hot, `refill` (+2) restores right after. From level 2
;; the eager schedule dips to 0 mid-run and passes BOTH endpoint checks
;; (level is back to 2 by the end) — VAL rejects it. The sound plan tops
;; up to 4 before starting (topup needs idle), so the mid-run dip
;; bottoms at 2 and the invariant holds throughout.
(define (domain fuel-gap)
  (:requirements :typing :durative-actions :numeric-fluents)
  (:types rig)
  (:predicates (idle) (hot) (done) (dipped) (refilled))
  (:functions (level))
  (:durative-action run
    :parameters (?r - rig)
    :duration (= ?duration 10)
    :condition (and (at start (idle)) (over all (>= (level) 1)))
    :effect (and (at start (not (idle))) (at start (hot))
                 (at end (not (hot))) (at end (done))))
  (:durative-action topup
    :parameters (?r - rig)
    :duration (= ?duration 1)
    :condition (at start (idle))
    :effect (at start (increase (level) 2)))
  (:durative-action dip
    :parameters (?r - rig)
    :duration (= ?duration 1)
    :condition (at start (hot))
    :effect (and (at start (dipped)) (at start (decrease (level) 2))))
  (:durative-action refill
    :parameters (?r - rig)
    :duration (= ?duration 1)
    :condition (at start (dipped))
    :effect (and (at start (refilled)) (at start (increase (level) 2)))))
