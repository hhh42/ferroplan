(define (domain ferroplan-self-host)
  (:requirements :strips :typing :negative-preconditions :fluents)
  (:types repository)

  (:predicates
    (dirty ?r - repository)
    (observed ?r - repository)
    (rdf-admitted ?r - repository)
    (allocated ?r - repository)
    (planned ?r - repository)
    (source-changed ?r - repository)
    (build-green ?r - repository)
    (bcinr-green ?r - repository)
    (validator-green ?r - repository)
    (receipt-bound ?r - repository)
    (draft-pr-open ?r - repository)
    (blocked ?r - repository)
  )

  (:functions
    (pending-events ?r - repository)
    (risk ?r - repository)
    (available-capacity ?r - repository)
  )

  (:action observe-repository
    :parameters (?r - repository)
    :precondition (and (dirty ?r) (not (blocked ?r)))
    :effect (and (observed ?r)))

  (:action admit-rdf-projection
    :parameters (?r - repository)
    :precondition (and (observed ?r) (not (blocked ?r)))
    :effect (and (rdf-admitted ?r)))

  (:action allocate-work-frontier
    :parameters (?r - repository)
    :precondition (and (rdf-admitted ?r) (not (blocked ?r)))
    :effect (and (allocated ?r)))

  (:action manufacture-plan
    :parameters (?r - repository)
    :precondition (and (allocated ?r) (not (blocked ?r)))
    :effect (and (planned ?r)))

  (:action implement-source-change
    :parameters (?r - repository)
    :precondition (and (planned ?r) (not (blocked ?r)))
    :effect (and (source-changed ?r) (dirty ?r)))

  (:action build-exact-workspace
    :parameters (?r - repository)
    :precondition (and (source-changed ?r) (not (blocked ?r)))
    :effect (and (build-green ?r)))

  (:action run-bcinr-admission
    :parameters (?r - repository)
    :precondition (and (build-green ?r) (not (blocked ?r)))
    :effect (and (bcinr-green ?r)))

  (:action run-independent-validation
    :parameters (?r - repository)
    :precondition (and (build-green ?r) (bcinr-green ?r) (not (blocked ?r)))
    :effect (and (validator-green ?r)))

  (:action bind-receipt-chain
    :parameters (?r - repository)
    :precondition (and (planned ?r) (validator-green ?r) (not (blocked ?r)))
    :effect (and (receipt-bound ?r)))

  (:action open-draft-pull-request
    :parameters (?r - repository)
    :precondition (and (receipt-bound ?r) (validator-green ?r) (not (blocked ?r)))
    :effect (and (draft-pr-open ?r)))
)
