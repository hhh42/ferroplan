(define (problem husbandry-contract-1)
  (:domain rpg-world)
  (:objects
    pat - agent
    farmstead - location)
  (:init
    ;; agent placement
    (at pat farmstead)
    ;; clustered sites + station at the single location
    (is-field farmstead)
    (is-pasture farmstead)
    (is-water farmstead)
    (has-barn farmstead)
    ;; tools + role
    (has-shovel pat)
    (herder pat)
    ;; trivial self-loop so reachability is well-defined
    (link farmstead farmstead)
    (= (dist farmstead farmstead) 0)
    ;; ---- every fluent referenced, initialized ----
    ;; pre-delivered raw inputs from other subsystems
    (= (livestock) 2)
    (= (grain) 0)
    (= (water) 2)
    ;; animal-husbandry outputs
    (= (dairy) 0)
    (= (meat) 0)
    (= (hide) 0)
    ;; reused shared stockpiles touched by this subsystem
    (= (fiber) 0)
    (= (food) 0)
    ;; remaining global stockpiles (kept zero; present so the domain is grounded)
    (= (logs) 0) (= (ore) 0) (= (stone) 0) (= (clay) 0) (= (herbs) 0)
    (= (planks) 0) (= (charcoal) 0) (= (ingots) 0) (= (blocks) 0) (= (bricks) 0)
    (= (cloth) 0) (= (clothing) 0) (= (meals) 0) (= (potions) 0)
    (= (axe-stock) 0) (= (pick-stock) 0) (= (shovel-stock) 0) (= (hammer-stock) 0)
    (= (coin) 0) (= (mana) 0))
  (:goal (and
    (>= (dairy) 1)     ; milked the herd
    (>= (fiber) 1)     ; sheared the herd at the barn
    (>= (hide) 1)      ; butchered a head
    (>= (food) 1)))    ; cured the meat into rations
  (:metric minimize (total-time)))