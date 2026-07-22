;; Over-all invariant GAP fixture (0.14 ext Phase 10). A bake needs
;; (ready) over its whole interval; the world (TILs) drops (ready) at t=8
;; and restores it at t=8.001. The endpoint-only invariant approximation
;; accepts a bake spanning the outage (start pre true, end pre true again),
;; which VAL rejects — the sound engine must schedule around the gap.
(define (domain kiln-gap)
  (:requirements :typing :durative-actions :timed-initial-literals)
  (:types piece)
  (:predicates (ready) (raw ?p - piece) (prepped ?p - piece) (baked ?p - piece))
  (:durative-action prep
    :parameters (?p - piece)
    :duration (= ?duration 6)
    :condition (at start (raw ?p))
    :effect (and (at start (not (raw ?p))) (at end (prepped ?p))))
  (:durative-action bake
    :parameters (?p - piece)
    :duration (= ?duration 3)
    :condition (and (at start (prepped ?p)) (over all (ready)))
    :effect (at end (baked ?p))))
