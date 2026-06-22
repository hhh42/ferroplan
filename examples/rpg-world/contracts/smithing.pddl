(define (problem smithing-forge-two-axes)
  (:domain rpg-world)
  ;; SMITHING contract: a lone smith at a fully-equipped smithy turns a small
  ;; pre-delivered stock of raw materials (logs + ore) into 2 forged axes.
  ;; Gathering (chop/mine) is handled by separate contracts, so raws are in :init.
  (:objects
    smithy - location
    bron - agent)
  (:init
    (at bron smithy)
    ;; one hub hosts every station this contract needs
    (has-sawmill smithy)
    (has-kiln smithy)
    (has-forge smithy)
    (has-anvil smithy)
    ;; tools + smith role (smith doubles ingot yield: 1 smelt -> 2 ingots)
    (has-hammer bron)
    (smith bron)
    ;; pre-delivered raw stock
    (= (logs) 3)        ; 1 -> 2 planks (saw); 2 -> 1 charcoal (kiln)
    (= (ore) 1)         ; ore + charcoal -> 2 ingots (smelt, smith bonus)
    ;; all other fluents initialized to 0
    (= (stone) 0) (= (clay) 0) (= (fiber) 0) (= (food) 0)
    (= (water) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0)
    (= (bricks) 0) (= (cloth) 0) (= (clothing) 0)
    (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  ;; forge two axes into the shared tool stock
  (:goal (>= (axe-stock) 2)))
