;; PROTOTYPE — generic data-table DURATIVE (G1). Same domain name as the
;; instantaneous version so it shares the problem file.
(define (domain craft-data-table)
  (:requirements :strips :typing :durative-actions :numeric-fluents)
  (:types recipe resource - object)
  (:constants c0 c1 c2 - recipe   r0 r1 r2 r3 - resource)
  (:predicates (recipe ?rec - recipe ?in - resource ?out - resource))
  (:functions (stock ?res - resource))

  (:durative-action craft
    :parameters (?rec - recipe ?in - resource ?out - resource)
    :duration (= ?duration 2)
    :condition (and (at start (recipe ?rec ?in ?out)) (at start (>= (stock ?in) 1)))
    :effect (and (at start (decrease (stock ?in) 1)) (at end (increase (stock ?out) 1)))))
