;; ε-emission order-inversion fixture (0.18 Phase 1; docs/roadmap-0.18.md).
;; The 2014 match-cellar shape, minimized: a light window that a mend fills
;; EXACTLY to the end, so mend-end and light-end land on the same internal
;; epoch. The tie-scan legally fires mend-end first; the emission pass must
;; preserve that order or the emitted mend overhangs the window by ε.
(define (domain epscross)
  (:requirements :typing :durative-actions)
  (:types match fuse)
  (:predicates (handfree) (unused ?m - match) (mended ?f - fuse) (light ?m - match))
  (:durative-action LIGHT_MATCH
    :parameters (?m - match)
    :duration (= ?duration 5)
    :condition (at start (unused ?m))
    :effect (and (at start (not (unused ?m)))
                 (at start (light ?m))
                 (at end (not (light ?m)))))
  (:durative-action MEND_FUSE
    :parameters (?f - fuse ?m - match)
    :duration (= ?duration 2.5)
    :condition (and (at start (handfree)) (over all (light ?m)))
    :effect (and (at start (not (handfree)))
                 (at end (mended ?f))
                 (at end (handfree)))))
