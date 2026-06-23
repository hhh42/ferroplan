;; The same three workers, but ALL cross-trained (each can mill AND smith). Now any
;; task can go to any free worker, so the skilled chains parallelise — shorter makespan.
(define (problem skilled-crosstrained) (:domain cabin-crew-skilled)
  (:objects ana ben cal - worker  s - location)
  (:init (is-site s) (at ana s) (at ben s) (at cal s)
    (sawyer ana) (sawyer ben) (sawyer cal) (smith ana) (smith ben) (smith cal)
    (= (logs) 0)(= (planks) 0)(= (beams) 0)(= (shingles) 0)(= (ore) 0)(= (ingots) 0)(= (nails) 0)(= (sand) 0)(= (panes) 0)(= (stone) 0))
  (:goal (and (roof-on) (door-hung) (windows-glazed))))
