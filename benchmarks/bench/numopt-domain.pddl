;; Numeric-optimal soundness fixture (0.21 Phase 4, the ipc2026-opt
;; entry's pin): known optima on a genuinely numeric task, independent
;; of the corpus. charge only ever increases; fire needs charge >= 4.
;; The three problems pin the exact numeric goal test (p01), the
;; fluent-bearing StateKey + length optima without a metric (p02), and
;; numeric preconditions on the optimal path (p03).
(define (domain numopt)
  (:requirements :numeric-fluents :action-costs)
  (:predicates (done))
  (:functions (charge) (total-cost))
  (:action pump :parameters ()
    :precondition (and)
    :effect (and (increase (charge) 1) (increase (total-cost) 1)))
  (:action bigpump :parameters ()
    :precondition (and)
    :effect (and (increase (charge) 5) (increase (total-cost) 4)))
  (:action fire :parameters ()
    :precondition (>= (charge) 4)
    :effect (and (done) (increase (total-cost) 1))))
