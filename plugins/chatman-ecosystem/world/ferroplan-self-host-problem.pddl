(define (problem ferroplan-self-host-current)
  (:domain ferroplan-self-host)
  (:objects ferroplan - repository)
  (:init
    (dirty ferroplan)
    (= (pending-events ferroplan) 1)
    (= (risk ferroplan) 1)
    (= (available-capacity ferroplan) 8)
  )
  (:goal
    (and
      (receipt-bound ferroplan)
      (validator-green ferroplan)
    )
  )
)
