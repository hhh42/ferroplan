;; level 2: the eager plan dips to 0 inside the run (endpoint checks both
;; pass; VAL-red). Sound: topup to 4 while idle, then run + dip + refill.
(define (problem fuel-gap-1)
  (:domain fuel-gap)
  (:objects r1 - rig)
  (:init (idle) (= (level) 2))
  (:goal (and (done) (dipped) (refilled)))
  (:metric minimize (total-time)))
