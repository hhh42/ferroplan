(define (domain ferroplan-self-host)
  (:requirements :strips :typing :negative-preconditions :fluents)
  (:types repository)

  (:predicates
    (epistemic-latent ?r - repository)
    (epistemic-observed ?r - repository)
    (epistemic-admitted ?r - repository)

    (unallocated ?r - repository)
    (allocated ?r - repository)

    (unplanned ?r - repository)
    (candidate-plan ?r - repository)
    (validated-plan ?r - repository)

    (actuation-sealed ?r - repository)
    (manufacturing ?r - repository)
    (receipted ?r - repository)
    (publishable ?r - repository)

    (stable ?r - repository)
    (drifted ?r - repository)
    (refused ?r - repository)

    (config-unknown ?r - repository)
    (config-nonconformant ?r - repository)
    (config-conformant ?r - repository)

    (dirty ?r - repository)
    (source-changed ?r - repository)
    (build-green ?r - repository)
    (bcinr-green ?r - repository)
    (validator-green ?r - repository)
    (allocation-bound ?r - repository)
    (plan-bound ?r - repository)
    (receipt-bound ?r - repository)
    (draft-pr-open ?r - repository)
    (blocked ?r - repository)
  )

  (:functions
    (pending-events ?r - repository)
    (risk ?r - repository)
    (available-capacity ?r - repository)
  )

  (:action inspect-configuration
    :parameters (?r - repository)
    :precondition (and (config-unknown ?r) (not (blocked ?r)))
    :effect (and
      (config-conformant ?r)
      (not (config-unknown ?r))))

  (:action repair-configuration
    :parameters (?r - repository)
    :precondition (and (config-nonconformant ?r) (not (blocked ?r)))
    :effect (and
      (config-conformant ?r)
      (not (config-nonconformant ?r))))

  (:action observe-latent-repository
    :parameters (?r - repository)
    :precondition (and
      (epistemic-latent ?r)
      (dirty ?r)
      (not (blocked ?r)))
    :effect (and
      (epistemic-observed ?r)
      (not (epistemic-latent ?r))))

  (:action admit-rdf-projection
    :parameters (?r - repository)
    :precondition (and
      (epistemic-observed ?r)
      (config-conformant ?r)
      (not (blocked ?r)))
    :effect (and
      (epistemic-admitted ?r)
      (not (epistemic-observed ?r))))

  (:action allocate-work-frontier
    :parameters (?r - repository)
    :precondition (and
      (epistemic-admitted ?r)
      (unallocated ?r)
      (not (blocked ?r)))
    :effect (and
      (allocated ?r)
      (allocation-bound ?r)
      (not (unallocated ?r))))

  (:action manufacture-candidate-plan
    :parameters (?r - repository)
    :precondition (and
      (allocated ?r)
      (unplanned ?r)
      (not (blocked ?r)))
    :effect (and
      (candidate-plan ?r)
      (plan-bound ?r)
      (not (unplanned ?r))))

  (:action begin-reversible-manufacturing
    :parameters (?r - repository)
    :precondition (and
      (candidate-plan ?r)
      (actuation-sealed ?r)
      (not (blocked ?r)))
    :effect (and
      (manufacturing ?r)
      (not (actuation-sealed ?r))))

  (:action implement-source-change
    :parameters (?r - repository)
    :precondition (and
      (manufacturing ?r)
      (candidate-plan ?r)
      (not (blocked ?r)))
    :effect (and
      (source-changed ?r)
      (dirty ?r)
      (epistemic-observed ?r)
      (unallocated ?r)
      (unplanned ?r)
      (actuation-sealed ?r)
      (drifted ?r)
      (config-unknown ?r)
      (not (epistemic-admitted ?r))
      (not (allocated ?r))
      (not (candidate-plan ?r))
      (not (manufacturing ?r))
      (not (stable ?r))
      (not (config-conformant ?r))
      (not (allocation-bound ?r))
      (not (plan-bound ?r))))

  (:action build-exact-workspace
    :parameters (?r - repository)
    :precondition (and
      (source-changed ?r)
      (not (blocked ?r)))
    :effect (and (build-green ?r)))

  (:action run-bcinr-admission
    :parameters (?r - repository)
    :precondition (and
      (build-green ?r)
      (epistemic-admitted ?r)
      (not (blocked ?r)))
    :effect (and (bcinr-green ?r)))

  (:action run-independent-validation
    :parameters (?r - repository)
    :precondition (and
      (candidate-plan ?r)
      (build-green ?r)
      (bcinr-green ?r)
      (config-conformant ?r)
      (not (blocked ?r)))
    :effect (and
      (validated-plan ?r)
      (validator-green ?r)
      (not (candidate-plan ?r))))

  (:action bind-receipt-chain
    :parameters (?r - repository)
    :precondition (and
      (validated-plan ?r)
      (validator-green ?r)
      (allocation-bound ?r)
      (plan-bound ?r)
      (actuation-sealed ?r)
      (not (blocked ?r)))
    :effect (and
      (receipted ?r)
      (receipt-bound ?r)
      (stable ?r)
      (not (actuation-sealed ?r))
      (not (drifted ?r))))

  (:action admit-publication-phase
    :parameters (?r - repository)
    :precondition (and
      (receipted ?r)
      (epistemic-admitted ?r)
      (allocated ?r)
      (validated-plan ?r)
      (stable ?r)
      (config-conformant ?r)
      (not (blocked ?r)))
    :effect (and
      (publishable ?r)
      (not (receipted ?r))))

  (:action open-draft-pull-request
    :parameters (?r - repository)
    :precondition (and
      (publishable ?r)
      (receipt-bound ?r)
      (validator-green ?r)
      (not (blocked ?r)))
    :effect (and (draft-pr-open ?r)))
)
