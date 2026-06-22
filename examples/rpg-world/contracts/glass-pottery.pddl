(define (problem glass-pottery-contract)
  (:domain rpg-world)
  (:objects
    pat - agent
    hub - location)
  (:init
    ;; agent placement
    (at pat hub)
    ;; trivially-true reachability seed (agent already at hub; no travel needed)
    (link hub hub)
    ;; sites + stations clustered at the hub
    (is-sandpit hub)
    (has-kiln hub)
    (has-wheel hub)
    ;; tool + role
    (has-shovel pat)
    (potter pat)
    ;; --- numeric fluents (initialize everything referenced) ---
    (= (dist hub hub) 1)
    ;; raw materials
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 2) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    ;; processed goods
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (cloth) 0) (= (clothing) 0) (= (meals) 0) (= (potions) 0)
    ;; tool stock
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    ;; economy + magic
    (= (coin) 0) (= (mana) 0)
    ;; glass & pottery line
    (= (sand) 0) (= (glass) 0) (= (pottery) 0) (= (fired-pottery) 0))
  ;; Plan: dig-sand x2 -> melt-glass (1 glass);
  ;;       throw-pottery (clay 2 -> pottery 2 via potter bonus on each? no:
  ;;       potter bonus gives +1 per throw, so throw-pottery x1 from 1 clay
  ;;       yields 2 pottery) -> fire-pottery x2 (2 fired-pottery).
  (:goal (and (>= (glass) 1) (>= (fired-pottery) 2)))
  (:metric minimize (total-time)))