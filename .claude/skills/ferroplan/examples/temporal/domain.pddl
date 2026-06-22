(define (domain forge-temporal)
  (:requirements :typing :durative-actions)
  (:types part clamp)

  (:predicates
    (free ?c - clamp)
    (held ?p - part)          ; part is currently clamped
    (heated ?p - part)        ; part has been heated
    (forged ?p - part))       ; goal: part is forged

  ;; Clamping must remain active for the whole forge (over-all invariant).
  (:durative-action clamp-part
    :parameters (?p - part ?c - clamp)
    :duration (= ?duration 2)
    :condition (at start (free ?c))
    :effect (and
      (at start (not (free ?c)))
      (at start (held ?p))
      (at end (not (held ?p)))
      (at end (free ?c))))

  ;; Heating happens at the end of its own duration.
  (:durative-action heat-part
    :parameters (?p - part)
    :duration (= ?duration 3)
    :condition (at start (not (heated ?p)))
    :effect (at end (heated ?p)))

  ;; Forging needs the part heated (at start) AND held for the whole action
  ;; (over all). This forces concurrency with clamp-part.
  (:durative-action forge-part
    :parameters (?p - part)
    :duration (= ?duration 1)
    :condition (and
      (at start (heated ?p))
      (over all (held ?p)))
    :effect (at end (forged ?p))))
