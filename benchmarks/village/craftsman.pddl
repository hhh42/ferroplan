;; Village ladder rung 1 — the lone craftsman.
;; One worker, one trade: chop oak, saw planks (needs the saw, at the
;; sawmill), join a stool (needs the mallet, at the workshop), sell it.
;; Goal: earn 12 coins. Forces the full loop: tools, travel, gather,
;; a two-recipe chain, market.
(define (problem craftsman) (:domain village)
  (:objects
    bob - worker
    oak plank stool - item
    saw-planks join-stool - recipe
    forest sawmill workshop town-market - station
    saw mallet - tool)
  (:init
    (at bob forest)
    (adjacent forest sawmill) (adjacent sawmill forest)
    (adjacent sawmill workshop) (adjacent workshop sawmill)
    (adjacent workshop town-market) (adjacent town-market workshop)
    (= (walk-time forest sawmill) 2) (= (walk-time sawmill forest) 2)
    (= (walk-time sawmill workshop) 2) (= (walk-time workshop sawmill) 2)
    (= (walk-time workshop town-market) 2) (= (walk-time town-market workshop) 2)
    (raw oak) (source oak forest) (= (gather-time oak) 3)
    (tool-at saw sawmill) (tool-at mallet workshop)
    ;; saw-planks: 2 oak -> 3 plank, saw, sawmill, 4 ticks
    (in1 saw-planks oak) (in2 saw-planks oak) (out saw-planks plank)
    (= (req1 saw-planks) 1) (= (req2 saw-planks) 1) (= (qty1 saw-planks) 1) (= (qty2 saw-planks) 1) (= (yield saw-planks) 3)
    (needs-tool saw-planks saw) (station-of saw-planks sawmill)
    (= (make-time saw-planks) 4)
    ;; join-stool: 3 plank (+0 extra) -> 1 stool, mallet, workshop, 5 ticks
    (in1 join-stool plank) (in2 join-stool plank) (out join-stool stool)
    (= (req1 join-stool) 2) (= (req2 join-stool) 1) (= (qty1 join-stool) 2) (= (qty2 join-stool) 1) (= (yield join-stool) 1)
    (needs-tool join-stool mallet) (station-of join-stool workshop)
    (= (make-time join-stool) 5)
    (market town-market) (= (price stool) 12) (= (price oak) 1)
    (= (price plank) 2)
    (= (stock oak) 0) (= (stock plank) 0) (= (stock stool) 0)
    (= (money) 0))
  (:goal (>= (money) 12)))
