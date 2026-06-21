;; Derived predicates (axioms): `reachable` is the transitive closure of the map's
;; `link` relation, computed once at grounding (the map is static). An agent can
;; `travel` only to a reachable point of interest — the "explore -> graph -> can I
;; get there?" primitive a game needs, without hand-listing every reachable pair.
(define (domain reachability)
  (:requirements :typing :adl)
  (:types poi)
  (:predicates
    (link ?a ?b - poi)        ; a directed path discovered between two POIs
    (reachable ?a ?b - poi)   ; derived: a path of one or more links from a to b
    (at ?p - poi))
  (:derived (reachable ?a ?b - poi)
    (or (link ?a ?b)
        (exists (?c - poi) (and (link ?a ?c) (reachable ?c ?b)))))
  (:action travel
    :parameters (?from ?to - poi)
    :precondition (and (at ?from) (reachable ?from ?to))
    :effect (and (not (at ?from)) (at ?to))))
