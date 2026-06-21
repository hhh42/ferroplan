; A tiny ADL domain to exercise the editor's or-preconditions + conditional effects.
(define (domain switches)
  (:requirements :strips :typing :negative-preconditions
                 :disjunctive-preconditions :conditional-effects)
  (:types loc switch)
  (:predicates
    (at ?l - loc)
    (on ?s - switch)
    (powered ?l - loc)
    (link ?s - switch ?l - loc))

  (:action toggle
    :parameters (?s - switch ?l - loc)
    :precondition (or (at ?l) (on ?s))
    :effect (and (on ?s) (when (link ?s ?l) (powered ?l))))

  (:action move
    :parameters (?from ?to - loc)
    :precondition (at ?from)
    :effect (and (not (at ?from)) (at ?to))))
