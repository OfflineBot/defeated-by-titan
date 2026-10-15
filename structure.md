# Structure

## Game loop

1. Hub (the garrison). Safe. You come back here after every run. Walk to the mission
   board, F opens it, F cycles entries, hold F deploys.
2. Sortie. Mission types: tutorial, skirmish (recruit / veteran / elite), parcours
   (pure movement, timed), breach, escort, plus rescue/hold objectives. Objective and
   kill counter live on the HUD.
3. Debrief: XP, earnings, stats.
4. Progression: earnings go into gear - blade tiers, tank size, hook parameters. Points
   are visible and spendable, everything persists. The progression is meant to be built
   out fully once the movement bar is met.
5. Death or failure: back to the hub. Mission progress is lost, gear is kept.

Multiplayer is designed-for from day one (nothing hardwired to a single player), even
though the netcode comes later. Sound is supplied by the project owner.

## HUD

- Crosshair: an X of four strokes rotated 45 degrees, center completely empty. Size and
  color are settings. Each stroke has a thin contrast rim so it reads on any background.
  No circles, no rings, no letters near the center - all of that existed once and was
  removed for good.
- Crosshair color semantics: cyan = a rope holds here (titan bodies included), amber =
  cortex in sight, white = nothing / free.
- Gas gauge, blade pips, health, objective - all at the edges. The center of the screen
  belongs to the game.
- Settings (Escape): sensitivity, FOV, assist strength and mode, hook toggle/hold,
  crosshair size and color, and fully rebindable keys (conflicts resolve by swapping).
  Changes apply instantly and persist.

## Controls (defaults)

| Input | Effect |
|---|---|
| WASD | Run. On a rope: W thrusts toward your look, A/D strafe around the anchor, S cancels the pull |
| Mouse | Look/aim. Hanging: sideways look capped near 80 degrees each way |
| Q / E | Left / right hook. Tap fires (toggle), tap while anchored refires, long press releases |
| Mouse wheel | Rope spread |
| LMB / RMB | Left / right blade |
| Space | Gas boost |
| Ctrl | Reel in |
| Shift | Sprint (final tuning open) |
| F | Interact / mission board |
| Escape | Menu / settings |
