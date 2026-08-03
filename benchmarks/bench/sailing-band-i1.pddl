;; d=-30, boat b0 at (0,-60): both LOWER band sides are 30 short
;; (x+y = -60 < -30, y-x = -60 < -30), best raisers move at +3 per step
;; (NE for x+y, NW for y-x), so the charged h(init) is
;; 1 + ceil(30/3) + ceil(30/3) = 21 — and the shortest monotone plan is
;; 10 NE + 10 NW + save = 21 steps. The second boat only widens the
;; search space (the 4-fluent state is why a flat h=1 cannot finish
;; under a small eval cap — the RED the charge turns GREEN).
(define (problem sailing-band-1)
    (:domain sailing-band)
    (:objects b0 b1 - boat p0 - person)
    (:init
        (crewed b0)
        (= (x b0) 0)
        (= (y b0) -60)
        (= (x b1) 40)
        (= (y b1) 20)
        (= (d p0) -30))
    (:goal (saved p0))
)
