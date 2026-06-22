(define (problem multi-agent-contract)
  (:domain rpg-world)
  (:objects
    w1 w2 - agent
    yard - location)
  (:init
    (at w1 yard)
    (at w2 yard)
    ;; clustered stations at one location
    (has-sawmill yard)
    (has-mason yard)
    ;; tools/roles
    (mason-skill w2)
    ;; numeric fluents
    (= (dist yard yard) 0)
    (= (logs) 6) (= (ore) 0) (= (stone) 6) (= (clay) 0)
    (= (fiber) 0) (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (and (>= (planks) 12) (>= (blocks) 9))))
