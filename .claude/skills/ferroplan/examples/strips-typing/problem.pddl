(define (problem move-ball)
  (:domain gripper)
  (:objects
    r1 - robot
    b1 - ball
    rooma roomb - room)
  (:init
    (at-robot r1 rooma)
    (free r1)
    (at-ball b1 rooma))
  (:goal (at-ball b1 roomb)))
