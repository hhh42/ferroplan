;; ============================================================================
;; ferroplan "logistics" — a transshipment / routing planning domain.
;;
;; DESIGN INTENT
;;  * Models the truck -> depot -> train shape that rpg-world CANNOT express,
;;    because rpg-world's stockpile is GLOBAL (one (logs) counter for the world).
;;    Here goods are PER-LOCATION: (stock ?l ?p) — units of package type ?p
;;    sitting at location ?l. Delivery means moving counts between locations.
;;  * Vehicles (trucks and a train line) MOVE between connected locations and
;;    carry a load up to a per-vehicle CAPACITY. Load/unload at a location
;;    transfer one unit between that location's stock and a vehicle's onboard load.
;;  * Trains only stop at rail-depots and run on rail links; trucks run on roads.
;;    A package crossing the map must be trucked to a rail-depot, railed across,
;;    then trucked to destination — genuine transshipment.
;;  * Uses what the engine does well: typing, ADL, durative actions, numeric
;;    fluents (graph distances, per-location stock, per-vehicle load/capacity).
;;
;; GOOD FLOW
;;   (stock origin pkg) --load--> (load truck) --drive--> (unload at depot)
;;       --load train--> (run train) --unload--> (load truck) --> (stock dest pkg)
;; ============================================================================
(define (domain logistics)
  (:requirements :typing :adl :durative-actions :numeric-fluents)

  (:types
    location vehicle package - object
    truck train - vehicle)

  (:predicates
    ;; ---- network ----
    (road ?a ?b - location)        ; a drivable road segment (static, both ways given)
    (rail ?a ?b - location)        ; a rail segment between rail-depots (static)
    (rail-depot ?l - location)     ; location served by the train line
    ;; ---- vehicle position / kind ----
    (at-veh ?v - vehicle ?l - location)
    (is-truck ?v - vehicle)
    (is-train ?v - vehicle))

  (:functions
    (dist ?a ?b - location)        ; travel time on the segment a->b
    ;; per-location stock of a package type: how many of ?p sit at ?l
    (stock ?l - location ?p - package)
    ;; how many units of ?p are currently aboard vehicle ?v
    (load ?v - vehicle ?p - package)
    ;; total units aboard ?v (across all package types)
    (aboard ?v - vehicle)
    ;; max units ?v can carry
    (capacity ?v - vehicle))

  ;; ----------------------------------------------------------------------
  ;; MOVEMENT
  ;; ----------------------------------------------------------------------

  ;; Trucks drive any road segment.
  (:durative-action drive
    :parameters (?v - truck ?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at-veh ?v ?from))
                    (at start (road ?from ?to)))
    :effect (and (at start (not (at-veh ?v ?from)))
                 (at end (at-veh ?v ?to))))

  ;; Trains run rail segments between rail-depots only.
  (:durative-action run-train
    :parameters (?v - train ?from ?to - location)
    :duration (= ?duration (dist ?from ?to))
    :condition (and (at start (at-veh ?v ?from))
                    (at start (rail ?from ?to))
                    (at start (rail-depot ?from))
                    (at start (rail-depot ?to)))
    :effect (and (at start (not (at-veh ?v ?from)))
                 (at end (at-veh ?v ?to))))

  ;; ----------------------------------------------------------------------
  ;; TRANSSHIPMENT (load / unload) — works for trucks and trains alike
  ;; ----------------------------------------------------------------------

  ;; Pick a unit of ?p up off the ground at ?l onto vehicle ?v (capacity-gated).
  (:durative-action load-pkg
    :parameters (?v - vehicle ?l - location ?p - package)
    :duration (= ?duration 1)
    :condition (and (at start (at-veh ?v ?l))
                    (at start (>= (stock ?l ?p) 1))
                    (at start (< (aboard ?v) (capacity ?v))))
    :effect (and (at end (decrease (stock ?l ?p) 1))
                 (at end (increase (load ?v ?p) 1))
                 (at end (increase (aboard ?v) 1))))

  ;; Drop a unit of ?p off vehicle ?v at the current location ?l.
  (:durative-action unload-pkg
    :parameters (?v - vehicle ?l - location ?p - package)
    :duration (= ?duration 1)
    :condition (and (at start (at-veh ?v ?l))
                    (at start (>= (load ?v ?p) 1)))
    :effect (and (at end (increase (stock ?l ?p) 1))
                 (at end (decrease (load ?v ?p) 1))
                 (at end (decrease (aboard ?v) 1)))))