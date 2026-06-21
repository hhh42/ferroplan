; A small delivery domain for the visualizer demo: locations connected by roads,
; a truck that drives between them, packages it loads/unloads.
(define (domain driving)
  (:requirements :strips :typing)
  (:types location truck package)
  (:predicates
    (road ?a ?b - location)
    (truck-at ?t - truck ?l - location)
    (pkg-at ?p - package ?l - location)
    (in ?p - package ?t - truck))

  (:action drive
    :parameters (?t - truck ?from ?to - location)
    :precondition (and (truck-at ?t ?from) (road ?from ?to))
    :effect (and (not (truck-at ?t ?from)) (truck-at ?t ?to)))

  (:action load
    :parameters (?p - package ?t - truck ?l - location)
    :precondition (and (pkg-at ?p ?l) (truck-at ?t ?l))
    :effect (and (not (pkg-at ?p ?l)) (in ?p ?t)))

  (:action unload
    :parameters (?p - package ?t - truck ?l - location)
    :precondition (and (in ?p ?t) (truck-at ?t ?l))
    :effect (and (not (in ?p ?t)) (pkg-at ?p ?l))))
