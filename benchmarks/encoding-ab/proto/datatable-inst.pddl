;; PROTOTYPE — generic data-table instantaneous (G1). One craft action; recipes
;; are constants; the (recipe ?rec ?in ?out) data table (supplied in :init) "finds
;; the types". Closest analog to the Prohibited `consume` idiom.
(define (domain craft-data-table)
  (:requirements :strips :typing :numeric-fluents)
  (:types recipe resource - object)
  (:constants c0 c1 c2 - recipe   r0 r1 r2 r3 - resource)
  (:predicates (recipe ?rec - recipe ?in - resource ?out - resource))
  (:functions (stock ?res - resource))

  (:action craft
    :parameters (?rec - recipe ?in - resource ?out - resource)
    :precondition (and (recipe ?rec ?in ?out) (>= (stock ?in) 1))
    :effect (and (decrease (stock ?in) 1) (increase (stock ?out) 1))))
