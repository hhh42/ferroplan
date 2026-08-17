;; Mini chained-band domain (0.24 Phase 6 fixture): the a2 CHAINED charge's
;; RED/GREEN witness — the pathwaysmetric shape the 0.21 a1 charge does not
;; reach. The goal sits behind a three-deep numeric supply chain: finish
;; needs stock-c, whose only raiser is gated on stock-b, whose only raiser
;; is gated on stock-a. a1 charges exactly ONE level (finish's c-gap prices
;; make-c ×4, h(init)=5) and the b/a legs contribute ZERO — h is flat across
;; the whole 17-step approach and the drift dimensions blow the plateau ball
;; past any small cap. The chained charge recurses the charged achievers'
;; own pre_num (depth-capped), pricing the full chain: h(init) = 1 + 4 + 5
;; + 12 = 22 — the exact optimum — and every make-a step descends.
(define (domain chained-band)
    (:requirements :typing :numeric-fluents)
    (:predicates (saved))
    (:functions (stock-a) (stock-b) (stock-c) (drift-x) (drift-y) (drift-z))
    (:action finish
        :parameters ()
        :precondition (>= (stock-c) 12)
        :effect (saved))
    (:action make-c
        :parameters ()
        :precondition (>= (stock-b) 10)
        :effect (increase (stock-c) 3))
    (:action make-b
        :parameters ()
        :precondition (>= (stock-a) 12)
        :effect (increase (stock-b) 2))
    (:action make-a
        :parameters ()
        :effect (increase (stock-a) 1))
    ;; Three goal-irrelevant counters: under the flat a1 h the search has no
    ;; reason not to drift, so the plateau ball is 6-dimensional and a small
    ;; eval cap dies honestly; under the chained gradient the relaxed plan
    ;; never selects them and helpful-restricted descent ignores them. The
    ;; trivially-true precondition is LOAD-BEARING: a fluent no precondition
    ;; reads never enters rel_fluents, the visited key ignores it, and the
    ;; drift dimension dedups away to nothing (measured: the 6-dim ball
    ;; collapsed to the bare chain and EHC walked it in 23 evals flat).
    (:action wander-x
        :parameters ()
        :precondition (>= (drift-x) 0)
        :effect (increase (drift-x) 1))
    (:action wander-y
        :parameters ()
        :precondition (>= (drift-y) 0)
        :effect (increase (drift-y) 1))
    (:action wander-z
        :parameters ()
        :precondition (>= (drift-z) 0)
        :effect (increase (drift-z) 1)))
