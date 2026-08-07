;; Gap 60 from charge 0 at +1 per pump: optimum 60 steps, length
;; currency (no metric). Blind (FF_OPT_NO_NUMH=1) the proof pays the
;; toggles' 2^8 product — every state with g < 60 pops first, ~14k
;; expansions; armed, num_h = 60 - charge is EXACT, every toggle
;; successor sits at f = 61 > 60, and the frontier collapses to the
;; pump chain (~61 expansions — two orders). Same certified cost both
;; ways, PROVEN note naming "+numRPG" when armed: the RED/GREEN pair
;; tests/numopt_arm.rs runs as child scenarios.
(define (problem numopt-p04)
  (:domain numopt-pump)
  (:init (= (charge) 0))
  (:goal (>= (charge) 60)))
