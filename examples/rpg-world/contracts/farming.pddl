(define (problem farm-contract-1)
  (:domain rpg-world)
  (:objects
    farmer1 - agent
    homestead - location)
  (:init
    (at farmer1 homestead)
    ;; clustered stations / site
    (is-field homestead)
    (has-mill homestead)
    (has-granary homestead)
    (has-hearth homestead)
    ;; agent equipment + role
    (has-shovel farmer1)
    (farmer farmer1)
    (cook farmer1)
    ;; map: a trivial self-reachable node (link is static)
    (link homestead homestead)
    (= (dist homestead homestead) 0)
    ;; every fluent referenced by the farming fragments, initialized
    (= (seeds) 2)
    (= (grain) 0)
    (= (flour) 0)
    (= (grain-reserve) 0)
    (= (water) 4)
    (= (meals) 0)
    ;; remaining global stockpiles touched indirectly / for completeness
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  ;; Goal: a baked-bread surplus and a banked grain reserve.
  ;; Path: till -> plant -> irrigate -> harvest (grain 2->3 w/ farmer bonus, +1 seed)
  ;;       repeat to accrue >=2 grain to mill + >=3 grain to store; mill -> bake.
  (:goal (and (>= (meals) 2) (>= (grain-reserve) 3))))