;; PROTOTYPE — action-specific DURATIVE chain (K=3). Same domain name as the
;; instantaneous version so it shares the problem file; differs only in that each
;; craft takes time (input consumed at start, output appears at end), like rpg/.
(define (domain craft-specific)
  (:requirements :strips :durative-actions :numeric-fluents)
  (:functions (stock_r0) (stock_r1) (stock_r2) (stock_r3))

  (:durative-action craft_0 :parameters ()
    :duration (= ?duration 2)
    :condition (at start (>= (stock_r0) 1))
    :effect (and (at start (decrease (stock_r0) 1)) (at end (increase (stock_r1) 1))))
  (:durative-action craft_1 :parameters ()
    :duration (= ?duration 2)
    :condition (at start (>= (stock_r1) 1))
    :effect (and (at start (decrease (stock_r1) 1)) (at end (increase (stock_r2) 1))))
  (:durative-action craft_2 :parameters ()
    :duration (= ?duration 2)
    :condition (at start (>= (stock_r2) 1))
    :effect (and (at start (decrease (stock_r2) 1)) (at end (increase (stock_r3) 1)))))
