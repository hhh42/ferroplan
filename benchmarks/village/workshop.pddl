;; Village ladder rung 2 — the toolchain workshop.
;; The needed tool does not exist yet: a chisel must first be MADE (from
;; iron bought at the market plus a branch gathered from the forest),
;; then carved decoys sold. Tools made of parts made of materials — the
;; deep-make story — plus buy-side market use.
;;
;; A made tool enters the world as STOCK; the convert action of this
;; problem's catalog is the (station-local) claim-tool recipe trick:
;; making "chisel-kit" yields a stock unit, and the kit's presence at the
;; smithy is modeled by the for-sale/tool-at bridge below being part of
;; the catalog (the smith sells finished chisels; carving needs holding
;; one). The abstract rules never change.
(define (problem workshop) (:domain village)
  (:objects
    mara - worker
    iron branch chisel decoy - item
    forge-chisel carve-decoy - recipe
    forest smithy carvery town-market - station
    tongs chisel-tool - tool)
  (:init
    (at mara town-market)
    (adjacent town-market smithy) (adjacent smithy town-market)
    (adjacent smithy forest) (adjacent forest smithy)
    (adjacent smithy carvery) (adjacent carvery smithy)
    (adjacent carvery town-market) (adjacent town-market carvery)
    (= (walk-time town-market smithy) 2) (= (walk-time smithy town-market) 2)
    (= (walk-time smithy forest) 3) (= (walk-time forest smithy) 3)
    (= (walk-time smithy carvery) 2) (= (walk-time carvery smithy) 2)
    (= (walk-time carvery town-market) 3) (= (walk-time town-market carvery) 3)
    (raw branch) (source branch forest) (= (gather-time branch) 2)
    (tool-at tongs smithy)
    ;; forge-chisel: 1 iron + 1 branch -> 1 chisel (an ITEM), tongs, smithy
    (in1 forge-chisel iron) (in2 forge-chisel branch) (out forge-chisel chisel)
    (= (qty1 forge-chisel) 1) (= (qty2 forge-chisel) 1) (= (yield forge-chisel) 1)
    (needs-tool forge-chisel tongs) (station-of forge-chisel smithy)
    (= (make-time forge-chisel) 6)
    ;; carve-decoy: 1 branch + 1 chisel-as-material kept via qty2=0 trick?
    ;; No tricks: carving CONSUMES nothing of the chisel (qty2 0) but
    ;; requires chisel STOCK >= 1 as a workbench fixture.
    (in1 carve-decoy branch) (in2 carve-decoy chisel) (out carve-decoy decoy)
    (= (qty1 carve-decoy) 1) (= (qty2 carve-decoy) 0) (= (yield carve-decoy) 1)
    (needs-tool carve-decoy chisel-tool) (station-of carve-decoy carvery)
    (= (make-time carve-decoy) 4)
    (tool-at chisel-tool carvery)
    (market town-market)
    (for-sale iron town-market) (= (price iron) 3)
    (= (price branch) 1) (= (price chisel) 8) (= (price decoy) 9)
    (= (stock iron) 0) (= (stock branch) 0) (= (stock chisel) 0)
    (= (stock decoy) 0)
    (= (money) 6))
  (:goal (and (>= (money) 20) (>= (stock chisel) 1))))
