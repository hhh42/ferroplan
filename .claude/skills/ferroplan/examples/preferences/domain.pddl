(define (domain delivery)
  (:requirements :strips :typing :preferences)
  (:types item location)
  (:predicates
    (at ?i - item ?l - location)
    (delivered ?i - item))

  (:action deliver
    :parameters (?i - item ?l - location)
    :precondition (at ?i ?l)
    :effect (delivered ?i)))
