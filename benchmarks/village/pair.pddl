;; Village ladder rung 3 fixture — two workers, one communal economy.
;; The union of the craftsman and workshop catalogs on one map: bob and
;; mara share the world's stock and till; the HIRE example
;; (examples/village.rs) gives each mind its own goal contract over it.
(define (problem pair) (:domain village)
  (:objects
    bob mara - worker
    oak plank stool branch iron chisel decoy - item
    saw-planks join-stool forge-chisel carve-decoy - recipe
    forest sawmill workshop smithy carvery town-market - station
    saw mallet tongs chisel-tool - tool)
  (:init
    (at bob forest) (at mara town-market)
    (adjacent forest sawmill) (adjacent sawmill forest)
    (adjacent sawmill workshop) (adjacent workshop sawmill)
    (adjacent workshop town-market) (adjacent town-market workshop)
    (adjacent town-market smithy) (adjacent smithy town-market)
    (adjacent smithy carvery) (adjacent carvery smithy)
    (adjacent forest smithy) (adjacent smithy forest)
    (= (walk-time forest sawmill) 2) (= (walk-time sawmill forest) 2)
    (= (walk-time sawmill workshop) 2) (= (walk-time workshop sawmill) 2)
    (= (walk-time workshop town-market) 2) (= (walk-time town-market workshop) 2)
    (= (walk-time town-market smithy) 2) (= (walk-time smithy town-market) 2)
    (= (walk-time smithy carvery) 2) (= (walk-time carvery smithy) 2)
    (= (walk-time forest smithy) 3) (= (walk-time smithy forest) 3)
    (raw oak) (source oak forest) (= (gather-time oak) 3)
    (raw branch) (source branch forest) (= (gather-time branch) 2)
    (tool-at saw sawmill) (tool-at mallet workshop)
    (tool-at tongs smithy) (tool-at chisel-tool carvery)
    (in1 saw-planks oak) (in2 saw-planks oak) (out saw-planks plank)
    (= (req1 saw-planks) 1) (= (req2 saw-planks) 1) (= (qty1 saw-planks) 1) (= (qty2 saw-planks) 1) (= (yield saw-planks) 3)
    (needs-tool saw-planks saw) (station-of saw-planks sawmill)
    (= (make-time saw-planks) 4)
    (in1 join-stool plank) (in2 join-stool plank) (out join-stool stool)
    (= (req1 join-stool) 2) (= (req2 join-stool) 1) (= (qty1 join-stool) 2) (= (qty2 join-stool) 1) (= (yield join-stool) 1)
    (needs-tool join-stool mallet) (station-of join-stool workshop)
    (= (make-time join-stool) 5)
    (in1 forge-chisel iron) (in2 forge-chisel branch) (out forge-chisel chisel)
    (= (req1 forge-chisel) 1) (= (req2 forge-chisel) 1) (= (qty1 forge-chisel) 1) (= (qty2 forge-chisel) 1) (= (yield forge-chisel) 1)
    (needs-tool forge-chisel tongs) (station-of forge-chisel smithy)
    (= (make-time forge-chisel) 6)
    (in1 carve-decoy branch) (in2 carve-decoy chisel) (out carve-decoy decoy)
    (= (req1 carve-decoy) 1) (= (req2 carve-decoy) 1) (= (qty1 carve-decoy) 1) (= (qty2 carve-decoy) 0) (= (yield carve-decoy) 1)
    (needs-tool carve-decoy chisel-tool) (station-of carve-decoy carvery)
    (= (make-time carve-decoy) 4)
    (market town-market)
    (for-sale iron town-market) (= (price iron) 3)
    (= (price oak) 1) (= (price plank) 2) (= (price stool) 12)
    (= (price branch) 1) (= (price chisel) 8) (= (price decoy) 9)
    (= (stock oak) 0) (= (stock plank) 0) (= (stock stool) 0)
    (= (stock branch) 0) (= (stock iron) 0) (= (stock chisel) 0)
    (= (stock decoy) 0)
    (= (money) 6))
  (:goal (>= (stock stool) 1)))
