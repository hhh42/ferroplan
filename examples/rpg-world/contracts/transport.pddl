(define (problem transport-logistics-contract)
  (:domain rpg-world)
  (:objects
    pat - agent
    depot1 pasture1 - location)
  (:init
    (at pat depot1)
    (porter pat)
    ;; clustered stations / sites
    (has-sawmill depot1)
    (has-cartwright depot1)
    (has-depot depot1)
    (is-pasture pasture1)
    ;; static map (kept static; no action touches link)
    (link depot1 pasture1)
    (link pasture1 depot1)
    (= (dist depot1 pasture1) 1)
    (= (dist pasture1 depot1) 1)
    ;; pre-delivered raw inputs
    (= (planks) 8)
    (= (ingots) 2)
    (= (food) 4)
    ;; transport-logistics fluents
    (= (cart-parts) 0)
    (= (carts) 0)
    (= (pack-animals) 0)
    (= (carry-cap) 0)
    (= (cargo) 0)
    ;; other referenced global fluents initialized for safety
    (= (coin) 0))
  (:goal (and (>= (coin) 2) (>= (carry-cap) 1) (>= (carts) 0)))
  (:metric minimize (total-time)))