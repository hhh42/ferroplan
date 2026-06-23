;; ============================================================================
;; cabin — raise a log cabin from standing trees. A deliberately DEEP crafting
;; chain: the goal (cabin-finished) pulls one long, ordered sequence of work —
;; fell trees, mill them into planks/beams/shingles, forge nails from ore, fire
;; window glass from sand, quarry stone, then build IN ORDER: foundation → walls →
;; roof → floor → door → hang → window frames → glaze → finish.
;;
;; Modeled as a NUMERIC classical domain (instantaneous actions; each action adds
;; its work to (total-time), which the :metric minimizes) rather than durative —
;; ferroplan's metric optimizer (mode pddl3) handles the long ~50-step plan that
;; the temporal decision-epoch search can't. Single site; global resource counters;
;; resources are consumed and products appear in the same step.
;; ============================================================================
(define (domain cabin)
  (:requirements :typing :adl :numeric-fluents :action-costs)
  (:types worker location - object)
  (:predicates
    (at ?w - worker ?l - location)
    (is-site ?l - location)
    (lumberjack ?w - worker)       ; fells double
    (foundation-laid) (walls-up) (roof-on) (floor-laid)
    (door-built) (door-hung) (window-frames-set) (windows-glazed)
    (cabin-finished))
  (:functions
    (logs) (planks) (beams) (shingles)
    (ore) (ingots) (nails)
    (sand) (panes)
    (stone)
    (total-time))

  ;; ---------------- harvest (from the land) ----------------
  (:action fell-tree
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l))
    :effect (and (increase (logs) 1) (when (lumberjack ?w) (increase (logs) 1))
                 (increase (total-time) 3)))
  (:action mine-ore
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l))
    :effect (and (increase (ore) 1) (increase (total-time) 3)))
  (:action quarry-stone
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l))
    :effect (and (increase (stone) 1) (increase (total-time) 3)))
  (:action dig-sand
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l))
    :effect (and (increase (sand) 1) (increase (total-time) 2)))

  ;; ---------------- mill / smith / fire (process raws) ----------------
  (:action saw-planks                     ; 1 log -> 3 planks
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (logs) 1))
    :effect (and (decrease (logs) 1) (increase (planks) 3) (increase (total-time) 2)))
  (:action hew-beam                       ; 1 log -> 2 beams
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (logs) 1))
    :effect (and (decrease (logs) 1) (increase (beams) 2) (increase (total-time) 3)))
  (:action split-shingles                 ; 1 log -> 4 shingles
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (logs) 1))
    :effect (and (decrease (logs) 1) (increase (shingles) 4) (increase (total-time) 2)))
  (:action smelt-ingot                    ; 1 ore -> 1 ingot
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (ore) 1))
    :effect (and (decrease (ore) 1) (increase (ingots) 1) (increase (total-time) 4)))
  (:action forge-nails                    ; 1 ingot -> 10 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (ingots) 1))
    :effect (and (decrease (ingots) 1) (increase (nails) 10) (increase (total-time) 3)))
  (:action fire-glass                     ; 2 sand -> 1 pane
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (sand) 2))
    :effect (and (decrease (sand) 2) (increase (panes) 1) (increase (total-time) 4)))

  ;; ---------------- build: a STRICT LINEAR chain (each stage needs the previous) ----
  (:action lay-foundation                 ; 3 stone + 2 beams
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (>= (stone) 3) (>= (beams) 2))
    :effect (and (decrease (stone) 3) (decrease (beams) 2) (foundation-laid) (increase (total-time) 5)))
  (:action raise-walls                    ; needs foundation; 6 planks + 3 beams + 8 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (foundation-laid) (>= (planks) 6) (>= (beams) 3) (>= (nails) 8))
    :effect (and (decrease (planks) 6) (decrease (beams) 3) (decrease (nails) 8) (walls-up) (increase (total-time) 6)))
  (:action frame-roof                     ; needs walls; 3 beams + 12 shingles + 6 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (walls-up) (>= (beams) 3) (>= (shingles) 12) (>= (nails) 6))
    :effect (and (decrease (beams) 3) (decrease (shingles) 12) (decrease (nails) 6) (roof-on) (increase (total-time) 6)))
  (:action lay-floor                      ; needs roof; 5 planks + 5 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (roof-on) (>= (planks) 5) (>= (nails) 5))
    :effect (and (decrease (planks) 5) (decrease (nails) 5) (floor-laid) (increase (total-time) 4)))
  (:action build-door                     ; needs floor; 3 planks + 2 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (floor-laid) (>= (planks) 3) (>= (nails) 2))
    :effect (and (decrease (planks) 3) (decrease (nails) 2) (door-built) (increase (total-time) 3)))
  (:action hang-door                      ; needs the built door; 2 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (door-built) (>= (nails) 2))
    :effect (and (decrease (nails) 2) (door-hung) (increase (total-time) 2)))
  (:action set-window-frames              ; needs door hung; 3 planks
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (door-hung) (>= (planks) 3))
    :effect (and (decrease (planks) 3) (window-frames-set) (increase (total-time) 3)))
  (:action glaze-windows                  ; needs frames; 3 panes + 3 nails
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (window-frames-set) (>= (panes) 3) (>= (nails) 3))
    :effect (and (decrease (panes) 3) (decrease (nails) 3) (windows-glazed) (increase (total-time) 3)))
  (:action finish-cabin                   ; the keystone, once the windows are in
    :parameters (?w - worker ?l - location)
    :precondition (and (at ?w ?l) (is-site ?l) (windows-glazed))
    :effect (and (cabin-finished) (increase (total-time) 2))))
