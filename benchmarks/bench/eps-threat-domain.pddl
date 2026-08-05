;; 0.21 Phase 7 fixture: same-slot reader-START vs deleter-END (the
;; map-analyzer i17/i18/i20 residue, minimized). OCCUPY's at-end effect
;; deletes (clear ?j); BUILD's at-start precondition reads it. At one raw
;; epoch the search legally fires BUILD's start while (clear j1) still
;; holds and THEN the deleting end — the epsilon-separation pass must
;; emit the reader's start strictly before the deleter's end, or the
;; emitted plan executes BUILD against a fact already deleted. Two
;; junctions so a SECOND occupier's unrelated end can share the slot
;; (eps-threat-p02): ends are RIGID — pinned at start+duration with the
;; starts already ε-chained one slot earlier — so the reader must cross
;; the whole end run without permuting it, the exact i17 geometry whose
;; infeasible ordering the STN consistency check vetoed back to raw.
(define (domain epsthreat)
  (:requirements :typing :durative-actions)
  (:types junction)
  (:predicates (clear ?j - junction) (built ?j - junction) (occupied ?j - junction))
  (:durative-action occupy
    :parameters (?j - junction)
    :duration (= ?duration 1)
    :condition (and)
    :effect (and (at end (not (clear ?j))) (at end (occupied ?j))))
  (:durative-action build
    :parameters (?j - junction)
    :duration (= ?duration 2)
    :condition (at start (clear ?j))
    :effect (and (at end (built ?j)))))
