(define (problem enchant-magic-contract)
  (:domain rpg-world)
  (:objects
    mira bran - agent
    sanctum - location)
  (:init
    ;; agents on site
    (at mira sanctum) (at bran sanctum)
    ;; mira is the spellcaster
    (mage mira) (enchanter mira)
    ;; bran needs healing
    (injured bran)
    ;; stations clustered at the sanctum
    (has-enchant-altar sanctum) (has-scribe-desk sanctum) (has-crystal-bed sanctum)
    (has-altar sanctum)
    ;; map: trivial self-link so reachability is well-formed
    (link sanctum sanctum)
    (= (dist sanctum sanctum) 0)
    ;; pre-delivered inputs from other subsystems
    (= (mana) 12)
    (= (cloth) 1)
    (= (potions) 2)
    (= (herbs) 2)
    (= (axe-stock) 1)
    ;; every other referenced fluent initialized
    (= (mana-crystals) 0)
    (= (scrolls) 0)
    (= (enchanted-gear) 0)
    (= (draughts) 0)
    ;; unrelated stockpiles kept at zero for completeness
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (fiber) 0)
    (= (food) 0) (= (water) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (clothing) 0) (= (meals) 0)
    (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0))
  (:goal (and (>= (mana-crystals) 1)
              (>= (scrolls) 1)
              (>= (enchanted-gear) 1)
              (not (injured bran)))))