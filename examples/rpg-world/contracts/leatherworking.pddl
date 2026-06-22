(define (problem leather-contract-1)
  (:domain rpg-world)
  (:objects
    tess - agent
    workshop - location)
  (:init
    (at tess workshop)
    ;; stations clustered at the one location
    (has-tannery workshop)
    (has-leather-bench workshop)
    ;; roles
    (tanner tess)
    (leatherworker tess)
    ;; pre-delivered raw input from hunting subsystem
    (= (hide) 8)
    ;; every fluent this subsystem references, initialized
    (= (leather) 0)
    (= (leather-armor) 0)
    (= (bags) 0)
    ;; trivial self-link so reachable is well-formed (no travel needed)
    (link workshop workshop)
    (= (dist workshop workshop) 1))
  (:goal (and (made-armor) (made-bag)))
  (:metric minimize (total-time)))