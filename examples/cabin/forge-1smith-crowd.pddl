;; Still ONE smith, but a crew of FIVE — the extra labourers can't smith, so it's no faster.
;; Smithing-heavy order: forge 80 nails. Smelting + forging are SMITH-ONLY; mining is
;; general labour. Run with FF_TDEMAND=1 FF_TCONC=1 and compare the makespan across the
;; forge-* problems: more SMITHS help; more labourers don't.
(define (problem forge-1smith-crowd) (:domain cabin-crew-skilled)
  (:objects ben cal dan eve fin - worker  s - location)
  (:init (is-site s) (at ben s) (at cal s) (at dan s) (at eve s) (at fin s)
    (smith ben)
    (= (logs) 0)(= (planks) 0)(= (beams) 0)(= (shingles) 0)(= (ore) 0)(= (ingots) 0)(= (nails) 0)(= (sand) 0)(= (panes) 0)(= (stone) 0))
  (:goal (>= (nails) 80)))
