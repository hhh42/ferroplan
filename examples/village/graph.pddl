;; Smallest sanity instance: one craftsman, one house slot, a 3-node map.
(define (problem village1) (:domain village)
  (:objects c1 - agent  camp forest quarry - location  s1 - slot)
  (:init
    (at c1 camp)
    (is-camp camp) (is-forest forest) (is-quarry quarry)
    (road camp forest) (road forest camp)
    (trail camp quarry) (trail quarry camp)
    (= (dist camp forest) 2) (= (dist forest camp) 2)
    (= (dist camp quarry) 3) (= (dist quarry camp) 3)
    (has-axe c1) (has-pick c1) (skilled c1)
    (= (wood) 0) (= (stone) 0) (= (sticks) 0)
    (= (chops-left forest) 20))
  (:goal (and (square-built) (fire-lit))))
