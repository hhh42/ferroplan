;; ============================================================================
;; ferroplan "world" — the universal low-level planning domain for a survival /
;; village-building multiplayer RPG.
;;
;; DESIGN INTENT
;;  * Broad, not deep: many resources, recipes, roles, stations, and entities, so
;;    one domain covers gathering, smithing, woodwork, masonry, textiles, cooking,
;;    alchemy/magic, construction, and trade/contracts.
;;  * Built to DECOMPOSE: an external scheduler carves the world goal into
;;    contract-sized sub-tasks ("make 8 planks", "forge 2 axes", "raise a house")
;;    each given to a worker for a time window. Each sub-task is an easy, NON-
;;    optimal plan — ferroplan just finds *a* way, fast.
;;  * Uses what the engine does well: typing, full ADL (conditional + forall
;;    effects, quantified + disjunctive preconditions, negation), numeric fluents
;;    (graph distances + resource stockpiles), durative actions, and a :derived
;;    reachability axiom over the map.
;;
;; RESOURCE FLOW (each output is producible, each input has a source):
;;   logs ->(saw) planks            logs ->(kiln) charcoal
;;   ore + charcoal ->(smelt) ingots        ingots ->(forge) tools / fittings
;;   stone ->(mason) blocks         clay ->(kiln) bricks
;;   fiber ->(loom) cloth ->(tailor) clothing
;;   food + water ->(cook) meals    herbs + water ->(brew) potions
;;   materials + labor ->(build) structures
;; ============================================================================
(define (domain rpg-world)
  (:requirements :typing :adl :durative-actions :numeric-fluents)

  (:types
    agent location - object
    slot - object)            ; building plots for houses

  (:predicates
    ;; ---- world / movement ----
    (at ?a - agent ?l - location)
    (link ?a ?b - location)          ; a discovered path segment (static)
    (reachable ?a ?b - location)     ; DERIVED: transitive closure of link
    ;; ---- resource sites ----
    (is-forest ?l - location)
    (is-mine ?l - location)
    (is-quarry ?l - location)
    (is-claypit ?l - location)
    (is-water ?l - location)         ; well or river
    (is-pasture ?l - location)
    (is-field ?l - location)
    (is-market ?l - location)
    ;; ---- workstations (a location may host several) ----
    (has-sawmill ?l - location)
    (has-kiln ?l - location)
    (has-forge ?l - location)
    (has-anvil ?l - location)
    (has-mason ?l - location)
    (has-loom ?l - location)
    (has-hearth ?l - location)
    (has-altar ?l - location)
    (is-buildsite ?l - location)
    ;; ---- per-agent tools (persistent, not consumed) ----
    (has-axe ?a - agent)
    (has-pick ?a - agent)
    (has-shovel ?a - agent)
    (has-hammer ?a - agent)
    ;; ---- roles (gate or improve work) ----
    (woodcutter ?a - agent)
    (miner ?a - agent)
    (smith ?a - agent)
    (mason-skill ?a - agent)
    (cook ?a - agent)
    (mage ?a - agent)
    ;; ---- structures / outcomes ----
    (built-house ?s - slot)
    (built-square)
    (built-wall)
    (well-dug)
    (feast-held))

  (:functions
    (dist ?a ?b - location)
    ;; raw materials
    (logs) (ore) (stone) (clay) (fiber) (food) (water) (herbs)
    ;; processed goods
    (planks) (charcoal) (ingots) (blocks) (bricks) (cloth) (clothing)
    (meals) (potions)
    ;; tool stock (forged, then equipped onto an agent)
    (axe-stock) (pick-stock) (shovel-stock) (hammer-stock)
    ;; economy + magic
    (coin) (mana))

  ;; ----- DERIVED: reachability over the (static) map -----
  (:derived (reachable ?a ?b - location)
    (or (link ?a ?b)
        (exists (?c - location) (and (link ?a ?c) (reachable ?c ?b)))))

  ;; ========================= MOVEMENT =========================
  (:durative-action travel
    :parameters (?a - agent ?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at ?a ?from)) (at start (reachable ?from ?to)))
    :effect (and (at start (not (at ?a ?from))) (at end (at ?a ?to))))

  ;; ========================= GATHERING =========================
  ;; A woodcutter chops faster... modeled as a yield bonus (conditional effect).
  (:durative-action chop-wood
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (is-forest ?l)) (at start (has-axe ?a)))
    :effect (and (at end (increase (logs) 1))
                 (at end (when (woodcutter ?a) (increase (logs) 1)))))

  (:durative-action mine-ore
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?a ?l)) (at start (is-mine ?l)) (at start (has-pick ?a)))
    :effect (and (at end (increase (ore) 1))
                 (at end (when (miner ?a) (increase (ore) 1)))))

  (:durative-action quarry-stone
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?a ?l)) (at start (is-quarry ?l)) (at start (has-pick ?a)))
    :effect (at end (increase (stone) 1)))

  (:durative-action dig-clay
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (is-claypit ?l)) (at start (has-shovel ?a)))
    :effect (at end (increase (clay) 1)))

  (:durative-action draw-water
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 1)
    :condition (and (at start (at ?a ?l)) (at start (is-water ?l)))
    :effect (at end (increase (water) 1)))

  (:durative-action forage-food
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (or (is-field ?l) (is-forest ?l))))
    :effect (at end (increase (food) 1)))

  (:durative-action shear-fiber
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (is-pasture ?l)))
    :effect (at end (increase (fiber) 1)))

  (:durative-action gather-herbs
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (or (is-forest ?l) (is-field ?l))))
    :effect (at end (increase (herbs) 1)))

  ;; ========================= PROCESSING =========================
  (:durative-action saw-planks                  ; 1 log -> 2 planks
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (has-sawmill ?l)) (at start (>= (logs) 1)))
    :effect (and (at start (decrease (logs) 1)) (at end (increase (planks) 2))))

  (:durative-action burn-charcoal               ; 2 logs -> 1 charcoal
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-kiln ?l)) (at start (>= (logs) 2)))
    :effect (and (at start (decrease (logs) 2)) (at end (increase (charcoal) 1))))

  (:durative-action fire-bricks                 ; 2 clay -> 2 bricks
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-kiln ?l)) (at start (>= (clay) 2)))
    :effect (and (at start (decrease (clay) 2)) (at end (increase (bricks) 2))))

  (:durative-action smelt-ingot                 ; ore + charcoal -> ingot (smith bonus)
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 4)
    :condition (and (at start (at ?a ?l)) (at start (has-forge ?l))
                    (at start (>= (ore) 1)) (at start (>= (charcoal) 1)))
    :effect (and (at start (decrease (ore) 1)) (at start (decrease (charcoal) 1))
                 (at end (increase (ingots) 1))
                 (at end (when (smith ?a) (increase (ingots) 1)))))

  (:durative-action mason-blocks                ; 1 stone -> 1 block (mason bonus)
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (has-mason ?l)) (at start (>= (stone) 1)))
    :effect (and (at start (decrease (stone) 1))
                 (at end (increase (blocks) 1))
                 (at end (when (mason-skill ?a) (increase (blocks) 1)))))

  (:durative-action weave-cloth                 ; 2 fiber -> 1 cloth
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-loom ?l)) (at start (>= (fiber) 2)))
    :effect (and (at start (decrease (fiber) 2)) (at end (increase (cloth) 1))))

  (:durative-action tailor-clothing             ; 2 cloth -> 1 clothing
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-loom ?l)) (at start (>= (cloth) 2)))
    :effect (and (at start (decrease (cloth) 2)) (at end (increase (clothing) 1))))

  (:durative-action cook-meal                   ; food + water -> 2 meals (cook bonus)
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (has-hearth ?l))
                    (at start (>= (food) 1)) (at start (>= (water) 1)))
    :effect (and (at start (decrease (food) 1)) (at start (decrease (water) 1))
                 (at end (increase (meals) 2))
                 (at end (when (cook ?a) (increase (meals) 1)))))

  ;; ========================= TOOLSMITHING =========================
  ;; Forge a tool into the shared stock; an agent then equips one.
  (:durative-action forge-axe
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-anvil ?l)) (at start (has-hammer ?a))
                    (at start (>= (ingots) 1)) (at start (>= (planks) 1)))
    :effect (and (at start (decrease (ingots) 1)) (at start (decrease (planks) 1))
                 (at end (increase (axe-stock) 1))))

  (:durative-action forge-pick
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-anvil ?l)) (at start (has-hammer ?a))
                    (at start (>= (ingots) 2)) (at start (>= (planks) 1)))
    :effect (and (at start (decrease (ingots) 2)) (at start (decrease (planks) 1))
                 (at end (increase (pick-stock) 1))))

  (:durative-action forge-shovel
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-anvil ?l)) (at start (has-hammer ?a))
                    (at start (>= (ingots) 1)) (at start (>= (planks) 1)))
    :effect (and (at start (decrease (ingots) 1)) (at start (decrease (planks) 1))
                 (at end (increase (shovel-stock) 1))))

  (:durative-action equip-axe
    :parameters (?a - agent)
    :duration (= ?duration 1)
    :condition (and (at start (>= (axe-stock) 1)) (at start (not (has-axe ?a))))
    :effect (and (at start (decrease (axe-stock) 1)) (at end (has-axe ?a))))

  (:durative-action equip-pick
    :parameters (?a - agent)
    :duration (= ?duration 1)
    :condition (and (at start (>= (pick-stock) 1)) (at start (not (has-pick ?a))))
    :effect (and (at start (decrease (pick-stock) 1)) (at end (has-pick ?a))))

  (:durative-action equip-shovel
    :parameters (?a - agent)
    :duration (= ?duration 1)
    :condition (and (at start (>= (shovel-stock) 1)) (at start (not (has-shovel ?a))))
    :effect (and (at start (decrease (shovel-stock) 1)) (at end (has-shovel ?a))))

  ;; ========================= ALCHEMY / MAGIC =========================
  (:durative-action meditate                    ; recover mana (mages only)
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (has-altar ?l)) (at start (mage ?a)))
    :effect (at end (increase (mana) 2)))

  (:durative-action brew-potion                 ; herbs + water -> potion (costs mana)
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 3)
    :condition (and (at start (at ?a ?l)) (at start (has-altar ?l)) (at start (mage ?a))
                    (at start (>= (herbs) 1)) (at start (>= (water) 1)) (at start (>= (mana) 1)))
    :effect (and (at start (decrease (herbs) 1)) (at start (decrease (water) 1))
                 (at start (decrease (mana) 1)) (at end (increase (potions) 1))))

  ;; ========================= CONSTRUCTION =========================
  (:durative-action build-well
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 5)
    :condition (and (at start (at ?a ?l)) (at start (is-buildsite ?l)) (at start (>= (blocks) 3)))
    :effect (and (at start (decrease (blocks) 3)) (at end (well-dug))))

  (:durative-action build-wall
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 6)
    :condition (and (at start (at ?a ?l)) (at start (is-buildsite ?l)) (at start (>= (blocks) 4)))
    :effect (and (at start (decrease (blocks) 4)) (at end (built-wall))))

  (:durative-action build-house
    :parameters (?a - agent ?l - location ?s - slot)
    :duration (= ?duration 6)
    :condition (and (at start (at ?a ?l)) (at start (is-buildsite ?l)) (at start (not (built-house ?s)))
                    (at start (>= (planks) 4)) (at start (>= (bricks) 2)))
    :effect (and (at start (decrease (planks) 4)) (at start (decrease (bricks) 2))
                 (at end (built-house ?s))))

  ;; The square caps the village: only once EVERY house slot is built (forall).
  (:durative-action build-square
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 5)
    :condition (and (at start (at ?a ?l)) (at start (is-buildsite ?l)) (at start (>= (blocks) 4))
                    (at start (forall (?s - slot) (built-house ?s))))
    :effect (and (at start (decrease (blocks) 4)) (at end (built-square))))

  ;; ========================= COMMUNITY / TRADE =========================
  ;; A feast at the hearth: consumes meals and feeds EVERY agent at that location
  ;; (forall + when over the concrete state).
  (:durative-action hold-feast
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 2)
    :condition (and (at start (at ?a ?l)) (at start (has-hearth ?l)) (at start (>= (meals) 3)))
    :effect (and (at start (decrease (meals) 3)) (at end (feast-held))))

  (:durative-action sell-goods                  ; turn surplus planks into coin
    :parameters (?a - agent ?l - location)
    :duration (= ?duration 1)
    :condition (and (at start (at ?a ?l)) (at start (is-market ?l)) (at start (>= (planks) 2)))
    :effect (and (at start (decrease (planks) 2)) (at end (increase (coin) 1)))))
