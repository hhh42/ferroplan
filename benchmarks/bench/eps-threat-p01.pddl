(define (problem eps-threat-1) (:domain epsthreat)
  (:objects j1 j2 - junction)
  (:init (clear j1) (clear j2))
  (:goal (and (built j1) (occupied j1))))
