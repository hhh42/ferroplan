;; PROTOTYPE — action-specific instantaneous crafting chain (K=3).
;; One hardcoded action per recipe. r0 -> r1 -> r2 -> r3.
(define (domain craft-specific)
  (:requirements :strips :numeric-fluents)
  (:functions (stock_r0) (stock_r1) (stock_r2) (stock_r3))

  (:action craft_0 :parameters ()
    :precondition (>= (stock_r0) 1)
    :effect (and (decrease (stock_r0) 1) (increase (stock_r1) 1)))
  (:action craft_1 :parameters ()
    :precondition (>= (stock_r1) 1)
    :effect (and (decrease (stock_r1) 1) (increase (stock_r2) 1)))
  (:action craft_2 :parameters ()
    :precondition (>= (stock_r2) 1)
    :effect (and (decrease (stock_r2) 1) (increase (stock_r3) 1))))
