(define (problem movement-reachability)
  (:domain rpg-world)
  (:objects
    miner1 - agent
    home n1 n2 n3 mine - location)
  (:init
    (at miner1 home)
    (miner miner1)
    (has-pick miner1)

    ;; linear chain: home -> n1 -> n2 -> n3 -> mine
    (link home n1)
    (link n1 n2)
    (link n2 n3)
    (link n3 mine)

    (is-mine mine)

    ;; distances for each hop the agent will actually take
    (= (dist home n1) 1)
    (= (dist n1 n2) 1)
    (= (dist n2 n3) 1)
    (= (dist n3 mine) 1)

    ;; resource fluents referenced
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0)
    (= (fiber) 0) (= (food) 0) (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (>= (ore) 2)))
