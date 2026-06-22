(define (problem deliver-pkgs)
  (:domain delivery)
  (:objects
    pkg1 pkg2 - item
    depot - location)
  (:init
    (at pkg1 depot)
    (at pkg2 depot))
  (:goal (and
    (delivered pkg1)
    (preference p-deliver2 (delivered pkg2))))
  (:metric minimize (is-violated p-deliver2)))
