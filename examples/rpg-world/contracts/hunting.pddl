(define (problem hunt-fish-contract)
  (:domain rpg-world)
  (:objects
    h1 - agent
    camp forest pond - location)
  (:init
    ;; --- agent start ---
    (at h1 camp)
    (hunter h1)
    (angler h1)
    ;; --- map (static links, both directions for easy reachability) ---
    (link camp forest) (link forest camp)
    (link camp pond) (link pond camp)
    (link forest pond) (link pond forest)
    (= (dist camp forest) 2)
    (= (dist forest camp) 2)
    (= (dist camp pond) 2)
    (= (dist pond camp) 2)
    (= (dist forest pond) 2)
    (= (dist pond forest) 2)
    ;; --- sites ---
    (is-forest forest)
    (is-water pond)
    ;; --- pre-delivered gear: one spear in stock so h1 can equip and use it ---
    (= (spear-stock) 1)
    (= (bow-stock) 0)
    ;; --- subsystem fluents initialized ---
    (= (meat) 0) (= (hide) 0) (= (bone) 0) (= (fish) 0)
    ;; --- referenced shared resources (init to 0; not needed for this plan) ---
    (= (planks) 0) (= (fiber) 0) (= (ingots) 0))
  (:goal (and (>= (meat) 2) (>= (hide) 2) (>= (bone) 1) (>= (fish) 2)))
  ;; A satisficing plan: equip-spear; travel camp->forest; set-trap; hunt-game
  ;; (hunter bonus -> meat 2, hide 1, bone 1); check-trap (meat 3, hide 3);
  ;; travel forest->pond; fish-water x? (angler -> fish 2 in one cast, +1 more).
  )