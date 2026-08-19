# world — the setting, the tone, the visual style and the platform, and why each of them is a constraint on the code

Updated: 2026-08-12 · Stage: 🟨 (carried over out of the design bible; the style rules are
partly enforced by tests, the world itself is a stub — `world::map::build_map` still builds
nothing)

## Setting

Humanity lives in **three concentric bastion rings** — **Ashgate** outside, **Ironrose** in the
middle, **Highspire** inside. The rings were raised against the titans over a hundred years ago,
and nobody who left them has come back.

**The central difference to the source material: the war is already lost.** The title is not a
threat, it is a statement of fact. Ashgate has long since fallen; the Vanguard runs **salvage
missions into its own ruins**, not campaigns of reconquest.

That one decision pays for the entire mission structure. You rescue carts, hold positions and
withdraw — you do not take territory. It moves the tone from heroic to elegiac, and it is the
reason a mission is allowed to end in a retreat without feeling like a loss.

**The Vanguard is a salvage corps, not an army.** Ranks are grades of craft, not military ranks.
That lets progression be told as competence rather than promotion, which is exactly what pillar
P2 asks for ([`pillars.md`](pillars.md)).

## What titans are

**Deliberately left unanswered.** The compendium collects the Vanguard's field observations, not
truths. Vessel Forms are described as an **infection**, not a gift — the player uses something he
does not understand and that costs him.

> **The narrative rule: we never explain more than a character in the field could know.**

That is a rule for writing HUD text, compendium entries and log lines as much as for cutscenes.
A tooltip that states a titan's regeneration rate as a fact has broken it.

## Tone

**Muted, adult, without cynicism.** No gore: **titans vaporize instead of bleeding**, wounds vent
steam.

That was a style decision *and* a platform-moderation decision on Roblox. Off Roblox the
moderation half falls away and **the style half stays** — it was doubly justified, and it is the
better of the two reasons: steam reads at a distance and at speed, blood does not.

## Visual style

**Low poly with soft normals and flat color surfaces.** No hard-facetted look, no PBR detail. The
**entire environment runs off one single color atlas** — guaranteed color consistency, minimal
draw calls, and a recognizable hand.

| Layer | How it gets its color | Why |
|---|---|---|
| Environment | **one atlas** | color consistency and draw calls, both at once |
| Figures and titans | **vertex colors** | survives any remodeling, needs no UV work |

Which one applies to which asset stands in the registry (`farbe:` / the art RON), **not in the
code** ([`../models.md`](../models.md)).

### The three signal colors — not negotiable

| Color | Meaning | May appear nowhere else |
|---|---|---|
| **Cyan** | gas, Vector Gear, anchor points | no cyan set dressing |
| **Amber** | cortex, weak points, objectives | no amber lanterns |
| **Crimson** | danger, damage, critical state | no red roofs |

The base palette stays muted against them: stone gray, brick red, olive green, sand brown.

**This is the rule that makes the game readable at speed**, and it holds for placeholders exactly
as much as for finished art — a placeholder that breaks the style falsifies the very judgement
the prototype exists to produce. The norm and its enforcement live in
[`../conventions.md`](../conventions.md) §3.

### Lighting

A **strong directional light** and **aggressive distance fog** for depth layering. The fog works
twice: atmosphere and culling. Bevy's PBR plus a `DirectionalLight` plus fog is the translation
of the bible's "Future Lighting" — **the style stays exactly as specified**, only the mechanism
changes ([`../architecture.md`](../architecture.md), translation table).

> ⚠️ **This paragraph used to say "Neither exists yet" and that is STALE since 2026-08-13.**
> Both are built: `src/render/light.rs::setup_sky` raises a **dome** (three linear-RGB stops out of
> `art.ron: lighting.sky`, zenith → horizon → nadir, `radius_m` 820, pinned to the eye) and
> `camera_light_settings` returns a real `DistanceFog` whose colour **is** the horizon stop, so a
> distant block dissolves into the sky rather than into a grey wall.
>
> **What is true is that they do not READ yet.** Two independent rounds looking at 2026-08-18/19
> frames called the result *"flat/harsh, half the image near-black"* and *"a flat grey sky"*.
> That is a tuning or a wiring fault, not a missing feature — **do not re-implement it**, measure
> why the dome and the fog are not doing their job (`docs/NEXT.md` §3).

## Platform

**PC only. Keyboard and mouse as the only input device.** No mobile, no gamepad, no touch.

That is a design *advantage*, not a restriction, and it runs through the whole backlog:

- **The aiming system may be built for mouse precision.** Snap (`F-024`) becomes a comfort option
  instead of a necessity — *Assisted* stays the default and *Free* is a realistic choice for
  ambitious players.
- **There is no lowest common denominator for the HUD.** More information can be on screen at
  once, because no thumb covers half of it.
- **Control depth is not a problem.** Q, E, B, C, F, Shift, Ctrl, double-taps and modifiers side
  by side are reasonable on a keyboard and unreasonable on a gamepad.
- **Two quality profiles instead of five:** a **minimum profile** (entry-level laptop, integrated
  graphics) and a **full profile**. **Both target 60 FPS** — see
  [`../lessons/performance.md`](../lessons/performance.md), where that number is also where the
  budget gaps are recorded.

**The accessibility requirements survive in full**: free key rebinding, colorblind modes, a
screenshake slider, motion reduction. One platform fewer does not mean less care.

## Multiplayer, as far as it is a world decision

**Co-operative, not competitive.** Twenty players per mission instance, ten per raid, forty in
the hub. The four ground rules — no damage between players, no collision between players,
separate loot per player, no kicking in public instances — and everything they force on the
architecture are in [`../multiplayer.md`](../multiplayer.md), because they are consequences for
the code long before they are consequences for the fiction.

The one that is genuinely a design decision rather than a technical one: **downed instead of
instant death.** It creates the most valuable moment in the whole co-op design — a team mate
deciding whether to land in the middle of titan fire to pick somebody up. The revive is
deliberately slow enough to be risky. Solo players get a limited self-revive so that solo stays
playable.

### Session structure

| Level | Place | Lifetime |
|---|---|---|
| Hub | persistent instance, 40 players | permanent |
| Group | travels together between scenes | until it dissolves |
| Mission | reserved instance, 10–20 players | until it completes, then torn down |

Joining runs over **quick search** (`F-152`, one click) or an **instance browser** (`F-153`,
manual). Joining a mission in progress is allowed; joining a raid after a phase has started is
not. **A dropped connection holds the slot for 120 s** (`F-158a`) — somebody with unstable
internet does not lose half an evening.

Related: [`README.md`](README.md) · [`pillars.md`](pillars.md) · [`enemies.md`](enemies.md) ·
[`../conventions.md`](../conventions.md) · [`../models.md`](../models.md) ·
[`../multiplayer.md`](../multiplayer.md)
