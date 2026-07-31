(define (domain visitgrid)
  (:requirements :strips)
  (:predicates (at ?x) (conn ?x ?y) (visited ?x))
  (:action move
    :parameters (?from ?to)
    :precondition (and (at ?from) (conn ?from ?to))
    :effect (and (not (at ?from)) (at ?to) (visited ?to))))
