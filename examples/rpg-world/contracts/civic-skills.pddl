(define (problem civic-skills-contract)
  (:domain rpg-world)

  (:objects
    mira tomas - agent
    haven - location)

  (:init
    ;; --- where everyone stands (single hub, trivially reachable to itself) ---
    (at mira haven)
    (at tomas haven)
    (link haven haven)
    (= (dist haven haven) 1)

    ;; --- stations co-located at the hub ---
    (has-hall haven)
    (is-market haven)

    ;; --- mira is the trainee apprentice; tomas is the seasoned organizer ---
    (apprentice mira)

    ;; --- pre-delivered raw inputs from other subsystems ---
    (= (coin) 6)        ; for market day (2) + festival (1) + slack
    (= (meals) 4)       ; for the festival
    (= (lore) 0)
    (= (renown) 0)

    ;; --- every other fluent this subsystem could touch, initialized ---
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (cloth) 0) (= (clothing) 0) (= (meals) 4) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (mana) 0))

  ;; A short plan: study-lore x several (tomas can master-mentor later), train mira
  ;; as a smith, run a market day to earn renown, attain mastery, hold a festival.
  (:goal (and (smith mira)
              (master tomas)
              (market-day-held)
              (festival-held))))