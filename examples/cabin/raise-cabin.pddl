;; Raise a whole log cabin from nothing but standing trees and raw land. The goal
;; (cabin-finished) pulls one long sequence — fell ~a dozen trees, saw planks, hew
;; beams, split shingles, mine+smelt+forge nails, dig sand + fire glass, quarry
;; stone — and only THEN build, in order: foundation → walls → roof → floor → door
;; → hang it → window frames → glaze → finish. A ~52-step plan: the whole job.
(define (problem raise-cabin)
  (:domain cabin)
  (:objects you - worker  site - location)
  (:init
    (at you site) (is-site site)
    (lumberjack you)             ; you fell double
    (= (logs) 0) (= (planks) 0) (= (beams) 0) (= (shingles) 0)
    (= (ore) 0) (= (ingots) 0) (= (nails) 0)
    (= (sand) 0) (= (panes) 0) (= (stone) 0)
    (= (total-time) 0))
  (:goal (cabin-finished))
  (:metric minimize (total-time)))
