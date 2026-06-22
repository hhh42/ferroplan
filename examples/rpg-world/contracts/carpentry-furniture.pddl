(define (problem carpentry-contract-1)
  (:domain rpg-world)
  (:objects
    bob - agent
    w1 - location)
  (:init
    ;; agent placement + map (single node; reachable not needed here)
    (at bob w1)
    (link w1 w1)
    ;; station + tool + role
    (has-carpenter-bench w1)
    (has-hammer bob)
    (carpenter bob)
    ;; pre-delivered raw input from the carpentry/sawmill subsystem
    (= (planks) 12)
    ;; every other fluent this subsystem (and the recipes) touch, initialized
    (= (furniture) 0)
    (= (barrels) 0)
    (= (tool-handles) 0)
    (= (cart-parts) 0)
    ;; baseline fluents referenced by the domain, kept at zero
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (cloth) 0) (= (clothing) 0) (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0)
    (= (dist w1 w1) 1))
  (:goal (and (>= (furniture) 1)
              (>= (barrels) 1)
              (>= (cart-parts) 1))))