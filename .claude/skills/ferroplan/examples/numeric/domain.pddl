(define (domain fuel-travel)
  (:requirements :strips :typing :numeric-fluents)
  (:types location)
  (:predicates (at ?l - location))
  (:functions (fuel) (cost ?l - location))

  (:action drive
    :parameters (?from - location ?to - location)
    :precondition (and (at ?from) (>= (fuel) (cost ?to)))
    :effect (and (not (at ?from)) (at ?to)
                 (decrease (fuel) (cost ?to))))

  (:action refuel
    :parameters ()
    :precondition (< (fuel) 10)
    :effect (increase (fuel) 5)))
