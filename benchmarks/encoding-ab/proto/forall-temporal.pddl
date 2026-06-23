;; PROTOTYPE — generic forall-numeric DURATIVE (G2). The riskiest combo: forall
;; over resources inside a durative condition/effect. Same domain name so it
;; shares the problem file.
(define (domain craft-forall)
  (:requirements :typing :durative-actions :numeric-fluents :universal-preconditions :conditional-effects)
  (:types recipe resource - object)
  (:constants c0 c1 c2 - recipe   r0 r1 r2 r3 - resource)
  (:functions (stock ?res - resource)
              (need ?rec - recipe ?res - resource)
              (make ?rec - recipe ?res - resource))

  (:durative-action craft
    :parameters (?rec - recipe)
    :duration (= ?duration 2)
    :condition (at start (forall (?res - resource) (>= (stock ?res) (need ?rec ?res))))
    :effect (and (at start (forall (?res - resource) (decrease (stock ?res) (need ?rec ?res))))
                 (at end (forall (?res - resource) (increase (stock ?res) (make ?rec ?res)))))))
