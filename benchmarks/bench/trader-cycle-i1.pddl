;; Cash 20 -> 500 at +2 net per capacity-1 lap: buy g1 at m1 (10), four
;; paid hops to m5, sell at 20, four hops home — 10 steps, +2. g2 loses
;; money everywhere (decoy). The relaxed charge prices the goal at
;; ceil(480/20) = 24 sells and stays finite against the ~2,400-step real
;; plan — two orders blind. The lap IS walkable (the travel fact chain
;; gives h a per-hop slope: 2,376 steps in 7,305 evals on the 0.20
;; binary), which is exactly the control's point: the charge adds only
;; constant one-level terms here, never a cycle gradient, so a cap under
;; that crawl stays out of reach before AND after.
(define (problem trader-cycle-1)
    (:domain trader-cycle)
    (:objects m1 m2 m3 m4 m5 - market g1 g2 - goods)
    (:init
        (at m1)
        (link m1 m2) (link m2 m1)
        (link m2 m3) (link m3 m2)
        (link m3 m4) (link m4 m3)
        (link m4 m5) (link m5 m4)
        (= (cash) 20)
        (= (capacity) 1)
        (= (bought g1) 0) (= (bought g2) 0)
        (= (fee m1 m2) 1) (= (fee m2 m1) 1)
        (= (fee m2 m3) 1) (= (fee m3 m2) 1)
        (= (fee m3 m4) 1) (= (fee m4 m3) 1)
        (= (fee m4 m5) 1) (= (fee m5 m4) 1)
        (= (price g1 m1) 10) (= (sellprice g1 m1) 5)
        (= (price g1 m2) 100) (= (sellprice g1 m2) 5)
        (= (price g1 m3) 100) (= (sellprice g1 m3) 5)
        (= (price g1 m4) 100) (= (sellprice g1 m4) 5)
        (= (price g1 m5) 100) (= (sellprice g1 m5) 20)
        (= (price g2 m1) 10) (= (sellprice g2 m1) 8)
        (= (price g2 m2) 10) (= (sellprice g2 m2) 9)
        (= (price g2 m3) 10) (= (sellprice g2 m3) 9)
        (= (price g2 m4) 10) (= (sellprice g2 m4) 9)
        (= (price g2 m5) 10) (= (sellprice g2 m5) 8))
    (:goal (>= (cash) 500))
)
