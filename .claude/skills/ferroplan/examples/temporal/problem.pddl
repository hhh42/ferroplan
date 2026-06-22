(define (problem forge-one)
  (:domain forge-temporal)
  (:objects
    widget - part
    vise - clamp)
  (:init
    (free vise))
  (:goal (forged widget)))
