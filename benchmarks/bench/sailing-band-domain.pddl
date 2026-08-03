;; Mini sailing-band domain (0.21 Phase 3 fixture): the numeric-precondition
;; charge's RED/GREEN witness. Propositional goal (saved ?p) reachable ONLY
;; through save's four band preconditions — the (>= (+ x y) (d)) diagonal-band
;; shape of IPC-2023 sailing. Movement rates match the real domain's NE/NW/E/W/S
;; subset so the charge's repetition counts are exact.
(define (domain sailing-band)
    (:requirements :typing :numeric-fluents)
    (:types boat person - object)
    (:predicates (saved ?t - person) (crewed ?b - boat))
    (:functions (x ?b - boat) (y ?b - boat) (d ?t - person))
    (:action go_north_east
        :parameters (?b - boat)
        :effect (and (increase (x ?b) 1.5) (increase (y ?b) 1.5)))
    (:action go_north_west
        :parameters (?b - boat)
        :effect (and (decrease (x ?b) 1.5) (increase (y ?b) 1.5)))
    (:action go_est
        :parameters (?b - boat)
        :effect (increase (x ?b) 3))
    (:action go_west
        :parameters (?b - boat)
        :effect (decrease (x ?b) 3))
    (:action go_south
        :parameters (?b - boat)
        :effect (decrease (y ?b) 2))
    ;; (crewed ?b) keeps the fact achiever unique — only b0 can save, so the
    ;; unit pin's charged h(init) is deterministic while b1 still widens the
    ;; state space for the end-to-end cap fixture.
    (:action save_person
        :parameters (?b - boat ?t - person)
        :precondition (and (crewed ?b)
                           (>= (+ (x ?b) (y ?b)) (d ?t))
                           (>= (- (y ?b) (x ?b)) (d ?t))
                           (<= (+ (x ?b) (y ?b)) (+ (d ?t) 25))
                           (<= (- (y ?b) (x ?b)) (+ (d ?t) 25)))
        :effect (saved ?t))
)
