;; The gap-60 pump chain (0.22 Phase 4 fixture, paired with numopt-p04):
;; ONE +1 pump on (charge), plus eight independent set-only junk toggles
;; that widen the blind state space by 2^8 without touching the numeric
;; gradient. Pure numeric goal, NO metric — exactly the onlycraft shape
;; (prop h reads 0, blind Dijkstra pays the whole product) and exactly
;; the length-currency arm condition. The +1-per-layer chain makes the
;; layer counter's arithmetic transparent: from charge 0 toward gap 60,
;; admissible_goal_layers(init) must read GoalAt(60) — 59 is an
;; unsound-in-the-other-direction weakening and 61 is INADMISSIBLE
;; (the true optimum IS 60 pumps) — the off-by-one pinned forever.
(define (domain numopt-pump)
  (:requirements :numeric-fluents)
  (:predicates (jt0) (jt1) (jt2) (jt3) (jt4) (jt5) (jt6) (jt7))
  (:functions (charge))
  (:action pump :parameters ()
    :precondition (and)
    :effect (increase (charge) 1))
  (:action flip0 :parameters () :precondition (and) :effect (jt0))
  (:action flip1 :parameters () :precondition (and) :effect (jt1))
  (:action flip2 :parameters () :precondition (and) :effect (jt2))
  (:action flip3 :parameters () :precondition (and) :effect (jt3))
  (:action flip4 :parameters () :precondition (and) :effect (jt4))
  (:action flip5 :parameters () :precondition (and) :effect (jt5))
  (:action flip6 :parameters () :precondition (and) :effect (jt6))
  (:action flip7 :parameters () :precondition (and) :effect (jt7)))
