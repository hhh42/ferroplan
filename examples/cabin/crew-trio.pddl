;; 3-worker crew. Solve: ff -o crew.pddl -f crew-trio.pddl --mode temporal (FF_TDEMAND=1 FF_TCONC=1)
;; Compare the makespan across crew-solo / crew-pair / crew-trio to see parallelism pay off.
(define (problem crew-trio) (:domain cabin-crew)
  (:objects w1 w2 w3 - worker  s - location)
  (:init (is-site s) (at w1 s) (at w2 s) (at w3 s)
    (= (logs) 0)(= (planks) 0)(= (beams) 0)(= (shingles) 0)(= (ore) 0)(= (ingots) 0)(= (nails) 0)(= (sand) 0)(= (panes) 0)(= (stone) 0))
  (:goal (and (roof-on) (door-hung) (windows-glazed))))
