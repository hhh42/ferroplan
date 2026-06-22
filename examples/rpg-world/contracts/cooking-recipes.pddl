(define (problem cooking-feast)
  (:domain rpg-world)
  (:objects
    chef helper - agent
    field kitchen - location)
  (:init
    ;; map (static link; reachable is derived)
    (link field kitchen) (link kitchen field)
    (= (dist field kitchen) 1) (= (dist kitchen field) 1)
    ;; positions
    (at chef kitchen) (at helper kitchen)
    ;; sites & stations (clustered for a fast plan)
    (is-field field)
    (has-mill kitchen) (has-hearth kitchen)
    ;; roles
    (cook chef) (baker chef)
    ;; pre-delivered raw inputs
    (= (water) 4) (= (food) 4)
    ;; cooking-recipes stockpiles
    (= (grain) 0) (= (flour) 0) (= (bread) 0) (= (stew) 0) (= (pies) 0)
    ;; reused stockpiles referenced by the chain
    (= (meals) 0)
    ;; remaining global fluents (initialized for completeness)
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (cloth) 0) (= (clothing) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  ;; Plan sketch: harvest-grain x2 (field) -> grind-flour (2 grain->2 flour) ->
  ;; bake-bread, cook-stew, bake-pie (baker/cook bonuses help) -> plate-spread x1+
  ;; until meals>=4 -> hold-grand-feast buffs chef & helper.
  (:goal (and (feast-held) (well-fed chef))))