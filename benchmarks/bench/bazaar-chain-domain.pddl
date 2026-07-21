(define (domain bazaar-chain)
  (:requirements :strips :typing)
  (:types holder item)
  (:predicates (has ?h - holder ?x - item) (wants ?h - holder ?x - item))
  ;; barter with preferences: unlike bazaar.pddl's any-for-any swap, a
  ;; vendor releases ?y ONLY for the ?x they want — depth-k goals force
  ;; k-hop trade-up chains (`wants` is static, so grounding prunes to the
  ;; wanted exchanges)
  (:action trade
    :parameters (?a ?b - holder ?x ?y - item)
    :precondition (and (has ?a ?x) (has ?b ?y) (wants ?b ?x))
    :effect (and (not (has ?a ?x)) (not (has ?b ?y))
                 (has ?a ?y) (has ?b ?x))))
