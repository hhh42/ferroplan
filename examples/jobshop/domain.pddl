;; ============================================================================
;; ferroplan "jobshop" — a job-shop / scheduling domain.
;;
;; DESIGN INTENT
;;  * Each JOB is a fixed SEQUENCE of operations (s1 -> s2 -> ... -> sK).
;;    An op may only run once its predecessor in the same job is done.
;;  * Each OPERATION runs on a MACHINE. A machine does ONE op at a time:
;;    machine-exclusion is modeled with a (free ?m) token, consumed AT START
;;    and restored AT END of the durative op. This is the resource-exclusion
;;    pattern that rpg-world deliberately omits.
;;  * Operations have per-(job,stage) processing durations (numeric fluent).
;;  * Goal = all jobs complete. The planner minimises-ish makespan by
;;    overlapping ops of different jobs on distinct free machines.
;;
;; Style follows rpg-world: :typing, :adl, :durative-actions, :numeric-fluents,
;; durative actions with at-start/at-end conditions & effects.
;; ============================================================================
(define (domain jobshop)
  (:requirements :typing :adl :durative-actions :numeric-fluents)

  (:types
    job machine stage - object)   ; stage = an ordinal position in a job's route

  (:predicates
    ;; ---- machine exclusion ----
    (free ?m - machine)               ; machine idle; consumed at-start of an op

    ;; ---- job routing (static structure of a problem) ----
    (route ?j - job ?s - stage ?m - machine)  ; job j's stage s runs on machine m
    (next ?s1 ?s2 - stage)            ; s2 immediately follows s1 in ordinal order
    (final-stage ?j - job ?s - stage) ; the last stage of job j

    ;; ---- job progress (dynamic) ----
    (at-stage ?j - job ?s - stage)    ; stage s is the next op of job j to run
    (done-stage ?j - job ?s - stage)  ; job j's stage s op has finished
    (busy ?j - job)                   ; job currently has an op in progress
    (job-complete ?j - job))

  ;; per-(job,stage) processing time (data-driven duration)
  (:functions
    (proctime ?j - job ?s - stage) - number)

  ;; ========================= OPERATION =========================
  ;; Run job ?j's stage ?s on machine ?m.
  ;;  * requires the job queued at this stage and not already busy,
  ;;  * the stage routed to this machine,
  ;;  * the machine free (consumed at start, restored at end => exclusion).
  (:durative-action operate
    :parameters (?j - job ?s - stage ?m - machine)
    :duration (= ?duration (proctime ?j ?s))
    :condition (and
                 (at start (at-stage ?j ?s))
                 (at start (route ?j ?s ?m))
                 (at start (free ?m))
                 (at start (not (busy ?j))))
    :effect (and
                 (at start (not (free ?m)))
                 (at start (not (at-stage ?j ?s)))
                 (at start (busy ?j))
                 (at end (free ?m))
                 (at end (not (busy ?j)))
                 (at end (done-stage ?j ?s))))

  ;; ========================= ADVANCE =========================
  ;; Once a non-final stage is done, queue the job at the following stage.
  (:durative-action advance
    :parameters (?j - job ?s1 ?s2 - stage)
    :duration (= ?duration 0.01)
    :condition (and
                 (at start (done-stage ?j ?s1))
                 (at start (next ?s1 ?s2)))
    :effect (and
                 (at start (not (done-stage ?j ?s1)))
                 (at end (at-stage ?j ?s2))))

  ;; ========================= COMPLETE =========================
  ;; When a job's final stage is done, mark the job complete.
  (:durative-action complete-job
    :parameters (?j - job ?s - stage)
    :duration (= ?duration 0.01)
    :condition (and
                 (at start (done-stage ?j ?s))
                 (at start (final-stage ?j ?s)))
    :effect (and
                 (at start (not (done-stage ?j ?s)))
                 (at end (job-complete ?j)))))