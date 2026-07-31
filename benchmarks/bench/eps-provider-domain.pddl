;; 0.20 Phase 5 fixture: same-slot START pair where one start's at-start
;; add PROVIDES the other's at-start precondition (the map-analyzer
;; i17/i18/i20 shape, minimized). The epsilon-separation pass must emit
;; the provider's start strictly before the dependent's.
(define (domain epsprov)
  (:requirements :typing :durative-actions)
  (:predicates (clear) (moved))
  (:durative-action clearjunction
    :parameters ()
    :duration (= ?duration 3)
    :condition (and)
    :effect (and (at start (clear))))
  (:durative-action traverse
    :parameters ()
    :duration (= ?duration 2)
    :condition (at start (clear))
    :effect (and (at end (moved)))))
