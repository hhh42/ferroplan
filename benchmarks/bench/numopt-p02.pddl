;; Pure numeric goal, NO metric: the certificate is a LENGTH optimum
;; (the ipc2026-opt board's honest unit). States along the pump chain
;; differ ONLY in charge — a StateKey that drops fluents would merge
;; them all into init and "prove" unsolvable. Optimum: 2 steps
;; (bigpump + pump = charge 6).
(define (problem numopt-p02)
  (:domain numopt)
  (:init (= (charge) 0) (= (total-cost) 0))
  (:goal (>= (charge) 6)))
