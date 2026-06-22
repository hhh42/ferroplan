(define (domain courier)
  (:requirements :strips :typing :negative-preconditions :conditional-effects :universal-preconditions)
  (:types parcel location - object)
  (:predicates
    (at-courier ?l - location)
    (parcel-at ?p - parcel ?l - location)
    (held ?p - parcel)
    (delivered ?p - parcel)
    (depot ?l - location))

  ;; One move action. The (when ...) conditional effect carries every
  ;; held parcel along — no per-parcel "move-while-carrying-X" variants.
  (:action move
    :parameters (?from - location ?to - location)
    :precondition (and (at-courier ?from) (not (= ?from ?to)))
    :effect (and
      (at-courier ?to)
      (not (at-courier ?from))
      (forall (?p - parcel)
        (when (held ?p)
          (and (parcel-at ?p ?to) (not (parcel-at ?p ?from)))))))

  (:action pick
    :parameters (?p - parcel ?l - location)
    :precondition (and (at-courier ?l) (parcel-at ?p ?l) (not (held ?p)))
    :effect (and (held ?p) (not (parcel-at ?p ?l))))

  ;; Deliver only at a depot (negative precondition: not already delivered).
  (:action drop
    :parameters (?p - parcel ?l - location)
    :precondition (and (at-courier ?l) (held ?p) (depot ?l) (not (delivered ?p)))
    :effect (and (parcel-at ?p ?l) (not (held ?p)) (delivered ?p))))
