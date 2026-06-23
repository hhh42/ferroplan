;; PROTOTYPE — generic forall-numeric instantaneous (G2). One craft action whose
;; precondition/effect quantify over ALL resources, reading per-recipe (need ?rec
;; ?res)/(make ?rec ?res) quantity functions. "Fat" ADL operators.
(define (domain craft-forall)
  (:requirements :typing :numeric-fluents :universal-preconditions :conditional-effects)
  (:types recipe resource - object)
  (:constants c0 c1 c2 - recipe   r0 r1 r2 r3 - resource)
  (:functions (stock ?res - resource)
              (need ?rec - recipe ?res - resource)
              (make ?rec - recipe ?res - resource))

  (:action craft
    :parameters (?rec - recipe)
    :precondition (forall (?res - resource) (>= (stock ?res) (need ?rec ?res)))
    :effect (forall (?res - resource)
              (and (decrease (stock ?res) (need ?rec ?res))
                   (increase (stock ?res) (make ?rec ?res))))))
