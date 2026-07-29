(define (problem eps-cross-1) (:domain epscross)
  (:objects m1 - match f1 f2 - fuse)
  (:init (handfree) (unused m1))
  (:goal (and (mended f1) (mended f2))))
