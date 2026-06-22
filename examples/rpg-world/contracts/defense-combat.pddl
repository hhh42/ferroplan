(define (problem defend-keep)
  (:domain rpg-world)
  (:objects
    grog - agent
    keep - location)
  (:init
    (at grog keep)
    ;; map: trivial self-loop so reachability is well-defined (no travel needed)
    (link keep keep)
    (dist keep keep) ; numeric fluent referenced by travel; init to 0 below
    ;; stations clustered at the keep
    (has-anvil keep)
    (has-trainyard keep)
    (has-watchtower keep)
    (has-hearth keep)
    ;; agent's tools/role
    (has-hammer grog)
    ;; the threat to clear
    (threat keep)
    ;; pre-delivered raw inputs from other subsystems
    (= (ingots) 1)
    (= (planks) 1)
    (= (meals) 2)
    ;; defense fluents referenced
    (= (weapon-stock) 0)
    (= (ration) 0)
    ;; init every other referenced numeric fluent to keep the problem well-formed
    (= (dist keep keep) 0)
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (charcoal) 0) (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (and (cleared keep)))
  (:metric minimize (total-time)))