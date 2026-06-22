(define (domain gripper)
  (:requirements :strips :typing)
  (:types robot ball room)
  (:predicates
    (at-robot ?r - robot ?x - room)
    (at-ball ?b - ball ?x - room)
    (holding ?r - robot ?b - ball)
    (free ?r - robot))

  (:action move
    :parameters (?r - robot ?from - room ?to - room)
    :precondition (at-robot ?r ?from)
    :effect (and (at-robot ?r ?to) (not (at-robot ?r ?from))))

  (:action pick
    :parameters (?r - robot ?b - ball ?x - room)
    :precondition (and (at-robot ?r ?x) (at-ball ?b ?x) (free ?r))
    :effect (and (holding ?r ?b) (not (at-ball ?b ?x)) (not (free ?r))))

  (:action drop
    :parameters (?r - robot ?b - ball ?x - room)
    :precondition (and (at-robot ?r ?x) (holding ?r ?b))
    :effect (and (at-ball ?b ?x) (free ?r) (not (holding ?r ?b)))))
