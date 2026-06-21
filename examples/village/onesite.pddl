;; Camp is also forest+quarry (no travel): isolate NUMERIC ACCUMULATION search.
(define (problem village3) (:domain village)
  (:objects c1 - agent  camp - location  s1 - slot)
  (:init (at c1 camp) (is-camp camp) (is-forest camp) (is-quarry camp)
    (has-axe c1) (has-pick c1) (skilled c1)
    (= (wood) 0) (= (stone) 0) (= (sticks) 0) (= (chops-left camp) 20))
  (:goal (and (square-built) (fire-lit))))
