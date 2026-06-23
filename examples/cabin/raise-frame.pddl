;; The first day's work: get a roof over your head. Fell trees, mill planks and
;; beams, split shingles, forge a few nails, quarry stone — then lay the
;; foundation, raise the walls, and frame the roof. (Goal: roof-on — the shell,
;; not yet floored/doored/glazed.) A shorter sequence than the full cabin.
(define (problem raise-frame)
  (:domain cabin)
  (:objects you - worker  site - location)
  (:init
    (at you site) (is-site site)
    (lumberjack you)
    (= (logs) 0) (= (planks) 0) (= (beams) 0) (= (shingles) 0)
    (= (ore) 0) (= (ingots) 0) (= (nails) 0)
    (= (sand) 0) (= (panes) 0) (= (stone) 0)
    (= (total-time) 0))
  (:goal (roof-on))
  (:metric minimize (total-time)))
