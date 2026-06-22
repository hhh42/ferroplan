(define (problem masonry-build-sub)
  (:domain rpg-world)
  (:objects
    mira - agent
    yard - location)
  (:init
    (at mira yard)
    ;; site hosts quarry + mason station + buildsite, all clustered
    (is-quarry yard)
    (has-mason yard)
    (is-buildsite yard)
    ;; tools + role
    (has-pick mira)
    (mason-skill mira)
    ;; numeric fluents
    (= (dist yard yard) 0)
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0) (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (and (built-wall) (well-dug))))
