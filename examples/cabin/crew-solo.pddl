;; 1-worker crew. Solve: ff -o crew.pddl -f crew-solo.pddl --mode temporal (FF_TDEMAND=1 FF_TCONC=1)
;; Compare the makespan across crew-solo / crew-pair / crew-trio to see parallelism pay off.
(define (problem crew-solo) (:domain cabin-crew)
  (:objects w1 - worker  s - location)
  (:init (is-site s) (at w1 s)
    (= (logs) 0)(= (planks) 0)(= (beams) 0)(= (shingles) 0)(= (ore) 0)(= (ingots) 0)(= (nails) 0)(= (sand) 0)(= (panes) 0)(= (stone) 0))
  (:goal (and (roof-on) (door-hung) (windows-glazed))))
