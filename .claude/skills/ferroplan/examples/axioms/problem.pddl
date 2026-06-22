(define (problem reach-chain)
  (:domain reach)
  (:objects a b c d - node)
  (:init
    (link a b)
    (link b c)
    (link c d)
    (visited a))
  ;; d is only reachable from a TRANSITIVELY (a->b->c->d): exercises the axiom.
  (:goal (visited d)))
