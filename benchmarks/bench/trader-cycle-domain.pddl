;; Mini markettrader (0.21 Phase 3 NEGATIVE control): profit exists only as a
;; CYCLE — buy consumes cash, sell restores more, but every sale needs a prior
;; buy and a paid trip to the other market. The delete-relaxed charge (goal
;; charge + one-level precondition charge) sees "a few sells reach the cash
;; goal" and stays FINITE while the real plan needs many buy-travel-sell laps;
;; the fixture pins that this cycle-blindness remains (LP-RPG's own paper
;; domain, field best 2/20 — re-attributed out of the winnable pot).
(define (domain trader-cycle)
    (:requirements :typing :numeric-fluents)
    (:types market goods - object)
    (:predicates (at ?m - market) (link ?a ?b - market))
    (:functions (cash)
                (capacity)
                (bought ?g - goods)
                (price ?g - goods ?m - market)
                (sellprice ?g - goods ?m - market)
                (fee ?a ?b - market))
    (:action travel
        :parameters (?a ?b - market)
        :precondition (and (at ?a) (link ?a ?b) (>= (cash) (fee ?a ?b)))
        :effect (and (not (at ?a)) (at ?b) (decrease (cash) (fee ?a ?b))))
    ;; capacity 1 forbids batching: every unit of profit is a full
    ;; buy-travel-sell-return lap, exactly the deep cycle the relaxation
    ;; cannot price.
    (:action buy
        :parameters (?g - goods ?m - market)
        :precondition (and (at ?m) (>= (capacity) 1) (>= (cash) (price ?g ?m)))
        :effect (and (decrease (capacity) 1)
                     (decrease (cash) (price ?g ?m))
                     (increase (bought ?g) 1)))
    (:action sell
        :parameters (?g - goods ?m - market)
        :precondition (and (at ?m) (>= (bought ?g) 1))
        :effect (and (increase (capacity) 1)
                     (increase (cash) (sellprice ?g ?m))
                     (decrease (bought ?g) 1)))
)
