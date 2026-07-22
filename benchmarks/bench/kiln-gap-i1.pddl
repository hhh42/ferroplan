;; prep runs 0..6, so the tempting bake start (6.001) spans the 8..8.001
;; outage; the only VALID bakes start at or after 8.002 (or end by 8,
;; impossible with prepped at 6). Endpoint-only invariants take the bait.
(define (problem kiln-gap-1)
  (:domain kiln-gap)
  (:objects p1 - piece)
  (:init (raw p1) (ready)
         (at 8 (not (ready)))
         (at 8.001 (ready)))
  (:goal (baked p1))
  (:metric minimize (total-time)))
