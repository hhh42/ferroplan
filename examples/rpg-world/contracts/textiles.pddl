(define (problem textiles-clothing)
  (:domain rpg-world)
  (:objects
    weaver - agent
    home - location)
  (:init
    (at weaver home)
    (is-pasture home)
    (has-loom home)
    (= (dist home home) 0)
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0)
    (= (fiber) 0) (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (>= (clothing) 1)))
