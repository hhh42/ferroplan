;; p1: 1 jobs x 2 stages x 2 machines (~4 operate groundings). smoke
(define (problem p1) (:domain jobshop)
  (:objects j1 - job  s1 s2 - stage  m1 m2 - machine)
  (:init
    (next s1 s2)
    (at-stage j1 s1) (final-stage j1 s2)
    (route j1 s1 m1) (= (proctime j1 s1) 4)
    (route j1 s2 m2) (= (proctime j1 s2) 2)
    (free m1)
    (free m2)
  )
  (:goal (and (job-complete j1))))
