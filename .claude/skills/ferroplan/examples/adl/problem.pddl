(define (problem two-parcels)
  (:domain courier)
  (:objects
    pkg1 pkg2 - parcel
    home hub - location)
  (:init
    (at-courier home)
    (parcel-at pkg1 home)
    (parcel-at pkg2 home)
    (depot hub))
  (:goal (and (delivered pkg1) (delivered pkg2))))
