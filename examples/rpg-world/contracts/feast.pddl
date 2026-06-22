(define (problem cooking-feast)
  (:domain rpg-world)
  (:objects
    chef - agent
    home - location)
  (:init
    (at chef home)
    ;; cluster everything at one location
    (is-field home)     ; forage food here
    (is-water home)     ; draw water here
    (has-hearth home)   ; cook + feast here
    (cook chef)         ; cook bonus: meal yields 3
    ;; numeric fluents
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0)
    (= (dist home home) 0))
  (:goal (feast-held)))
