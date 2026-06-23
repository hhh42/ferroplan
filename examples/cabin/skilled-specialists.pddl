;; A specialist crew: ana mills (sawyer), ben smiths, cal is general labour. Milling
;; can ONLY go to ana and smelting/forging ONLY to ben — so those chains can't spread
;; across the crew. Compare the makespan to skilled-crosstrained.
(define (problem skilled-specialists) (:domain cabin-crew-skilled)
  (:objects ana ben cal - worker  s - location)
  (:init (is-site s) (at ana s) (at ben s) (at cal s)
    (sawyer ana) (smith ben)
    (= (logs) 0)(= (planks) 0)(= (beams) 0)(= (shingles) 0)(= (ore) 0)(= (ingots) 0)(= (nails) 0)(= (sand) 0)(= (panes) 0)(= (stone) 0))
  (:goal (and (roof-on) (door-hung) (windows-glazed))))
