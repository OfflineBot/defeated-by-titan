# Movement

The heart of the game. These behaviors are fixed; they were tuned over many playtests on
the previous build and relearning them would waste that work.

## Hooks

Q fires the left hook, E the right one.

Any surface takes a hook. No special anchor points, no whitelist. (Rare deliberate
exceptions can exist later - things like water - but they stay rare.)

Hooks toggle. A tap fires and the rope stays attached, no holding required. Tap the same
side again while anchored and it releases and refires at your current aim in one motion.
A long press is a plain release. A shot that finds nothing clears itself. Hold-mode is
available in the settings for people who prefer it.

The marker is the truth. The point you see marked is the point the rope hits, exactly,
even at full speed. Any gap between marker and landing point is treated as a bug.

The hook travels fast and far - target range is around 500 m - but you can watch the line
unroll. Fast enough to feel instant, visible enough to read.

The two ropes spread apart sideways, and the mouse wheel sets how far apart they may go.
The spread adapts on its own: tight while you're moving fast, wider when you stand and
look around.

Aim assist is a 0-100 slider with three modes: free aim, assisted (default - snaps only
inside a catch angle), and full snap. The snap only searches sideways, never up or down
from the crosshair. One axis keeps it predictable.

## The rope

1. The rope has a hard maximum length and works like a ratchet: once shortened, it never
   lengthens on its own. At the limit it's a real constraint - you swing on the sphere
   around the anchor instead of drifting away.
2. Attached means pulled. The moment a rope connects you get drawn toward the anchor,
   by default, even standing on the ground, without pressing anything or looking at it.
3. W adds thrust in the direction you're looking, on top of the pull. From a standstill,
   W toward the anchor should feel like being yanked there, not like walking.
4. A and D move you sideways relative to the anchor - along the sphere, not along the
   camera. The base pull stays on while strafing, just weaker: the harder you go
   sideways, the less pull. A/D can never lengthen the rope.
5. S cancels the pull. Sideways motion stays, the distance stops shrinking, and it never
   grows. S never drags you toward the rope.
6. Ctrl reels in actively, much stronger than the base pull.
7. Two ropes at once is a normal state, not an error. Both hold, you hang tensioned
   between them. That's how you build stand-offs and line yourself up before a dive.

While hanging, the sideways look is limited to about 80 degrees each way so the camera
can't wander behind your own anchor.

## Feel

- The world is heavy. Gravity is high (raised repeatedly after "too floaty" verdicts),
  the jump is strong, running is 6 m/s. The contrast is what makes swinging feel big.
- Momentum is conserved. A clean arc carries 30-60+ m/s. Swinging around an anchor is
  correct behavior, not overshoot.
- Gas is the movement resource. Boost (Space), steering under a rope, and active reeling
  all burn gas. An empty tank halves your steering and kills the boost. Refill at supply
  stations. Costs should be felt but not stingy.
- The field of view widens with speed - any direction - opens quickly, returns slowly.
  A soft transition, never a snap.
- Stopping from a sprint runs out instead of sliding. Sliding only happens above a
  minimum speed.
