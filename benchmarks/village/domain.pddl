;; The village (0.17 Phase 4) — ferroplan's abstract crafting-economy domain.
;;
;; THE CONTRACT (by direct request, docs/roadmap-0.17.md): the rules are
;; ABSTRACT and few — one gather, one make, one buy, one sell, movement and
;; tool pickup — and the entire content of a village lives in the INIT
;; DATA: which items exist, what each recipe consumes and produces and at
;; what quantities, which tool and station and skill it needs, what things
;; cost. A game ships content packs (catalogs), never new actions.
;;
;; Numeric fluents carry stock and money; durative actions carry labor
;; time. HIRING is deliberately NOT a domain action: a hire is the game
;; loop giving a worker's Session a goal contract (the bazaar pattern,
;; examples/village.rs) — the planner plans one mind's labor.
;;
;;   ff -o benchmarks/village/domain.pddl -f benchmarks/village/craftsman.pddl
;;   ff -o benchmarks/village/domain.pddl -f benchmarks/village/workshop.pddl
(define (domain village)
  (:requirements :typing :durative-actions :numeric-fluents)
  (:types worker item recipe station tool)
  (:predicates
    (at ?w - worker ?s - station)
    (adjacent ?a - station ?b - station)
    (holding ?w - worker ?t - tool)
    (tool-at ?t - tool ?s - station)
    ;; catalog statics: where raw items come from, what recipes need
    (source ?i - item ?s - station)
    (raw ?i - item)
    (in1 ?r - recipe ?i - item)
    (in2 ?r - recipe ?i - item)
    (out ?r - recipe ?i - item)
    (needs-tool ?r - recipe ?t - tool)
    (station-of ?r - recipe ?s - station)
    (market ?s - station)
    (for-sale ?i - item ?s - station))
  (:functions
    (stock ?i - item)          ; the worker's carried stock of an item kind
    (money)
    (qty1 ?r - recipe)         ; input quantities consumed
    (qty2 ?r - recipe)
    (yield ?r - recipe)        ; output quantity produced
    (gather-time ?i - item)
    (make-time ?r - recipe)
    (walk-time ?a - station ?b - station)
    (price ?i - item))         ; unit price at a market (buy and sell)

  (:durative-action walk
    :parameters (?w - worker ?a - station ?b - station)
    :duration (= ?duration (walk-time ?a ?b))
    :condition (and (at start (at ?w ?a)) (at start (adjacent ?a ?b)))
    :effect (and (at start (not (at ?w ?a))) (at end (at ?w ?b))))

  (:durative-action pickup-tool
    :parameters (?w - worker ?t - tool ?s - station)
    :duration (= ?duration 1)
    :condition (and (over all (at ?w ?s)) (at start (tool-at ?t ?s)))
    :effect (and (at start (not (tool-at ?t ?s))) (at end (holding ?w ?t))))

  ;; ONE gather rule: any raw item, at its source station.
  (:durative-action gather
    :parameters (?w - worker ?i - item ?s - station)
    :duration (= ?duration (gather-time ?i))
    :condition (and (over all (at ?w ?s)) (at start (raw ?i))
                    (at start (source ?i ?s)))
    :effect (at end (increase (stock ?i) 1)))

  ;; ONE make rule: the recipe parameter carries everything.
  (:durative-action make
    :parameters (?w - worker ?r - recipe ?a - item ?b - item ?o - item
                 ?t - tool ?s - station)
    :duration (= ?duration (make-time ?r))
    :condition (and (over all (at ?w ?s))
                    (at start (in1 ?r ?a)) (at start (in2 ?r ?b))
                    (at start (out ?r ?o))
                    (at start (needs-tool ?r ?t)) (at start (holding ?w ?t))
                    (at start (station-of ?r ?s))
                    (at start (>= (stock ?a) (qty1 ?r)))
                    (at start (>= (stock ?b) (qty2 ?r))))
    :effect (and (at start (decrease (stock ?a) (qty1 ?r)))
                 (at start (decrease (stock ?b) (qty2 ?r)))
                 (at end (increase (stock ?o) (yield ?r)))))

  ;; ONE buy rule and ONE sell rule: the marketplace prices are catalog data.
  (:durative-action buy
    :parameters (?w - worker ?i - item ?s - station)
    :duration (= ?duration 1)
    :condition (and (over all (at ?w ?s)) (at start (market ?s))
                    (at start (for-sale ?i ?s))
                    (at start (>= (money) (price ?i))))
    :effect (and (at start (decrease (money) (price ?i)))
                 (at end (increase (stock ?i) 1))))

  (:durative-action sell
    :parameters (?w - worker ?i - item ?s - station)
    :duration (= ?duration 1)
    :condition (and (over all (at ?w ?s)) (at start (market ?s))
                    (at start (>= (stock ?i) 1)))
    :effect (and (at start (decrease (stock ?i) 1))
                 (at end (increase (money) (price ?i))))))
