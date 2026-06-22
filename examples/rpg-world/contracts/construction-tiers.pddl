(define (problem construction-tiers-contract)
  (:domain rpg-world)
  (:objects
    bob - agent
    yard - location)
  (:init
    (at bob yard)
    ;; bob has the needed role + tool
    (builder bob)
    (priest bob)
    (has-hammer bob)
    ;; the yard is a buildsite and pre-equipped with the two binder stations
    (is-buildsite yard)
    (has-sawmill yard)
    (has-anvil yard)
    ;; --- pre-delivered raw + processed inputs from other subsystems ---
    ;; binders: need plenty of planks (frames) + ingots (fittings)
    (= (planks) 30)
    (= (ingots) 10)
    ;; station/monument raw materials
    (= (blocks) 30)
    (= (bricks) 20)
    (= (stone) 12)
    ;; --- every other referenced fluent initialized ---
    (= (logs) 0) (= (ore) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (charcoal) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0)
    ;; construction-tiers binders start empty
    (= (timber-frame) 0) (= (cut-fittings) 0)
    ;; trivial self-link so any (dist)/reachable refs are well-formed (no travel needed)
    (link yard yard)
    (= (dist yard yard) 1))
  (:goal (and
    (has-forge yard)        ; build a forge from frame+fittings+bricks+stone
    (has-loom yard)         ; build a loom from frame+fittings+planks
    (has-altar yard)        ; raise altar (prereq for temple)
    (built-watchtower)      ; civic monument
    (built-temple)))        ; consecrated by the priest, needs the altar first
  )