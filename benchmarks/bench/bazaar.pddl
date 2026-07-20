(define (domain bazaar)
  (:requirements :strips :typing)
  (:types holder item)
  (:predicates (has ?h - holder ?x - item))
  ;; the recorded game-economy shape: ANY item for ANY item, money is just
  ;; another item — the item x item action space is the grounding stress case
  (:action trade
    :parameters (?a ?b - holder ?x ?y - item)
    :precondition (and (has ?a ?x) (has ?b ?y))
    :effect (and (not (has ?a ?x)) (not (has ?b ?y))
                 (has ?a ?y) (has ?b ?x))))
