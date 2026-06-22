(define (problem woodline-1)
  (:domain rpg-world)
  (:objects
    w1 - agent
    camp - location)
  (:init
    (at w1 camp)
    (woodcutter w1)
    (has-axe w1)
    (is-forest camp)
    (has-sawmill camp)
    (has-kiln camp)
    (= (dist camp camp) 0)
    (= (logs) 0)
    (= (planks) 0)
    (= (charcoal) 0))
  (:goal (and (>= (planks) 6) (>= (charcoal) 1))))
