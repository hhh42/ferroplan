(define (problem house-and-square)
  (:domain rpg-world)
  (:objects
    w - agent
    hub - location
    s1 s2 - slot)
  (:init
    (at w hub)
    ;; workstations all at hub
    (has-sawmill hub)
    (has-kiln hub)
    (has-mason hub)
    (is-buildsite hub)
    (mason-skill w)
    ;; pre-stocked raw materials: 4 logs -> 8 planks, 4 clay -> 4 bricks, 2 stone -> 4 blocks
    (= (logs) 4) (= (clay) 4) (= (stone) 2)
    ;; other numeric fluents
    (= (ore) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0) (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (built-square)))
