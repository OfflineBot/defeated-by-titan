# references — numbers taken from research, each with its source

Updated: 2026-08-12 · Stage: 🟨 (researched once, not attacked by a second head)

**Why this file exists.** On 2026-08-12 a research round produced the geometry the district is
built to, and it was written into a scratch directory that a later agent could no longer find —
because the session directory had changed underneath it. The numbers below are load-bearing for
`assets/data/maps.ron` and `assets/data/scale.ron`, so they live in the repository now.

**How to read it.** Every figure carries a confidence: `canonical` (stated by the source work),
`real-measured` (surveyed from the real town the district is modelled on) or `inferred`. **Where a
figure is inferred, it is a decision, not a fact** — and it is the user's to overrule.

⚠️ **We build our own game.** These are proportions and measurements used to inform geometry;
nothing here reproduces artwork or text, and every name in this project is our own
(`docs/architecture.md`'s translation table).

## The three figures that are canonical

| what | value | note |
|---|---|---|
| the wall | **50 m** | our `scale.ron: wall.height_m` is **120 m** — that is 50 × `wall_factor: 2.4`, and the project's own scaling turned out to agree with the source without anyone having checked |
| the largest titan | **60 m** | above our current `titan.ron` ceiling; see `Q-028` |
| the gate opening | **8 m** | the only size anchor for a gate anywhere in the source material |

**There is no canonical plan of the district.** The wiki material is plot, not geometry. Everything
below therefore comes from the real town the district is modelled on.

## The street-to-height ratio — the number that decides traversal

Surveyed from **Nördlingen** (Bavaria) via OpenStreetMap: **1 344 buildings, 463 ways, 4 100 street
centreline samples measured against building outlines.**

| | measured | confidence |
|---|---|---|
| street width, facade to facade | **8.1 m** (residential 7.7 m) | real-measured |
| building ridge | **11–13 m** — 167 of 246 tagged buildings are 2-storey, 65 are 3 | real-measured (storeys), inferred (absolute ridge) |
| **street : height** | **0.62 : 1** | real-measured |
| building height | ≈ **1.6 × street width** | derived |

**Why this is the most important number in the file:** a rope swings only while the horizontal gap
to the anchor is smaller than the anchor's height (`d < H`, `FINDINGS.md` FIND-041). At 0.62 : 1 the
houses **are** the traversal route — `d < H` holds from mid-street to the ridge opposite. **Dense
medieval streets and good traversal are the same design**, which is why the fix for "it does not
look right" and the fix for "the rope does nothing" were the same change.

## The build numbers derived from it

| | value | confidence |
|---|---|---|
| frontage per house | 12–14 m | real-measured |
| party walls | **zero gaps**, closed block perimeters | real-measured |
| street width | 7.5–9 m | real-measured |
| block pitch | 40–45 m | real-measured 60, tightened so courtyards stay swingable |
| ground coverage | 39 % | real-measured |
| density | ~24 buildings / ha | real-measured |
| district footprint | **900 × 470 m** | **inferred** — source estimates span 5 × 5 km down to 0.86 km and cannot be reconciled |

## What could not be established, and is therefore invented deliberately

- **The waterway.** No source gives it a course or a width, and the model town has **no river inside
  its wall at all** — only a moat. **Our canal is a gameplay decision, not fidelity**: it is a
  lowered channel with bridges, because a district needs a barrier that is crossed rather than
  flown over (`FIND-056`).
- **The garrison headquarters.** Only its *function* is canonical — gas and blades are kept there
  and it is fought over. Its size, plan and position in the district are ours (`F-019`, `FIND-064`).
- **The district's true size**, see above.
- **Anything between 12 m and 35 m.** Our own size table has a hole there, and it is why the swing
  lanes are still scaffolding rather than architecture — `docs/QUESTIONS.md` **Q-036**, open, the
  user's to answer.

## The lesson this file is also here to record

**A scratch directory is not a deliverable.** The research above cost a full round and was written
only to `/tmp`; a later agent looked for it, could not find it, and rebuilt its conclusions from
two `FINDINGS` entries instead. **Research that anything gets built on belongs in `docs/`.**

---

# The reference's movement model (Attack on Titan Revolution)

*Researched 2026-08-12. The user asked for this twice — "es ist wichtig dass es schönes physics
movement ist. die seile geben gute beschleunigung!" and "schau dir zur not noch besser das andere
spiel an zur inspiration. schau wie es in roblox ist!" — because it is a comparison to a game he
has played and we have not.*

**Scale of the reference** (Roblox games API, 2026-08-12): **1 090 209 334 visits**, **16 589
concurrent players** at the moment of the query, 1 422 459 favourites. This is not a curiosity; it
is the genre's mass-market answer and it has been tuned in public for three years.

**Where the evidence comes from.** The load-bearing source is the developers' own patch record —
posted to their Discord and reproduced verbatim on the wiki (`Template:2023 Patches` …
`Template:2026 Patches`) — plus the wiki's `Equipment` and `Game Mechanics` pages and the
dev-published Google Docs update notes. Those are dev statements, not folklore. **Fandom returns
402 to a plain fetch but 200 to its MediaWiki API** (`/api.php?action=parse&page=X&prop=wikitext`)
— that is how these were read, and it is worth remembering. **reddit.com is unreachable from
here** (403 on the JSON API, and the search tool refuses the domain outright), so the players'
own arguing is missing from this document.

## 1. The control surface — documented

From the wiki's `Equipment` page, which reproduces the in-game control list.

| Key | Action |
|---|---|
| **Q** / **E** | Fire **left** / **right** hook |
| **Q/E + W / A / S / D** | Up / left / down / right **swerve** while on the gear |
| **A ×2** / **D ×2** | Side flip — launch yourself left/right |
| **S ×2** | Backflip — launch yourself backwards |
| **B** | **Hook break**: fires **both** hooks *behind* you at valid surfaces |
| **Space (hold)** | **Boost** — let gas out for a speed increase |
| **Space (×2)** | **Mega boost** — launch a long distance **in the camera direction** |
| **M1** | Slash (can be held to delay the swing) · **R** reload |
| **Shift** | **Sprint on the ground** — *community-consensus*, not in the wiki's ODM list |
| **Ctrl** | Shift-lock (camera). Note this is why **Shift is free for sprint** |

Since a 2023 patch, `B` **auto-targets**: "backwards grapple will find the furthest object behind
your character instead of based on where your mouse is". A dedicated key that ignores the cursor
and takes the best anchor behind you is a braking/redirect move, not an aiming move.

## 2. What actually accelerates the player — **the central question**

There is **no developer statement** naming the implementation. But the patch record constrains it
hard, and the constraints all point one way.

| Evidence | Source | What it implies |
|---|---|---|
| "Default **ODM Speed** increased from **190 > 200** (max increased from **252 > 257.5**)" | 2023 patch | ODM Speed is a **speed number**, not a force or an acceleration |
| "ODM Speed started at 200 at grade E-, now **210**; ODM Control started at 100.0%, now **105.5%**" | 2024 patch | Speed is absolute, **Control is a percentage** — two different roles |
| Setting **"Gear Shift"**: "Allows you to control the **maximum speed** at which your ODM goes" (2024); since Update 4 "a flat maximum value **between 50 m/s to 500 m/s**" | 2024 + 2026 patches | A user-facing **speed cap** only makes sense if the system drives *toward a speed* |
| "**Reduced boosting speed** just holding Space" | 2024 patch | Boost is spoken of as a *speed*, not as a force |
| "**Fixed** flips/mega boost **scaling with ODM speed**" | 2023 patch | The impulse moves were deliberately **decoupled** from the speed stat — they are fixed-size launches |
| "Revamped how **momentum** works after unhooking (lasts longer)" · "Fixed sliding/rolling not conserving your momentum properly" · "Fixed **huge momentum gain** sometimes happening" · "Conquer now cancels all your momentum … so you don't go flying" | 2023–2026 patches | Momentum is a **separate, persisted, decaying quantity** that the hook drive blends with |
| "Fixed **ODM physics pausing mid air**" | 2025 patch | It is a running simulation that can stall — not an animation or a tween |

**Roblox context** (DevForum, `community-consensus`): developers building ODM overwhelmingly reach
for **velocity movers** — "most other games used BodyVelocity", with `LinearVelocity` as the modern
form, sometimes steered along a Bézier curve between player and anchor. `LineForce` is explicitly
rejected *despite* being better at tracking the anchor in real time, because it gives "no
configuration of speed, driftspeed, grapplespeed". **Rope and spring constraints are described as
buggy for this job.** The engine's own `RopeConstraint` is, like ours, a maximum-distance
constraint: it can only stop you leaving.

> **Conclusion — `inferred`, high confidence.** AoTR's hook is **not a rope constraint**. It is a
> **velocity drive toward the anchor**, its magnitude governed by the `ODM Speed` stat, capped by
> the player's `Gear Shift` setting, its turn rate governed by `ODM Control` (%), and blended with
> a preserved momentum vector that outlives the unhook. **The rope you see is a visual beam. The
> hook is the engine, not a leash.**

That is precisely what the user is describing when he says the ropes should give the acceleration.

## 3. Gas and boost — `documented`

| Mechanic | Detail |
|---|---|
| **Gas** | A **percentage bar** on the HUD. Spent by firing hooks, by boost, and by flips |
| **Refill** | Resupply stations during missions; a perk places a **portable station with 2 (→3) refills**; one skill recovers **50% ODM gas**; perks return **+1%–3.5% of the bar per titan kill** |
| **Hold Space** | Sustained boost: gas out, speed up. Nerfed once — "reduced boosting speed just holding Space" |
| **Double-tap Space** | **Mega boost**: a launch in the **camera direction**. Cooldown **2.25 s** (was 2.5 s). Does **not** scale with ODM Speed. Works while rolling/sliding (bug-fixed to do so) |
| **Cooldown feedback** | Flips and mega boost share a cooldown, shown by **the gas bar turning grey** |
| **Modifiers** | Injury "ODM-gear" **−15% ODM gas**; perks **+5%–15% ODM Gas**; a 2023 patch "increased all gas values by 130-150" |

**Answering the user's "shift soll nur mehr beschleunigung geben":** in the reference the boost is
an **independent thrust**, not a multiplier on rope movement. It is camera-directed, it costs gas,
and it works hooked, unhooked and off the ground. What the reference has that we do not is the
**split into two verbs** — a sustained hold *and* an impulse on a 2.25 s cooldown. The community
calls hook-then-immediately-double-tap the fastest traversal in the game ("boost hook").

## 4. Ground movement — `documented` that it exists, `community-consensus` on the key

- **Sprinting on foot is real and is a distinct capability**: the wiki's injury table says a **Legs**
  injury "**Loses ability to sprint and to jump**" (and −12.5% ODM speed). You cannot lose what
  does not exist.
- **Shift is the sprint key** — several independent guides say so, and the game binds shift-lock to
  **Ctrl** instead, which frees Shift. `community-consensus`.
- **Boost is not gated on being hooked**: a patch "fixed mega boosting not launching you when you
  are rolling/sliding", i.e. it is expected to fire from the ground.
- Order-of-magnitude anchor, **titan-form only, do not copy**: shifter **Walk 55 / Run 150**
  (Roblox studs/s), rebalanced in 2026 to 46.75/127.5 for one family and 66/180 for another. The
  run:walk ratio is ≈ **2.7×**.

## 5. Two hooks versus one

| Question | Answer | Confidence |
|---|---|---|
| Are the hooks independent? | Yes — one key each, both aimed at the same reticle | documented |
| Is there an explicit "both attached" move? | **Yes, one: `B`, hook break** — fires *both* hooks behind you at once. It is a brake/redirect, not a launch | documented |
| Does dual-anchoring launch or accelerate you? | **No documented mechanic.** The community's dual-hook move is offensive: fire Q+E into the nape and drive through | community-consensus |
| Is one better than two for traversal? | Genuinely contested — "1 Hook vs 2 Hooks" is a whole guide-video topic. The consensus tech is **hook switching** (re-anchor before momentum decays) plus boost, not holding two | community-consensus |

**Note for our design:** the user describes ropes that *activate* when you press forward with two
placed. The reference does **not** work that way — it gets its acceleration from every single hook,
so it never needs a two-hook special case. If our two-hook launch exists to compensate for a rope
that cannot pull, it is treating the symptom.

## 6. Aiming, and what can be hooked — `documented`

- **One small white reticle** marks where you will travel. Both hooks aim at it; the left/right
  separation is which shoulder fires and which swerve you can chain, **not** two previews.
- **The HUD prints a distance number at the reticle, and no number means out of range** — "always
  shoot the gear only if there are numbers on your screen". The range preview is a **hard yes/no**,
  not a fade. `ODM Range` moves that threshold; an **Eyes** injury cuts it by **−15%**.
- **Targeting is a raycast.** The patch record gives this away twice: "**other players are now
  excluded from any raycast checks** so you can now grapple to teammates", and later "added a
  setting to not grapple to your teammates".
- **Everything with collision is hookable by default, and exclusions are made one at a time.**
  Invisible tree collisions in Giant Forest were *removed* as hook targets; titans' **legs** were
  *fixed to be* hookable; with **aim assist** on, "the nape becomes a hook point". This is the
  strongest support in the whole document for the user's "everything hookable, including the
  ground": the reference's default is permissive and the special cases are subtractions.
- **Settings that exist around this**: `hook assist` (aim assist), `hook detach` (auto-unhook on
  kill), `Gear Shift` (speed cap), mobile lock-on camera, no-grapple-to-teammates.

## 7. The numbers, in one table

| Quantity | Value | Confidence |
|---|---|---|
| ODM Speed, grade E- | 190 → **200** (2023) → **210** (2024) | documented |
| ODM Speed, max grade | 252 → **257.5** (2023) | documented |
| ODM Control, grade E- | 100.0% → **105.5%** | documented |
| **Gear Shift** speed cap (player setting) | flat, **50 m/s – 500 m/s** | documented |
| Mega boost cooldown | **2.25 s** (was 2.5 s) | documented |
| Gas | a **% bar**; +1–3.5%/kill from perks; 50% from a skill | documented |
| Injury penalties | −15% range · −12.5% speed · −15% gas | documented |
| "Order: Advance" buff | **+10% ODM speed, +15% ODM control, 20 s**, to all players within **150 m** | documented |
| Perk stat swings | ODM Speed +10–20% · Gas +5–15% · Control +12.5–25% | documented |
| Shifter walk / run (**titan form, not the human**) | 55 / 150 → 46.75/127.5 and 66/180 | documented |
| **The unit of "ODM Speed"** | **unresolved** — see below | inferred |

**The unit problem, stated honestly.** The stat is ~210 and the cap setting is "50 m/s to 500 m/s".
Two readings fit: **(a)** the stat is studs/s and the HUD's "m" is a stud relabelled — then 210
sits neatly inside a 50–500 range, which is the simpler reading and the likelier one; **(b)**
Roblox's official 1 stud = 0.28 m, making 210 studs/s ≈ **59 m/s**, which also fits. **Under either
reading the reference cruises far above our 28 m/s reel speed** — between roughly 60 and 260 of
whatever the HUD calls a metre, with a ceiling the player can raise to 500.

## 8. What players say makes it feel good — and what the patch record says hurts

- **Momentum is the entire skill.** The guides converge on one sentence: every time you stop moving
  you are vulnerable and you waste gas; the craft is chaining anchors into an unbroken path. Named
  techniques: "momentum tech", "boost hook" (hook, then instantly double-tap Space), hook
  switching, hook-placement drills.
- **The game pays you in damage for being fast, and this is why it feels good.** One perk deals
  **maximum damage only while you are above 70%–50% of your maximum speed**; the mythic perk
  *Black Flash* converts **every 1% of speed into 0.3%–0.6% crit damage**. Speed is not decoration
  in the reference — **it is DPS**, so the movement system is also the combat system.
- **Every recurring complaint in three years of patches is a momentum-loss bug.** Momentum lost
  after rolling/sliding · lost when a skill fires · "fixed **huge momentum gain** sometimes
  happening" · hooks breaking when you get hit, when using back hooks, when a hooked titan
  despawns · "**ODM physics pausing mid air**" · mobile players stuck mid-air. The developers chase
  these one at a time for years. **The lesson: what players notice is anything that eats their
  momentum.** Not the top speed — the interruption.
- **They shipped a speed cap as a *setting*.** `Gear Shift` exists because uncapped speed is not
  universally fun, and because "how fast is fun" turned out to be per-player. That is a humbling
  data point for anyone about to pick one number.

## What our implementation does differently, and whether that is a choice or an accident

| # | Ours | The reference | Verdict |
|---|---|---|---|
| 1 | Rope is an avian `DistanceJoint`, `limits = (0, L)`: it pulls **only** when you exceed the length and **adds no energy** | The hook **is** the accelerator — a velocity drive toward the anchor | **Accident.** The biggest divergence in this document |
| 2 | Reel shortens the limit; XPBD turns that into `speed := reel_speed_m_s`, **0 → 28 m/s in one tick** | Also drives toward a *speed* (~210) — but under a **cap**, shaped by a **Control %**, blended with momentum | **Half choice, half accident.** Velocity-target is defensible; the instant binary step is not |
| 3 | `boost_m_s2: 34` along the look direction, one sustained verb | Same in kind (independent, camera-directed, gas-fed) — but **two** verbs: sustained hold **and** an impulse on a **2.25 s** cooldown | **Choice, but incomplete** — the impulse is what the community calls the fastest travel in the game |
| 4 | WASD air control 10 m/s², **gated above 6.3333 m/s** | Q/E + WASD "swerve", gated on **being hooked**, with `ODM Control` as a % stat | **Choice with an accidental edge** — the speed floor is exactly why it does nothing standing still |
| 5 | **Shift is a no-op on the ground** | Sprints on foot (Shift); sprint provably exists because an injury removes it | **Accident**, and the user asked for it directly |
| 6 | No impulse vocabulary at all: no flips, no backflip, no both-hooks-behind brake | Five such moves, all on the same gas budget, sharing one cooldown and one piece of feedback (the gas bar greys out) | **Choice for now** — but note the *feedback* trick is free and we have nothing like it |
| 7 | No speed cap, no per-player speed setting | `Gear Shift`, added after launch, 50–500 m/s | **Not yet a decision.** One line in `QUESTIONS.md`, not a build |
| 8 | Nothing in our game rewards being fast | Damage and crit damage scale with speed | **Accident of ordering.** Combat sits behind the movement gate, but the reference shows the two are one system |

**The honest summary of that table:** our rope is a *safety line* and theirs is an *engine*. Points
2–8 are tuning and scope; point 1 is a different model. No amount of `reel_speed_m_s` will make a
maximum-distance constraint feel like acceleration, because the constraint's whole job is to do
nothing until you are already at the end of the rope.

## What I could not establish

- **Any developer statement about the actual physics implementation.** Section 2's conclusion is
  inference from stat *shapes*, from a user-facing speed cap, and from three years of patch
  language. It is strong and it is consistent — but nobody at that studio has said "we assign
  velocity toward the anchor". Treat it as the best available reading, not as fact.
- **Hook range in metres.** `ODM Range` is a stat, the HUD shows a live distance, and **no absolute
  figure is published anywhere I could reach.**
- **Gas capacity and burn rate in absolute units.** Everything is a percentage of a bar. "Increased
  all gas values by 130-150" implies an internal pool in the hundreds; the unit and the drain per
  second are unknown.
- **The unit of `ODM Speed`** (section 7) — unresolved, and it decides whether the reference cruises
  at ~60 or ~210 m/s. Both readings put it far above ours, which is the part that matters.
- **Whether two anchored hooks accelerate differently from one.** The community treats it as a live
  question; the answers live in video guides that cannot be read as text.
- **The players' own words.** reddit.com is blocked to this crawler entirely, so section 8 rests on
  the patch record, the wiki's tips and guide sites. The patch record is arguably *better* evidence
  — a bug fixed twice is a complaint made a hundred times — but the tone, the arguments and the
  minority opinions are missing.
- **How any of it feels.** Nobody in this loop has played it, and that gap does not close with
  research. It is why the user keeps pointing at the other game.

### Sources

- Roblox games API (`games.roblox.com/v1/games`, universe 4658598196) — scale figures, 2026-08-12
- Official AoT Revolution Fandom wiki via MediaWiki API: `Equipment`, `Game Mechanics`,
  `Skill Trees`, `Perks`, `Tips and Tricks`, `Patches`, `Updates`
- Developer patch notes reproduced on that wiki: `Template:2023/2024/2025/2026 Patches`,
  `Template:Update_4` — the load-bearing source for every number above
- Developer update documents published as Google Docs, linked from the wiki's `Updates` page
- Roblox DevForum threads on ODM/3DMG implementation (LineForce vs BodyVelocity; components of ODM
  gear) — for the engine-level norm, not for AoTR specifically
- Community control guides (Gamezebo, Droid Gamers, TheGamer, Sportskeeda, GameRant) — used only
  for keybinds and only where several agree
