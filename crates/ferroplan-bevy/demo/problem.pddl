(define (problem deliver)
  (:domain driving)
  (:objects
    depot market garage harbor airport - location
    rig - truck
    crate1 crate2 - package)
  (:init
    ; a connected road network (bidirectional)
    (road depot market) (road market depot)
    (road market garage) (road garage market)
    (road garage harbor) (road harbor garage)
    (road depot airport) (road airport depot)
    (road airport harbor) (road harbor airport)
    ; start state
    (truck-at rig depot)
    (pkg-at crate1 market)
    (pkg-at crate2 harbor))
  (:goal (and
    (pkg-at crate1 garage)
    (pkg-at crate2 depot))))
