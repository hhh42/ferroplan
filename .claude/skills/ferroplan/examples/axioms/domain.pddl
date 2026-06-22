(define (domain reach)
  (:requirements :strips :typing :adl)
  (:types node)
  (:predicates
    (link ?a - node ?b - node)      ; static directed edge
    (reachable ?a - node ?b - node) ; derived: transitive closure of link
    (visited ?n - node))

  ;; reachable = transitive closure of the static `link` relation.
  ;; Defined ONLY over static base facts, so ferroplan closes it into init.
  (:derived (reachable ?a - node ?b - node)
    (link ?a ?b))
  (:derived (reachable ?a - node ?b - node)
    (exists (?m - node) (and (link ?a ?m) (reachable ?m ?b))))

  ;; A node is only visitable if it is (transitively) reachable from a visited one.
  (:action go
    :parameters (?from - node ?to - node)
    :precondition (and (visited ?from) (reachable ?from ?to))
    :effect (visited ?to)))
