;; Mini ext-plant-watering (0.22 Phase 1 fixture): the 0.21 charge's BILL.
;; The IPC-2023 ext-plant-watering skeleton, distilled: numeric Eq goals
;; (= (poured ?p) k) behind coordinate-Eq pour gates, one shared agent, a
;; tap to shuttle to. Several plants price the SAME move op through the
;; charge pass; `select`'s dedup kept whichever gap came FIRST in op order,
;; so the priced plant's identity flips as the agent walks — arriving at a
;; near plant re-points the charge at a far one and h JUMPS UP on progress
;; (ext-plant-watering i7: 0.15 s at 0.20 -> 2.9M evals unsolved at 0.21).
;; The damped charge prices a shared achiever at the SUM of its gaps
;; instead: every arrival retires its own term smoothly and each step
;; toward any unmet gate still descends.
(define (domain watering-line)
    (:requirements :typing :numeric-fluents)
    (:types agent plant tap - locatable)
    (:functions (x ?o - locatable) (y ?o - locatable)
                (carrying ?a - agent) (poured ?p - plant)
                (max_carry ?a - agent) (maxx) (maxy))
    (:action move_up
        :parameters (?a - agent)
        :precondition (<= (+ (y ?a) 1) (maxy))
        :effect (increase (y ?a) 1))
    (:action move_down
        :parameters (?a - agent)
        :precondition (>= (- (y ?a) 1) 0)
        :effect (decrease (y ?a) 1))
    (:action move_right
        :parameters (?a - agent)
        :precondition (<= (+ (x ?a) 1) (maxx))
        :effect (increase (x ?a) 1))
    (:action move_left
        :parameters (?a - agent)
        :precondition (>= (- (x ?a) 1) 0)
        :effect (decrease (x ?a) 1))
    (:action load
        :parameters (?a - agent ?t - tap)
        :precondition (and (= (x ?a) (x ?t)) (= (y ?a) (y ?t))
                           (<= (+ (carrying ?a) 1) (max_carry ?a)))
        :effect (increase (carrying ?a) 1))
    (:action pour
        :parameters (?a - agent ?p - plant)
        :precondition (and (= (x ?a) (x ?p)) (= (y ?a) (y ?p))
                           (>= (carrying ?a) 1))
        :effect (and (decrease (carrying ?a) 1) (increase (poured ?p) 1)))
)
