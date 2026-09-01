# QUESTIONS — archive, the ones that are answered

Updated: 2026-08-29 · Stage: 🟧 (answered, and moved so the live file shows what is OPEN)

> **Why this file exists.** `docs/QUESTIONS.md` had reached **222 kB across 84 headings**, and a
> file that mixes answered and open questions cannot answer the one thing it is for: *what is
> still mine to decide?* Everything here carries an answer, a resolution or a supersession in its
> own body. **Nothing was deleted, and every `ASSUMPTION:` and rollback point came with it.**
>
> Grep for the id; never open this whole.

## The index

- **Q-002 — Is 1 stud = 0.28 m the right conversion factor?**
- **Q-006 — Music: own composer or a licensing library?**
- **Q-009 — Does offscreen rendering really deliver an image on machine A? *(half answered, see the addendum)***
- **Q-024 — German or English in the source? The instruction contradicts itself**
- **Q-033 — ✅ ANSWERED 2026-08-12: gas refills ONLY at stations. My regeneration was wrong.**
- **Q-037 — On the last drop of gas: does `Shift` win, or does `W` on the rope? (2026-08-13, W4)**
- **Q-031 — RESOLVED as option (2) on 2026-08-13, on your instruction, indirectly**
- **Q-038 — the aim assist is a machine setting, and multiplayer has no machine  ✅ DECIDED (2)**
- **Q-039 — ✅ ANSWERED (2026-08-19) — looking straight down: should the fan collapse, or is "within what it asked for" enough?**
- **Q-042 — ✅ DECIDED (2) on 2026-08-20 — the search band needed BOTH assist knobs up before it appeared, and that was a trap on the settings screen**
- **Q-048 — both ropes to the crosshair, or the left/right split you asked for? ✅ ANSWERED**
- **Q-049 — Pendulum or Drive? The switch is built, both work, and only you can answer it**
- **Q-050 — under `Drive` the reel does nothing and `A`/`D` alone is a brake. Both are consequences, not decisions**
- **Q-052 — five movement verbs landed. Four numbers in them are yours, and one of them replaces the gas price as the thing that limits a dash (2026-08-24)**
- **Q-055 — ⛔ SUPERSEDED BY `Q-056`, and the assumption in it was never implemented**
- **Q-057 — the rope is a ratchet now: how hard may it catch you, and does `min_rope_m` become a leash? (2026-08-26)**
- **Q-058 — ✅ ANSWERED BY HIM, 2026-08-27: yes. `Drive` gets a real rope.**
- **Q-060 — what should the hub line say when it genuinely cannot tell you which door opens?**
- **✅ ANSWERED 2026-08-27 — the interactive run-through, batch 1 of 6**
- **✅ ANSWERED 2026-08-27 — batch 2 of 6**
- **✅ ANSWERED 2026-08-27 — batch 3 of 6**
- **✅ ANSWERED 2026-08-27 — batch 4 of 6**
- **✅ ANSWERED 2026-08-27 — batch 5 of 6**
- **✅ ANSWERED 2026-08-27 — batch 6 of 6, the run-through is complete**
- **Q-076 — ✅ ANSWERED 2026-08-27: the player gets **real mass**. Forces, not accelerations.**
- **Q-077 — 🔴 „zu leicht" is STILL OPEN, and `Q-076` is not its answer**
- **Q-078 — ✅ ANSWERED 2026-08-27: **everything is hookable.** The tag system goes too.**
- **Q-078 (addendum) — ✅ BUILT 2026-08-28. The rollback point moved: it is a **value**, not a condition**
- **Q-062 — the board is built; `hold F` is my choice and not yours, and `Q-059`/`Q-060` are now moot (2026-08-28)**
- **✅ ANSWERED 2026-08-29 — the four he could answer without the controller**
- **✅ ANSWERED 2026-08-29 — the water and the wall**

---

## Q-002 — Is 1 stud = 0.28 m the right conversion factor?

**Context:** backlog and bible count in studs (the Roblox unit), this project in meters
(`prompts/init.md` §3: 1 Bevy unit = 1 meter). The factor determines every distance in the
game.

**Evidence for 0.28** (the Roblox value), three independent cross-checks:

| Backlog | × 0.28 | matches |
|---|---|---|
| hook range 400 studs (`F-002`) | 112 m | `prompts/init.md` **§3** line 411: „ein Haken fliegt 60–120" (*a hook flies 60–120*) — cited as §1 here until 2026-08-10, wrong section. Superseded anyway: the range is **200 m** since Q-035 |
| Ashgate District 2000 × 2000 studs | 560 × 560 m | mission arc 5–7 min |
| Titanwood 3000 × 3000 studs | 840 × 840 m | largest map |

**ASSUMPTION:** 0.28 m/stud. The conversion happens **once**, when a number is taken over into
an `assets/data/*.ron`; there are no studs in the code. If the factor is wrong, only RON
numbers change, no code.

### Addendum 2026-08-09 — the first cross-check has fallen away

The user delivered a size table and gave the anchor range in it **directly as 90 m**
(`assets/data/scale.ron: vector.anchor_range_m`). That makes the precedence rule apply: **a
direct figure in meters from the user beats any derivation.** `game.ron` now stands at 90 m,
no longer at 112 m.

That hits the justification of the factor at the root: the 112 m **were** the first of the
three cross-checks above. Calculated backwards, 90 m / 400 studs would give a factor of
**0.225 m/stud** — 20% below 0.28. Exactly one of two possibilities follows, and **which one
is the user's call, not mine**:

1. **0.28 keeps holding for everything else**, and the hook range is simply a game-value
   decision that has nothing to do with the backlog number. Then Ashgate (560 × 560 m) and
   Titanwood (840 × 840 m) stay as they are.
2. **The factor is too high altogether.** Then every number converted so far shrinks by 20%:
   Ashgate to 450 × 450 m, Titanwood to 675 × 675 m — and every map would have to be
   recalculated.

**ASSUMPTION (until answered):** possibility 1. The factor 0.28 stays for everything the user
has said **nothing** about; where he gives a figure in meters, his figure holds and the
conversion is not consulted at all. The two remaining cross-checks (Ashgate, Titanwood) carry
the factor alone — **that is thinner than before**, and the stage in `docs/conventions.md` §1
is to be read accordingly.

**To roll back:** the line `1 stud = 0.28 m` in `docs/conventions.md` §1 and every `size_m` in
`assets/data/maps.ron` that came out of a stud number. Not affected: everything from
`assets/data/scale.ron` — that comes straight from the user and was never converted.

## Q-006 — Music: own composer or a licensing library?

**Context:** bible 8.4 and bible 6.4 (risk: audio rights): **exclusively original or licensed
music**. Affects budget and schedule from P4 onwards.

**ASSUMPTION:** until answered there are only **sound recipes** (`tools/sound/*.py`,
reproducible) and CC0 placeholders under `assets/extern/` with a line in `ATTRIBUTION.md`. No
third-party music in the repo, not even as a placeholder.

## Q-009 — Does offscreen rendering really deliver an image on machine A? *(half answered, see the addendum)*

**Context:** `prompts/init.md` §14 allows as an exception that an image comes out of a render
target instead of a window — **but only once it is proven that it really does deliver a PNG on
the N100.** Claimed, it is worth nothing. Without that proof the ceiling on machine A is
**🟨**, and this project has **not one image** so far.

**ASSUMPTION:** it does not work until it is measured. Everything built on A stays 🟨 with the
note *"logic tested, pixels unseen — machine A"*. Recorded as a task in `docs/TODO.md`.

### Addendum 2026-08-09 — half of it is measured, and it is the half that decides

Measured on `[cachy]` with the new `--offscreen` (`src/debug/screenshot.rs`), not explained:

| Question | Result |
|---|---|
| Does a render-target image **without a window** deliver a PNG? | **Yes.** 1280x720, full scene, exit 0 |
| Also **without a graphics session**? | **Yes.** `env -u WAYLAND_DISPLAY -u DISPLAY` changes nothing about the result — winit is switched off, `ScheduleRunnerPlugin` drives the app |
| Is it reproducible? | **Yes, bit for bit.** Two runs, `sha256 = eb212dfe…` both times |
| Why did it not work before? | Because `--headless` set **`backends: None`** and therefore never even looked for an adapter. "No window" and "no GPU" were the same decision — that was the real reason, not a limit of Bevy. `bevy_render-0.19.0/src/lib.rs:501-506` shows it: the window is an `Option` when the renderer is built |

**What is NOT proven by this:** that the **N100 under debian** finds a wgpu adapter.
`[cachy]` has an RTX 3080 with a Vulkan driver; the measurement shows that **no window and no
compositor** are needed, not that every GPU plays along.

**The question therefore shrinks** from "does offscreen rendering work at all?" to **"does
machine A find a wgpu adapter?"** — and that is no longer a design question but a single
measurement on A: `cargo run -- --offscreen --script scripts/t006-shot-near.txt --ticks 110
--screenshot docs/images/t006-player-view.png`. If it comes out well, the ceiling on A rises
from 🟨 to 🟧. Until then the assumption stands for **A**; for **B** it is settled.

### ANSWERED 2026-08-09 — by measurement on A, not by a decision

**Machine A finds an adapter.** Measured on `[debian]` by two independent jobs in the same
session, each reporting it without knowing the other was looking:

```
AdapterInfo { name: "Intel(R) Graphics (ADL-N)", device_type: IntegratedGpu,
              backend: Vulkan, driver: "Intel open-source Mesa driver",
              driver_info: "Mesa 25.0.7-2" }
```

Four PNGs were produced on A in that session, every one of them **bit-identical over repeated
runs**: `p1-overlay.png` (`sha256 054aaeff…`, three runs, 625 728 B), `b001-anchor.png`
(`aaf52739…`), `f056-husk.png` (`ade7a6b7…`), `f050-states.png` (`8c20c551…`).

**This is the question that gated everything.** Without an adapter on A, nothing built on A
could ever have carried a picture, and 🟧 needs a picture — so the whole project would have been
capped at 🟨 on its own main machine. **The ceiling on A is now 🟧.** The assumption above is
withdrawn; the `T-006` note in `docs/features.ron` no longer applies and has been corrected.

**What is still not proven:** that a **window** run works on A. Only `--offscreen` was measured.
Nobody has seen this game in a window on this machine, and that is a different question
(`docs/umgebung.md`).

## Q-024 — German or English in the source? The instruction contradicts itself

**Context:** on 2026-08-09 the user wrote, **verbatim**:

> „programmiere alles in deutsch. filebenennung! code aber auch comments alle auf englisch!"

The sentence contains both: *„alles in deutsch"* (*everything in German*) and *„alle auf
englisch"* (*all in English*). Two readings are possible, and they lead to completely
different work on **7815 lines in 53 files**:

1. **Everything stays German.** Then "code aber auch comments alle auf englisch" is a slip of
   the pen and there is nothing to do — `docs/conventions.md` said exactly that until today
   ("one language: German, throughout").
2. **File names German, identifiers and comments English.** Then *„filebenennung!"* qualifies
   the preceding *„alles in deutsch"*, and the *„aber"* marks the contrast for everything
   else.

**Addendum 2026-08-09, shortly afterwards — the user sharpened it:**

> „wenn nicht sicher dann eher englisch!"

*(if in doubt, rather English)*

That **confirms** reading 2 and answers the question at its core. It stays here anyway,
because it set a tiebreaker that goes beyond the wording: **German holds only where it is
named explicitly** (file names, commit messages, `docs/`); every doubtful case goes to
English. Two cases moved because of it that stood in German in the first version of this
question: log and HUD output, and the metrics of the script driver (`speed|tempo`,
`height|hoehe`, `titans|titanen` — the German second names are dropped).

**ASSUMPTION: reading 2.** Three reasons, each weak on its own, together carrying:
*„filebenennung!"* stands with an exclamation mark **directly behind** "alles in deutsch" and
reads as its qualification; the *„aber"* before "comments" only makes sense at a language
switch; and reading 1 would be an instruction that changes nothing — the user does not write
an instruction so that everything stays as it is.

The border was written out in [`docs/conventions.md`](../conventions.md) §4. In short: files,
folders and module names German · types, fields, functions, comments, test names and RON keys
English · commit messages, `docs/` and everything a **player** reads (HUD, log lines) German.

**To roll back:** exactly one commit. The migration runs as **one** conversion, not
step by step — half-German code would be worse than either of the two answers. A `git revert`
of that one commit restores the German state completely; after that only
`docs/conventions.md` §4 and this question have to be turned back. Everything that arrives
**new** between migration and answer follows reading 2 and would have to be brought along
under reading 1 — which is why this question stands here near the top, not at the bottom.

**Addendum 2026-08-09, later the same day — the user removed the last limit:**

> „es sootlle im bestfall nirgendwo deutsch sein! alles auf englisch!"

*(ideally there should be no German anywhere — everything in English)*

That sentence overrides both earlier ones and, with them, the whole border above:
**everything becomes English** — file names, identifiers, comments, RON keys, documentation,
tool output, log lines, the commit norm. German remains only in the user's own documents
(`prompts/`, `gameplay/`), in the quotations taken from them, and in the git history, which is
a record and not documentation. The section about the two languages in
[`docs/conventions.md`](../conventions.md) §4 is therefore **deleted rather than translated**;
the one rule that replaces it is: *everything is English.*

The question is thereby **answered by the user himself** and stands here only as the record of
how the answer came about. What the migration leaves open is a matter of wording, and it
belongs to the user rather than to me:

- **the four stage names** `Unbuilt · Built · Proven · Accepted` — ✅ is the user's marker
  (rule 1), and the migration renames it;
- **whether a German-relapse checker ships at all**: after the migration, German in `src/`,
  `assets/data/`, `tools/` and `scripts/` is itself a violation, and nothing checks that
  today.

**To roll back:** still exactly one commit — the migration commit itself.

## Q-033 — ✅ ANSWERED 2026-08-12: gas refills ONLY at stations. My regeneration was wrong.

> **The user's answer, from `user-messages.md`:** *"(gas refillt nur im main gebäude an bestimmten
> stationen/objekten)"* — gas refills **only in the main building, at certain stations/objects.**
>
> **That kills the assumption I ran under.** I chose "regenerates while neither boosting nor
> reeling" because he had not answered, and I said in this entry that the shape was his to pick. He
> picked, and he picked none of my three options: refuelling is a **place you go to**, not a timer.
>
> **What has to be rolled back, exactly as this entry promised:**
> - `assets/data/game.ron`: `gas_regen_per_s: 10.0` → **0.0** (the entry already records that 0.0
>   restores the old behaviour exactly) and `gas_regen_delay_s` becomes dead.
> - `src/vector/gas.rs`: the `refill_tank` / `arm_pause` branch comes out, and `Gas::regen_delay_left_s`
>   with it.
> - The four red tests in `tests/vector_gas.rs` that pin the regeneration go with it; the ones that
>   pin "nothing refills while spending" and "never above max" stay and become the station rules.
> - **`gas_tank: 300.0` STAYS.** He asked for that separately (*"gas tank sollte sehr viel mehr
>   haben"*) and it is the whole answer to *"der boost hält nicht lang genug"* — 16.67 s per tank.
>
> **What replaces it is a new feature, not a value:** refuel stations as world objects, and a
> mission loop where returning to the main building is a decision. That is `F-018`'s real shape and
> it does not exist yet. Queued in [`NEXT.md`](../NEXT.md).
>
> **Why he is right and I was not:** the bible couples gas to *risk*, not to a timer — burning gas
> is loud and a Bellower answers it (bible line 159). A tank that quietly refills itself while you
> hang around is exactly the timer the bible refuses. A tank you have to go somewhere to fill makes
> the whole map a resource decision.

## Q-037 — On the last drop of gas: does `Shift` win, or does `W` on the rope? (2026-08-13, W4)

The mixing rule (`docs/NEXT.md` §1B) gave the rope its own thrust, and every one of the nine
judges of that plan said the same thing independently: **it must not be free.** So there is now a
fourth `GasConsumer::Steer` at `vector.gas_steer_per_s: 16.0` — the boost's own price per m/s of
speed bought (16/30 against 18/34, `tests/data.rs` pins the difference at ≤ 0.15).

That creates a question the first three consumers never had. `Boost` and `Steer` are **both
rates, both wanted on the very same tick** of a swing — `Shift` held while `W` is held on a taut
rope is the ordinary case, not an edge one. On a nearly empty tank `vector.gas_priority` decides
which of the two the last drop buys, and the Dodge argument (*"being served last costs a dodge
0.9 % of its own price"*) does not transfer at all: here the two prices are 0.30 and 0.27 per
tick, so whoever stands second gets nothing on the tick the tank runs out.

`Q-017` is the same shape and you answered it by leaving `Boost` first. This is that answer's
next-door neighbour, and it is yours for the same reason: **what runs out first is balancing, not
mechanism.**

**ASSUMPTION:** `gas_priority: [Boost, Steer, ReelIn, Dodge]` — `Steer` is **inserted second**,
after `Boost`. The reasoning, so you can disagree with it precisely: `Shift` is a **deliberate
press** that means *"I need speed NOW"*, while `W`/`A`/`D` on a rope is held almost continuously
during flight. If `Steer` came first, the ambient input would quietly eat the drop the explicit
one asked for — and a `Shift` that does nothing reads as a broken game, where a rope pull that
fades out for one tick reads as an empty tank, which is what it is.

**What has to be rolled back if you decide otherwise:** exactly one line,
`assets/data/game.ron: vector.gas_priority`. Nothing in the code knows the order —
`vector::gas::book` walks the list and `src/vector/gas.rs`'s
`f006_the_file_decides_whether_the_last_drop_boosts_or_steers` drives **both** orders on purpose,
so moving `Steer` in front of `Boost` changes one array and no test has to be rewritten.
`tests/data.rs::t005_gas_priority_names_every_consumer_exactly_once` only counts, it does not
order.

**Why it matters today, and not in a month:** the tank is 300 and there is still no refuel
station outside the hub. 16/s of steering means 18.75 s of held `W` on a rope — a little longer
than the boost's 16.67 s — so on a long traversal the two really do meet at the bottom of the
tank, and the last drop is a decision somebody made. Right now that somebody is me.

**Also still yours, and cheaper:** `gas_steer_per_s: 16.0` itself is ⚠️ UNTUNED. It was solved
out of a ratio, not felt in a swing. If flying ends up feeling stingy, this key is the first
place to look — and it moves without touching a line of Rust.

---

## Q-031 — RESOLVED as option (2) on 2026-08-13, on your instruction, indirectly

**Whose authority:** yours, but not as an answer to this entry — you never replied to it. You
asked for **„ein attack system mit gegnern"**, and an attack system in which the enemy's facing
does not exist is option (1) with extra steps. **`ASSUMPTION:` that instruction outranks the
silence on this question**, because option (2) is also what the design bible implies everywhere
it talks about the nape. Say the word and it comes back out; the rollback is below.

**What was built** (`docs/FINDINGS.md` FIND-012 is the hole, FIND-089 is the price):

1. **`titan.ron: <kind>.strike_half_angle_deg: 55.0`**, on all eight kinds, no `serde(default)`.
   Half the arc a blow lands in, off the titan's forward vector, on the ground plane. With
   `attack_range_m` the strike volume is now a **cone**, not a cylinder.
   ⚠️ **UNTUNED and uniform** — 55° is a guess nobody has played, and the per-kind spread is the
   obvious next pass (a bellower swinging a tree against a warden shielding his own neck).
2. **A titan turns while winding up.** The turn in `titan::brain::walk` no longer hangs off the
   *walk's* gate (`distance_m > attack_range_m`), so a titan inside his own reach rotates at his
   `turn_deg_per_s`. `Strike` and `Recover` stay locked — the blow is committed, and `recover_s`
   is the punish window.

**The numbers, which is the whole point of the change:**

- rear-arc damage **34 → 0**, frontal **34** — same husk, same 5 m, same height, only the side
  differs (`tests/combat.rs::a_strike_from_behind_books_no_damage`).
- a husk in `Windup` inside `attack_range_m` turns **29.167° over 35 ticks = exactly 50 °/s**,
  where he turned **0.000°** yesterday (`tests/titan.rs::q031_a_titan_turns_while_winding_up`).
  **That is `Q-029`'s knob becoming a knob.**

**What this costs, measured, not assumed:** the warden's nape pass tightens from 0.20 m to
0.15 m of air; the husk's is unchanged; `scripts/game-full.txt` still wins 23/23 but at tick
**899** instead of 898. Full table in FIND-089.

**What has to be rolled back if you pick (1) after all:** two places and one file.
`src/combat/strike.rs::faces` (delete it and the call in `reaches`), the `turning` gate in
`src/titan/brain.rs::walk` (back to the one `pursuing` line), and the eight
`strike_half_angle_deg` keys — and then, per this entry's own original wording, **delete
`turn_deg_per_s` and `attack_range_m` rather than leaving them as decoration.** The tests that
would have to go with it are named above plus
`tests/titan.rs::q031_the_nape_survives_a_titan_who_tracks_you`; the four Q-030 geometry passes
would drop their `Tracking::Off` switch.

**Still yours, and now worth asking:** `strike_half_angle_deg` is the second untuned number in
this fight after `turn_deg_per_s`, and the two are read together — a wide cone forgives a slow
turn. **55° and 50 °/s means a husk covers 30° of a 90° error during his wind-up**, so circling
him works and standing still does not. If that reads as too easy, the cone narrows before the
turn speeds up.

---

## Q-038 — the aim assist is a machine setting, and multiplayer has no machine  ✅ DECIDED (2)

**Raised 2026-08-19 by the round that built `F-024`/`F-025`.** Carried here by the main head
because `docs/QUESTIONS.md` was not that agent's file — the detail and the rollback point are in
`docs/FINDINGS.md` **FIND-104**.

### The situation

`F-016`'s two knobs live in `PlayerSettings`, which is a **`Resource`** — one per running program,
seeded at `Startup` out of `game.ron`. That is right for mouse sensitivity and FOV: they belong to
the human at the keyboard, not to the character.

**But the assist changes where the rope goes.** So the snap only applies to `With<LocalPlayer>`,
and a remote player's arms resolve as if his assist were 0 %. Today that is invisible — there is
one player and `net::local` is the only transport. **On the day the netcode lands it is a
divergence**, and rule 4 is explicit that nothing gets built now which makes multiplayer expensive
later: *"player state never as a `Resource`, input is an `Intent`."*

### The three answers, and they are genuinely different games

1. **The assist is part of the Intent.** The resolved point (or the two knob values) rides on the
   wire, every client simulates every player identically, and a replay reproduces the shot.
   Costs wire bytes on every tick and makes the setting **cheatable** — a client that claims
   100 % gets 100 %.
2. **The assist is presentation only, and the server re-resolves.** The knobs stay local, the
   *fired direction* goes on the wire, and the assist is a way of choosing that direction rather
   than a thing the simulation knows about. Cheat-resistant, and it means a replay of somebody
   else's shot never shows his assist — which is fine, because it never showed his mouse either.
3. **The assist is a character stat, not a machine setting.** It moves out of `PlayerSettings`
   into the save profile and becomes something the trait tree can touch. That is a *design*
   decision with a balance consequence and it would make `F-016` a progression knob rather than a
   comfort one.

**ASSUMPTION the work continues under: (2).** It is the only one that is both cheat-resistant and
free today, it matches how `F-016` is worded (*a comfort setting*, `docs/gameplay/world.md`:
*"Snap wird zur Komfortoption statt zur Notwendigkeit"*), and it needs no wire change now.

**Rollback point:** `PlayerSettings::assist_catch_pct` / `assist_strength_pct` and the
`With<LocalPlayer>` filter in `vector::aim`. If the answer is (1), those two values move onto
`Intent` and the filter disappears; if (3), they move into the save profile.
**Nothing else depends on the choice yet** — the scoring function itself is pure and takes the two
numbers as arguments, so it survives all three answers unchanged.

### What would settle it

The user playing with the knobs and telling us the number he likes. If a *single* number turns out
to be right for everybody, (2) becomes trivially correct and the question dies. If he wants it
different per situation, (1) and (3) get interesting.

### ✅ DECIDED 2026-08-19: **(2), and the evidence is that it costs nothing.**

Decided by the round that built the socket transport, **not by the user** — under the autonomous
rule (`CLAUDE.md`: a decision that belongs to the user gets made anyway, visibly, with a rollback
point). What changed is that the question is no longer hypothetical: there is a wire now
(`src/net/wire.rs`), a peer really drives a body in this world (`tests/multiplayer.rs::
net_a_peer_on_a_real_socket_drives_his_own_body`), and the three answers can be priced instead of
argued.

**(2) is already what the code does, and it needed no byte.** A frame carries `yaw` and `pitch` —
*the direction that was fired*. Under (2) the assist is a way of **choosing** that direction on
the sender's machine, so the receiver takes it as given and re-resolves the hook from it. That is
exactly what `vector::aim`'s `Has<LocalPlayer>` filter already does: this machine assists this
machine's aim, and every other body's arms follow the angle that arrived. **Zero wire change, zero
divergence** — the thing the question feared (*"a remote player's arms would resolve as if his
assist were 0 %"*) is not a divergence at all once the direction is understood as the payload,
because his assist has already been spent by the time the angle is put in the packet.

**What (1) would cost, now that the frame is a measured object:** the two knobs are 8 bytes on a
**37-byte** frame — **+21.6 %** on every packet of every player of every tick, i.e. 44 kB/s → 53
kB/s at the bible's twenty players — bought in exchange for a value the sender can lie about
anyway. A client that wants 100 % assist under (1) sets the field; under (2) he can already aim
wherever he likes, so (1) buys no honesty and costs a fifth of the bandwidth.

**(3) is not refused, it is out of order.** Moving the assist into the save profile makes `F-016` a
progression knob, and the design's hard rule is that no meta system starts before the Vector Gear
gate is passed (`CLAUDE.md`, `docs/gameplay/pillars.md`). If the trait tree ever wants it, it takes
it from (2) without anything in `net/` changing.

**Rollback point is unchanged and still cheap:** `PlayerSettings::assist_catch_pct` /
`assist_strength_pct` and the `Has<LocalPlayer>` filter in `vector::aim`. For (1), those two move
onto `Intent` — which means `net::wire` grows two `f32` and `FRAME_BYTES` becomes 45, and
`wire_a_frame_is_always_the_same_size` is the test that will say so. For (3) they move into the
save profile. **Still nothing else depends on the choice**: the scoring function is pure and takes
the two numbers as arguments.

⚠️ **What is still the user's:** the *number*. Whether the shipped assist is 40 % or 80 % is a feel
question and this decision does not touch it.

---

## Q-039 — ✅ ANSWERED (2026-08-19) — looking straight down: should the fan collapse, or is "within what it asked for" enough?

**Raised 2026-08-19 by the round that fixed `B-008`.** It is a feel question and the user is the
only one who can settle it; the mechanical half is already fixed and measured
(`docs/BUGS.md` B-008, `docs/FINDINGS.md` FIND-121).

### The situation

`F-023` puts the two arms on a fan around the look direction — 11.21° per side at the shipped
wheel over an Ashgate street. That is a *screen* spread, and it is the same 11.21° at every pitch.
Looking level it is exactly right: both rays land on the wall you are looking at.

**Looking down it stops meaning anything.** "Left" and "right" of a crosshair that points at your
own feet are an arbitrary direction, and the world offset the fan buys — `d · sin(11.21°)` — is a
whole roof at 30 m and two roofs at 60 m. Measured from 30 m over the street at
`(168.19, ., -50.12)`, straight down, **the two arms landed on the same two roof caps from every
height**: the pavement the crosshair stood on was unhookable, and nothing said so.

`B-008` fixed the gross half: a side hit further off than `vector.aim_side_coherence_k` × what the
fan asked for, on a different body, is refused and the arm falls back to the centre. From 30 m the
two roofs read 1.84× and 1.96× and are now refused — both arms take the pavement.

**But from 60 m they read 1.21× and 1.25×**, and that is *inside* what the fan asked for at 60 m
(11.7 m per side). So from high enough over a street, looking straight down still hooks the roofs
beside you. The player's marker shows it, so it is not silent any more — but it is still not
aimable.

### The three answers

1. **Leave it.** From 60 m the roofs really are what the fan drew, the markers say so, and a
   player who wants the street can look at it from lower down or narrow the wheel. Cheapest, and
   it keeps the one thing the spread is good for while diving: catching a roof beside you without
   looking at it.
2. **Collapse the fan with pitch.** `effective_spread_rad` gets a sixth input and multiplies the
   resolved angle by something like `cos(pitch)` past a threshold, so at −90° all three rays are
   one. Predictable at every height, and it makes "look at it, hook it" true straight down. It
   costs the diving player the sideways catch, and it is a second knob on a model that already has
   eleven.
3. **Tighten `aim_side_coherence_k`.** 1.5 → 1.2 refuses the 60 m case too. One number, no new
   code — but 1.2 is close to what a wall at 45° of grazing legitimately produces (1.27), so it
   would start refusing coherent hits on real facades. **Measured, this is the wrong screw.**

**ASSUMPTION the work continues under: (1).** `B-008`'s complaint was *"lands somewhere else in
silence"*, and the silence and the gross case are both gone — the residue is a 1.2× overshoot that
the HUD marker shows. (2) is a *feel* change to verified work (`FIND-096`), and rule 5's habit says
do not change a model nobody has complained about the new behaviour of yet.

**Rollback point if he wants (2):** `src/vector/aim.rs::effective_spread_rad` — one term at the end
of the function and one key in `game.ron: vector.aim_sep_*`, plus a case in
`tests/vector_aiming.rs::f023_the_side_ray_sits_at_half_the_wheel_at_every_pitch`, which asserts
today that the angle is pitch-independent and **would have to be rewritten**. Nothing else reads
the pitch. If he wants (3) instead it is one number in `game.ron` and no code at all.

### What would settle it

Him flying over Ashgate, looking down at a street from 40–80 m, and saying whether the two arms
grabbing the roofs beside him reads as *"the game helped me"* or as *"the game ignored me"*.

### ✅ ANSWERED, 2026-08-19 — **(1) stands, (2) is ruled out, and nothing rolls back**

> *„ok von snapping. **die seile sollen immer auf der horzontalen fest sein.** also wenn das
> fadenkreuz 0, 0 ist sollen die seile nur auf der x achse snappen (objekte finden) **also
> seitlich!** dann ist es auch besser einzuschätzen."* — the user, 2026-08-19

He was writing about the **snap**, and the sentence answers the **fan** as well, because option (2)
contradicts it in so many words. Collapsing the fan with pitch means that at −90° all three rays
are one — the ropes stop being sideways exactly where he says they should *always* be sideways
(*„immer"*). (3) was already measured as the wrong screw (1.2 refuses coherent hits on real
facades at 45° of grazing). **So (1) is not just the assumption the work ran under any more, it is
the answer**, and the residue it leaves — the two roofs beside the street from 60 m, at 1.21× and
1.25× of what the fan asked for — is the sideways catch he asked to keep, shown by the marker.

**Nothing has to be rolled back**: the work already ran under (1). `effective_spread_rad` keeps no
pitch term and `f023_the_side_ray_sits_at_half_the_wheel_at_every_pitch` stays as it is.

⚠️ **What this does NOT answer** is `B-008`'s own residue — the pavement under a steeply-aimed
crosshair is still not the thing you hook from 60 m. That is a `aim_side_coherence_k` question, it
is measured (FIND-121), and it is not a fan question. Evidence for the closure:
`docs/FINDINGS.md` FIND-133, `scripts/f024-sideways.txt` leg 6 (30 m up, 60° down, the shot still
lands: `body 1361 at 169.57 1.50 −65.95`).

---

## Q-042 — ✅ DECIDED (2) on 2026-08-20 — the search band needed BOTH assist knobs up before it appeared, and that was a trap on the settings screen

**2026-08-20 · `F-016` · `src/shared/settings.rs::assist_is_on`, `src/hud/catch_band.rs`.**

The band is now on screen while the settings plate is up, which is what the user asked for
(*„damit man das besser einstellen kann"*). Measured while building it: a player who opens
`Settings` on a fresh run and turns **`Aim assist reach`** up sees **nothing happen**, at any
value, because the gate is `assist_catch_pct > 0 && assist_strength_pct > 0` and
`assist_strength_pct` ships at 0. He has to guess that a second, differently-named row is the
master switch. The first row he touches is the one that looks like it should draw the picture.

The gate itself is right and is not the question: it is the identical predicate `vector::aim`
filters on, so *no probe cast* and *no band* stay one decision (FIND-135) — a band that drew
while nothing was searching would be the lie the whole element exists against.

**The question is which of these the user wants**, and it is a design call, not a bug:
1. leave it — the two rows are two axes and the plate says so in its hint lines;
2. let the reach row light the band on its own, drawn differently (dimmed / dashed) to say
   *"this is what the search WOULD cover; the assist is off"*;
3. make `Aim assist strength` come up off 0 the first time the reach is raised.

**DECIDED 2026-08-20 — (2), and it is nearer (1) than it looks: the band is drawn from the
REACH alone, and the colour says whether anything is searching.**

The first assumption on this entry was *(1), left as it is*, and it was wrong for a reason the
entry itself contains: `F-025` is not built, so `assist_strength` **does nothing today except
open this gate**. Leaving it meant the picture the user asked for — *„damit man das besser
einstellen kann"* — could not be reached at all by a player who had not guessed that a second
row is a master switch. The strongest evidence against (1) was already in the tree:
`tests/menu.rs::nudge_reach` had to press the strength button *in secret* before it could see a
band, which is the trap written down as code.

What changed, in one predicate: `hud::catch_band::place_catch_band` draws from the new
`PlayerSettings::assist_has_reach()` (`catch > 0`) instead of `assist_is_on()`. The geometry is
**bit-identical** in both states; the one claim that differs — *is a ray being cast right now* —
is carried by the colour, `hud::catch_band::IDLE` (white at 0.40) against `NEUTRAL` (0.75).
Measured: **5.43:1 searching, 3.36:1 idle** over the settings backdrop on the worst world frame,
both clear of WCAG 1.4.11's 3:1, and the two states 1.61:1 apart — a knowing trade, because
pushing them 3:1 apart makes the idle state illegible and the idle state is the **only** state a
player can be in today.

⚠️ **The PROBE is untouched and still `assist_is_on`'s.** Drawing and searching are two
predicates on purpose; `docs/FINDINGS.md` FIND-137 argues why that is not the FIND-098 /
FIND-099 / FIND-127 / FIND-129 defect (in all four the *geometry* lied; here it cannot).
`tests/vector_hooks.rs::f016_at_zero_percent_the_aim_is_bit_for_bit_the_one_the_game_had_before`
was re-run green after the change.

**Rollback point if he wants (1) back:** one line —
`src/hud/catch_band.rs::place_catch_band`'s `.filter(|s| s.assist_has_reach())` goes back to
`.filter(|s| s.assist_is_on())`, and `PlayerSettings::assist_has_reach` and
`hud::catch_band::IDLE` become dead. The two tests that would then have to change are
`tests/hud.rs::f016_there_is_no_band_when_there_is_no_reach` and
`::f016_the_reach_alone_draws_the_band_and_the_colour_says_whether_it_searches`; the break was
watched red on exactly that one-line edit.

**Still open, and it is the picture and not the code:** whether a *dimmed* band reads as "this
is what the reach covers, nothing is running" or just as "a faint band" to somebody who has
never seen the bright one. Side by side the two are clearly different
(`docs/images/f016-band-100.png` against `docs/images/f016-band-idle-100.png`); alone, the idle
one is only a fainter ruler. If he says it reads as broken rather than as idle, the cheap answer
is a word on the `Aim assist reach` row's hint line — `src/menu/settings.rs`, which this round
did not own.

---

## Q-048 — both ropes to the crosshair, or the left/right split you asked for? ✅ ANSWERED

**Opened:** 2026-08-23, while the user played the reference on the Windows machine (`FIND-149`).
**Answered:** 2026-08-23, by him, in one sentence:

> *„dann das auseinander mit q und e kann weg. einfach da wo ich hinschau (also fadenkreuz) geht
> das seil hin."*

**`F-023` is retired. Both ropes fly at the point under the crosshair.** This overrides his own
2026-08-12 instruction that built the fan (*„es muss mehr rechts und links spreaden!!"*), and the
standing rule says it may — `CLAUDE.md`: *his instruction beats my derivation, and beats his own
earlier number.* The fan was **deleted, not flagged**: a dead branch nobody selects is worse than
a deletion, and this repository already refuses registry rows nothing can spawn.

### What changed, in one line each

- `vector::aim` casts **one** ray and writes one `AimPoint` into both halves of `ArmAim`.
- `Q` and `E` are unchanged as keys, arms and state machine — two ropes on **one** anchor.
- **16 `game.ron` keys, 3 fields, 12 functions and 31 tests** went with it; the wire is 4 bytes
  shorter (`FRAME_BYTES` 37 → 33). The full list is `docs/FINDINGS.md` **FIND-154**.
- **The assist stays.** It searches sideways along the crosshair's own screen row (`FIND-133`)
  and now publishes one winner instead of one per hemisphere. *"Sideways from the crosshair"* was
  never *"the two arms apart"*.
- **`FIND-129`'s promise stays**: a drawn pixel is the point its rope flies to, and the guard
  that proves it got a **stronger** assertion, not a looser one (the drawn x is the projected x
  in all 752 samples, worst 0.45 px).

### The assumption that was running under this question, and what it cost

**`ASSUMPTION:` "`F-023` stays as built. Nothing is changed on the strength of one observation."**
It stood for a few hours and cost nothing: no work was built on top of the fan in that window.
The rollback point this entry named — *"one file, one function: `vector::aim::aim`"* — turned out
to be **wrong by an order of magnitude**, and that is the lesson worth keeping. The aim itself was
one function; the wheel, the settings row, the `Intent` field, the wire slot, the metre model's
seven keys, `B-008`'s coherence guard and `hud::arm_aim::Bearing` were not. **A rollback point
written from the feature's own file underestimates a feature that has a player-facing control.**

### If he changes his mind again

`git show 83f09da` is the last commit with the fan in it — the model, the keys, the wheel and all
31 tests. It is a revert, not a re-derivation.

Related: `FIND-154` · `FIND-149` · `FIND-096` · `FIND-133` · `docs/NEXT.md` item 1 ·
`scripts/q048-one-point.txt`

---

## Q-049 — Pendulum or Drive? The switch is built, both work, and only you can answer it

**Opened:** 2026-08-23, out of `FIND-149` / `FIND-152`.
**Status:** ✅ **ANSWERED 2026-08-23 — `Drive`.** He asked for it twice. The switch stays, so a
session can still A/B; the numbers underneath it are the open part now.

`game.ron: vector.rope_force_model` now picks between two rope physics, and both are live:

- **`Pendulum`** — the game as it stood on 2026-08-22, bit for bit. An avian `DistanceJoint`
  plus gravity, `air_pull_m_s2` steering it. Letting go of every key still carries you.
- **`Drive`** — the reference's model, out of his own sentence: *„wenn ich nichts drucke dann
  wird auch nicht rangezogen!"*. No joint at all. The rope is a **direction**, the key is the
  **force**, and the velocity ramps toward `drive_speed_m_s` with `drive_ramp_s`.

**To feel it — one line, no rebuild of anything but the binary:**

```bash
sed -i 's/rope_force_model: Pendulum/rope_force_model: Drive/' assets/data/game.ron
cargo play                        # and the same sed the other way round to go back
```

### ✅ ANSWERED, 2026-08-23 — **`Drive` is the shipped default now, on his word**

The assumption written here on the morning of 2026-08-23 was *"the default stays `Pendulum` until
he says otherwise"*. **He said otherwise the same day**, after playing the drive:

> *„wenn ich mich hooke und w drücke oder generell booste dann soll ich erstmal ziemlich direkt
> daran gezogen werden. also ziemlich gerade. außer ich move nach links (a oder rechts d). **es
> darf „strenger" sein. also nicht so physics accurate aber mehr haptisch. also man macht was und
> man merkt es auch direkt!**"*

That is a request for **more** drive, not less — the second time he has described this model as
the one he wants (`FIND-149` was the first). `game.ron: vector.rope_force_model: Drive`, and
`FIND-153` is what was built on top of it.

**The old assumption's real content survives and it cost 19 tests to learn:** every measured
number in this repository is a statement about ONE of the two models, and until 2026-08-23 the
tests read whichever way `game.ron` happened to be set. Flipping the default took **13 of
`tests/vector_rope.rs`, 5 of `tests/combat.rs` and 1 of `tests/player.rs`** red at once without a
line of their subject having changed. They now **pin** `RopeForceModel::Pendulum` in their app
builder. → `FIND-153`.

**Rollback point:** still the one line, in both directions:

```bash
sed -i 's/rope_force_model: Drive/rope_force_model: Pendulum/' assets/data/game.ron
```

### The three numbers that are his to judge, and what each does

| key | 2026-08-23 a.m. | **today** | what it is | what to say if it is wrong |
|---|---|---|---|---|
| `drive_speed_m_s` | 50.0 | **70.0** | the speed `W` chases along the rope | *„zu langsam/zu schnell"* |
| `drive_ramp_s` | 0.25 | **0.08** | the onset — 63 % of the gap in this long, and the whole of *„gerade"* | *„zu träge"* / *„zu abrupt"* |
| `drive_lateral_m_s` | 18.0 | **30.0** | what `A`/`D` chase across the rope | *„ich kann nicht genug lenken"* |

🔴 **The ceiling on the first one is not that key.** `vector.max_speed_m_s` (75) is an avian
`MaxLinearSpeed` on the body itself, so any `drive_speed_m_s` above it is a number the solver
silently clips. **Raising the drive past ~70 means raising `max_speed_m_s` first** — and that key
was not this round's to touch.

**Measured, both models, one pinned binary (`scripts/f006-drive.txt`):** hooked with no key held,
both fall at exactly 21.33 m/s — neither rope pulls a player who holds nothing, because the
pendulum's rope is nearly slack in that geometry. 1.5 s of `W`: **52.94 m/s** under `Drive`
(capped) against **59.66 m/s** under `Pendulum` (still climbing toward the 75 m/s clamp). 100 m
from rest with `W` held: **2.15 s** against **2.27 s**.

Related: `FIND-149` · `FIND-152` · `docs/NEXT.md` item 1 · `Q-050` · `scripts/f006-drive.txt`

---

## Q-050 — under `Drive` the reel does nothing and `A`/`D` alone is a brake. Both are consequences, not decisions

**Opened:** 2026-08-23, with `Q-049`.
**Status:** 🟢 **closed 2026-08-25** — the `A`/`D` brake was fixed on 2026-08-23 (`FIND-153`);
the reel got a verb of its own on 2026-08-25 (`FIND-159`). **One thing in it is still yours**,
and it is section 1c below.

**1. ✅ ANSWERED 2026-08-25 — `Ctrl` under the drive is a WINCH.**

The problem, first: `Drive` builds no `DistanceJoint`, so there was no enforced length, so
`player::rope::shorten_ropes` never saw the rope — **and `vector::gas` billed `gas_reel_per_s: 6`
for the held key anyway.** It also broke the flagship run: `scripts/game-full.txt` climbs a 35 m
church roof by reeling, and under `Drive` it reported **4 of 24 asserts failed** —
`Speed 0.000`, `Height 0.300`.

**a. What it does now.** `player::locomotion::rope_winch`:
`a = r̂ · max(0, reel_speed_m_s − v·r̂) / drive_ramp_s`, over the arms further out than
`min_rope_m`. **Closing speed along the rope, and nothing else** — no look gate, one axis, and
it can never brake, because the coefficient is a `max(0, …)`.

**b. Why this and not the two answers this entry proposed on 2026-08-23.** Both of them were
*„fold it into `W`"* in different words, and `W` cannot do the job `F-005`'s own acceptance
sentence asks for: *„Spieler kann aus dem Tiefpunkt Hoehe gewinnen"*. At the low point of a
flight the anchor is behind and above you and you are looking where you are going, and the
drive's look gate `cᵢ = max(0, l̂ · r̂ᵢ)` is **exactly zero** there, by construction. So the two
verbs are one trade the player can feel:

| | `W` — the drive | `Ctrl` — the winch |
|---|---|---|
| speed | `drive_speed_m_s` **70** | `reel_speed_m_s` **28** |
| aim | look-gated: you go where you look | none: straight up the rope |
| axes | the whole velocity | the rope axis only — your swing survives it |
| ends | when `r̂` swings past your look | at `min_rope_m` |

**c. 🔴 THE PART THAT IS YOURS: is 28 m/s the right winch, and is a slower second gear what you
want at all?** `ASSUMPTION:` yes, and `reel_speed_m_s` is **not re-tuned** for the drive — the
same 28 that the pendulum's length used. On the one act that has ever been photographed the two
readings land within 8 % of each other (28.741 m/s against **26.695 m/s**, both onto the same
35.000 m roof), so a second key would be a second thing to get wrong.
**Rollback point:** one number, `assets/data/game.ron: vector.reel_speed_m_s`. Raising it is
free up to `max_speed_m_s` (75). Deleting the verb instead is the `RopeForceModel::Drive` arm of
the winch `match` in `src/player/locomotion.rs::air_control` plus the `Ctrl` binding in
`src/net/local.rs` — and then `scripts/game-full.txt` ACT 1 has to be re-flown, because nothing
else in this game lifts a standing player onto a roof.

**d. And the billing is honest again**, under both models: `vector::gas` stops charging
`gas_reel_per_s` the moment the arm is inside `min_rope_m`, which is where both mechanisms stop
moving anybody. Red test: `tests/vector_gas.rs::f005_a_reel_whose_rope_is_already_at_the_floor_is_not_billed`.

**e. ⚠️ One shape is measured and NOT designed.** Hold `Ctrl` through an anchor in open air — a
hook that bit a lamp post rather than a wall — and you shoot past it, `r̂` flips, and the winch
hauls you back. It is bounded by `reel_speed_m_s` and it is no worse than what the pendulum does
at `min_rope_m` (`FIND-035`: 17 m/s out of the player in one tick), but nobody chose it. Anchors
sit on surfaces, which is why it is hard to reach in practice. If you feel it, say so and it
gets a latch.

**f. ⚠️ And one reported symptom did not reproduce.** *„a running player loses ground contact on
single ticks, so `F-009`'s flip fires on the ground"* — four probe runs on flat `ashgate`
(standing, running at 6 m/s, skidding at 39 m/s, mid-jump), `A`·pause·`A` at four tap spacings,
never once billed `gas_flip`. What **was** measured and closed is the other half of the same
line: the old predicate said yes to `Downed` and to `OnWall`. The flicker stays open here.

**2. ✅ FIXED 2026-08-23 (`FIND-153`) — `A`/`D` with `W` released brakes you.** The drive chased
a target velocity; with only a lateral key held that target was `drive_lateral_m_s` (18 m/s) and
**nothing else**, so a player at 52.9 m/s who tapped `D` alone was pulled down to 20.9 m/s in a
second (measured, `scripts/f006-drive.txt` ACT 3).

His own instruction settled it — *„außer ich move nach links (a oder rechts d)"* asks the key to
take him **off the anchor line**, not to end the flight. So the released-`W` target now **keeps
the player's own velocity on every axis it does not command** and replaces only `ê_right`.
Measured over the same second: **70.0 → 70.0 m/s, turned 23°** instead of 70.0 → 30.0.
Red test: `tests/player.rs::f153_a_and_d_alone_steer_the_flight_instead_of_braking_it`.

⚠️ **The first fix for it was wrong in the other direction and the measurement caught it.** A
lateral that merely *adds* to the flight took `D`-alone to **75.000 m/s exactly** — that is
`vector.max_speed_m_s`, the avian clamp, and not a speed anybody chose either. The drive's own
`clamp_length_max(drive_speed_m_s)` now sits outside the blend, so `A`/`D` is a **redirect**:
same speed, ~20° of it pointing somewhere else. Measured 71.12 m/s in ACT 3.

Related: `Q-049` · `FIND-152` · `FIND-153` · `FIND-159` · `F-005` ·
`src/player/locomotion.rs::rope_winch` · `src/vector/gas.rs`

---

## Q-052 — five movement verbs landed. Four numbers in them are yours, and one of them replaces the gas price as the thing that limits a dash (2026-08-24)

`F-008` `F-009` `F-010` `F-017` `F-019` are built (🟨/🟧, see the commit). Every one of them
runs under an `ASSUMPTION:` because you were not here, and each says what to roll back.

### 1. What bounds a dash is no longer the gas price

You already know the arithmetic (`Q-046`): the tank went `300 -> 15000` for testability, so
`gas_dodge: 45` went from **6.7 dashes per sortie to 333**, and the cooldown the backlog row
asks for did not exist. A dash was a traversal move.

**ASSUMPTION:** a dash is now a **magazine** — `game.ron: vector.dodge_charges: 3.0`,
`dodge_recharge_s: 4.0` (12 s for a full reload), `dodge_cooldown_s: 0.6` (twice the
double-tap window, so drumming on Space cannot fire two). The gas price rides on top,
unchanged, and still makes the dash the expensive impulse next to the boost.
**Roll back:** delete the three keys from `game.ron: vector`, the three fields from
`data::VectorTuning`, and `DodgeCharges` from `src/shared/gear.rs` /
`src/vector/dodge.rs::spend_and_recharge`; `vector::gas` then falls back to the old line
because it asks `charges.is_none_or(...)`.
**The question is the SHAPE, not the numbers:** is a magazine what you want, or is a plain
cooldown enough? Three-in-a-row is a burst you can spend badly; a single 0.6 s cooldown is not.

🔴 **AMENDED 2026-08-25 — a dash is an AIR move now, and it was not one when this was written.**
`vector::gas` never asked what the player was standing on, so one press of `C` while running
bought `F-010`'s slide **and** the dash: gas 15000 -> 14955, a charge gone, and the slide's
promised `max(current, 12)` delivered **38.166 m/s** (12 + `dodge_impulse_m_s` 24). On the
ground `C` is the slide, in the air it is the dash — one evasion per state, which is the rule
the flip already obeyed. `docs/FINDINGS.md` FIND-159.
⚠️ **The consequence you may not want:** a grounded `C` under `player.slide_min_speed_m_s`
(3 m/s) now answers with **nothing at all**, where it used to answer with a dash. `F-028`'s rule
says no press without an answer, so `start_slides` logs which of the two refusals it was — but
the HUD hint that belongs next to it lives in `src/hud/arm_aim.rs` and was not this job's file.
**Rollback:** delete `&& in_the_air` from `wants_dodge` in `src/vector/gas.rs`.

### 2. A station is three VISITS, not three seconds of standing

`F-019` (below) gives each field station `gear.ron: resupply.station_uses: 3`. The first build
let a player who simply stood on one drain all three in 4.5 s without pressing anything — a
station that empties itself.
**ASSUMPTION:** one reload **per visit**. The latch closes when a pump starts and only opens
when the circle is empty, so three uses are three arrivals.
**Roll back:** `SupplyStation::served_this_visit` in `src/shared/gear.rs` and the two lines that
read it in `src/world/supply.rs::run_the_pumps`.

### 3. Where the four stations stand

Four, on `ashgate`, at `(0,2,20)`, `(0,2,105)`, `(0,2,190)` and `(-30,2,-60)` —
**on the gantry line**, i.e. on the swing spine itself, at a pitch of ~85 m so the next one is
always inside one hook range (90 m).
**ASSUMPTION:** supply belongs on the route, not beside it — a station off the lane is a station
nobody visits.
**Roll back:** `assets/data/maps.ron: ashgate.supply_stations` — it is a list of coordinates and
nothing reads it but `world::supply::build_stations`.
⚠️ **`graybox` deliberately has none** (`supply_stations: []`): it is the fixture a dozen tests
reason about at `y = 0`.

### 4. `F-009` flip is last in `gas_priority`, and that is the one position worth arguing about

`gas_priority` is now `[Boost, Steer, ReelIn, Dodge, Flip]`. A flip costs 20 flat while the three
rates together cost 0.4 per tick, so its place can cost it at most 2 % of its own price — but of
the five it is the one that **keeps you alive**, and that argues for putting it first.
**ASSUMPTION:** appended, because moving `Boost` off the front would overturn your own answer to
`Q-017` as a side effect of adding a verb.
**Roll back:** one word in `assets/data/game.ron: vector.gas_priority`.

### 5. Two things `F-017` and `F-019` are missing, and both are somebody else's file

- **`F-017`'s off switch has no UI.** `PlayerSettings::speed_fov_pct` exists, seeds at 100, and
  `0` is bit-for-bit the game that shipped before the curve — but the row that would move it
  lives in `src/menu/settings.rs`, which was another agent's while this landed. The backlog
  sentence *„abschaltbar fuer Motion Sickness"* is therefore **half done**: the mechanism is
  there, the slider is not.
- **`F-019`'s counter is not on the HUD.** *„Zaehler sichtbar"* is answered today by the colour
  (cyan = has reloads, amber = pumping, `ash_dark` = spent) and by a log line, not by a number.
  `src/hud/mod.rs` was another agent's. **The patch is: one text node reading
  `SupplyStation::uses_left` of the nearest station inside `radius_m`.**

Related: `Q-033` · `Q-044` · `Q-046` · `Q-017` · `docs/FINDINGS.md` FIND-158 · `F-008` `F-009`
`F-010` `F-017` `F-019`

---

## Q-055 — ⛔ SUPERSEDED BY `Q-056`, and the assumption in it was never implemented

> **Do not act on this entry.** It was written before the work landed and it describes a design
> that was then not built: it assumed the `in_the_air` gate would be **deleted** so the pull
> reaches the ground. That gate is **untouched**. What was built instead is a *forbid*, not a
> *haul* — `ground_desired_on_a_rope` removes the outbound half of the walk — and its assumption
> and rollback point live in **`Q-056`**. Kept for the record because it is what the reasoning
> looked like before the measurement, not because any part of it is current.
>
> ⚠️ And `Q-056` was itself refuted on the day it was written: it argued the winch out of the
> ground because it *"would have LIFTED A HOOKED PLAYER OFF THE FLOOR"*, and then a 250 m/s²
> taut-brake was put on the ground instead, which lifts him at 11-16 m/s². **A hazard you
> correctly name and then re-introduce through a different term is not a hazard you avoided.**

### the original question, superseded

**Asked 2026-08-26**, out of his *„wenn ich von seil weg gehe. also seil ist vorne und ich **laufe**
zurück werde cih nicht ran gezogen"* (`docs/NEXT.md` §3D, R1).

**The conflict is between two things he asked for, not between his wish and my taste.**

`src/player/locomotion.rs:~1124` gates the always-on pull behind `in_the_air`:

```rust
RopeForceModel::Drive if anchored > 0 && (grant.reel_in || in_the_air) => { ... }
```

and the comment says why: *"a hooked player standing on the ground keeps his legs, and `Ctrl` is
how he leaves it"*. **His word is „laufe" — walking, i.e. on the ground** — so R1 asks for exactly
the gate that was put there on purpose.

**Why it is not obviously safe to just delete the gate:**

- **the hub is a walkable place** (`f072_the_hub_is_a_place_and_not_a_screen`). A pull that reaches
  the ground means a player who fires a hook in the hub is dragged while trying to walk.
- **ground locomotion owns the velocity there** — `FIND-037`, *"the legs cannot produce more than
  the ground's top speed"*, and `f004_the_ground_does_not_write_the_velocity_of_a_player_the_rope_drags`
  is the test that pins the handover. A second writer on the ground is the shape that test exists
  to forbid.

**ASSUMPTION the work continues under:** the pull reaches the ground, **but the legs win** — the
ground keeps authority over the velocity and the pull becomes a lean, not a drag. Concretely: the
always-on pull applies on the ground **only while the player is not producing ground movement of
his own**, so walking backwards is *resisted* rather than *overridden*, and standing still slides
you in.

**Rollback point:** the `in_the_air` gate in `src/player/locomotion.rs`, one condition. Restoring
it is a one-line revert and takes the ground behaviour back to what it was on 2026-08-26.

**What would settle it in ten seconds of play:** fire a hook at a hub wall and try to walk away. If
that feels broken, the answer is "flight only" and the fix is the revert above. If it feels like a
tether, the assumption was right.

**Related:** `docs/NEXT.md` §3D · `FIND-172` · `FIND-037` · `Q-050` · `F-005` `F-006`

---

## Q-057 — the rope is a ratchet now: how hard may it catch you, and does `min_rope_m` become a leash? (2026-08-26)

**Asked out of** *„wenn das seil shcon eingezogen wurde soll es erstmal nicht länger werden!"*
(`docs/NEXT.md` §3D R4).

**What shipped:** `player::rope::Rope::rest_m` — recorded at the distance the hook bit at,
`min`-ed with the distance actually reached on every tick, floored at `vector.min_rope_m`, and dead
when the rope is (a re-fire gets the full length back). `sync_rope_length` publishes it as
`RopeLength::lengths_m` under `Drive`, where that field used to be *"however far away he happens to
be"*. `locomotion::rope_taut_brake` enforces it: beyond the rest length, the **outbound** half of
the velocity is cancelled, at up to `vector.drive_accel_max_m_s2`.

**Two decisions in it are yours.**

**1. The catch is bounded, and the bound is borrowed.** A velocity-level constraint asks for
`−v·r̂ / Δt`, which at 50 m/s outbound is **3000 m/s²** in one tick — a wall, and the same shape
`FIND-035` measured at the other end of the rope. It is clamped to `drive_accel_max_m_s2` (250)
instead, so 50 m/s of outbound flight is caught in 0.2 s over roughly 5 m of stretch: the rope
gives, then holds.
**ASSUMPTION:** 250 is right because it is already the number called *the player's weight*
(`FIND-172`). **It is a borrowed key, not a chosen one** — see the RON diff in this round's report
for the dedicated `vector.rope_catch_m_s2` that should replace it.
**Rollback:** the `taut` arm of the `match` in `air_control`, six lines.

**2. `min_rope_m` can become a leash, and nobody has played it.** The ratchet floors at
`vector.min_rope_m` = 3 m. A player who arrives within 3 m of an anchor and then flies out is held
on a 3 m rope until he lets go — a tether-ball. Under `Pendulum` that was the shipped behaviour
(the joint's `limits.max` does the same thing), so it is not new; under `Drive` it never existed
until today, and `Drive` is the model that ships.
**ASSUMPTION:** parity with the pendulum is the safe default. **Rollback:** the same six lines, or
raise the floor.

**What would settle both in a minute of play:** reel all the way in on a roof edge, then boost away.
If the catch feels like a wall, (1) is too high; if being stuck at 3 m feels wrong, (2) is the one to
change.

### 🔴 AMENDED 2026-08-26 — *„parity with the pendulum"* was false, and decision 2 has an ASSUMPTION now

**The contested sentence is the one in decision 2:** *"Under `Pendulum` that was the shipped
behaviour (the joint's `limits.max` does the same thing), so it is not new."* **It is not parity.**
Under `Pendulum` the rope only shortens when the player holds `Ctrl`; under `Drive` the always-on
pull shortens it **for free**. Measured 2026-08-26: one `W` at a 51.55 m anchor drove `Rope::rest_m`
to the 3 m floor in **2.5 s**, and it arrives on its own after about **eight idle seconds**. After
that, `W` pointed **away** from the anchor moved the player **0.00 m for 190 consecutive ticks** —
`air_thrust`'s 10 m/s² answered by a 250 m/s² catch the instant the distance passed `rest_m` by one
float. That is not a tight leash, it is an **absolute cage**, and it is reached without the player
asking for it. The user's word for R4 is *„erstmal"*, not *„nie"*.

**ASSUMPTION the work now continues under: the brake RESISTS, it does not FORBID.** A sustained
deliberate drive away pays the rope out, slowly. `rope_taut_brake` cancels the outbound speed **in
excess of `payout_m_s`** and leaves that much.

**"Slowly", as a number: `payout_m_s` = 0.333 m/s, and the sustained payout is 0.500 m/s.**

- **0.333 m/s** is `-gravity_m_s2 / simulation_hz` — the speed one tick of falling adds, i.e. the
  smallest speed this simulation can tell apart from *not moving*. A player must lean on the key for
  **three seconds to buy one metre**; a swing that goes momentarily outbound pays out under 7 cm
  before the ratchet takes it back.
- The **sustained** figure is `payout + a_out·dt`, because the brake and the thrust are two
  accelerations summed into one tick: the brake lands the outbound speed on exactly `payout_m_s` and
  `air_thrust`'s own 0.167 m/s of the same tick sits on top. At `player.air_accel_m_s2` = 10 that is
  **0.500 m/s**; the most any term in this game could add is `vector.boost_m_s2 · dt` = 0.57 m/s
  more. Measured: **1.492 m in three seconds of `W` away from a 3.00 m cage**, against **0.500 m**
  with the payout deleted — and 0.500 m is itself only the one tick of thrust the brake is always
  one tick behind.

**⚠️ DERIVED, and it wants to be a key.** `assets/data/game.ron` is the main head's file, so the
number is expressed out of two values already in it rather than added as a third. **The honest home
for it is `vector.rope_payout_m_s`**, next to `drive_idle_speed_m_s`, with no `serde(default)`. Whoever
owns `game.ron` should add it and delete the derivation; until then the derivation is one line.

**Rollback point:** `src/player/locomotion.rs`, `air_control` — the `let payout_m_s = …` line, and
the `payout_m_s` argument in the `taut` arm. Passing `0.0` there is bit-identical to the behaviour
this entry described, and `tests/player.rs::q057_the_taut_rope_resists_a_drive_away_instead_of_forbidding_it`
asserts both halves so the revert cannot be silent.

**What would settle it in a minute of play:** reel all the way in on a roof edge and then try to
walk out of the rope. If it feels like the rope is *giving*, it is right; if it feels like it is
*slipping*, `payout_m_s` is too high; if it still feels like a wall, it is too low.

**Related:** `Q-056` · `Q-050` · `FIND-182` · `FIND-183` · `FIND-184` · `FIND-181` · `FIND-172` ·
`FIND-152` · `FIND-035` · `docs/NEXT.md` §3D R4 · `F-004` `F-005`

---

## Q-058 — ✅ ANSWERED BY HIM, 2026-08-27: yes. `Drive` gets a real rope.

> *"wenn man a oder d drückt (relativ zum anker (DAS IST WICHTIG), immer alles relativ zum anker
> gesehen) dann soll man zur seite gehen können. **aber NICHT das seil verlängern!!**"*

*„NICHT das seil verlängern"* **is** a hard maximum length, which is the whole of the question
below. He also fixed the frame of reference — **everything is measured from the anchor**, so
`A`/`D` are tangential and not camera-right (`docs/NEXT.md` §3F).
**What must be rolled back if the arc feels wrong when he plays it:** `player::rope::attach_ropes`,
the one branch that gives a `Drive` rope its joint. `rope_force_model: Pendulum` in `game.ron`
remains the other escape hatch (`Q-049`).

### the original question, answered

**Asked 2026-08-26**, after `docs/NEXT.md` §3D was built twice and refuted 2/2 both times
(`FIND-186`). **This is the interface question `CLAUDE.md` says must stand before the fan-out**, and
it is his because it changes how the movement *feels*, not how it is written.

### What was found while answering `FIND-152`

**The ratchet he asked for already exists — under `Pendulum`, tested, with the one-writer rule
already satisfied.** `player::rope::shorten_ropes` is the **single** writer of `limits.max`, and it
already contains both halves (`src/player/rope.rs:420-465`):

> 1. **The reel** takes `ReelSpeed · substep_dt` off the length while the button is held.
> 2. **The take-up** (`B-004`) follows the length down to the distance that really exists — **no
>    rate cap, no button, and never upward.** *"This is what makes the length a ratchet."*

That is *„wenn das seil shcon eingezogen wurde soll es erstmal nicht länger werden"*, verbatim,
already shipped — on the model the game is **not** currently running.

### Why this is the whole of §3D and not just R4

A `limits = (0, L)` joint corrects **only when the distance exceeds `L`**, and avian solves **all**
of them simultaneously. So the acceptance sentence — *"the anchor distance must not increase"* —
becomes the **solver's** job, **per arm, for two arms at once**. That is precisely the case both
attempts got wrong by hand: attempt 1 bounded the sum instead of each, attempt 2 zeroed the whole
command trying to satisfy both. **Neither failure is expressible against a joint.**

### The cost, and it is real

`FIND-149` is his own first-hand report that the reference **drives and does not swing**, and
`Drive` exists because of it. **A hard maximum length puts an arc back in** — you can no longer fly
straight past an anchor and keep going; at `L` you are turned. That is the difference between the
two models, and this would make `Drive` *"a pendulum with a velocity drive on top"* rather than a
third thing.
⚠️ The test that already tells the two apart is
`tests/vector_rope.rs::f149_under_drive_a_hooked_player_who_presses_nothing_is_not_held_up_by_his_rope`
— **2.499 m of fall under `Drive` against `Pendulum`'s 0.000** with the anchor straight overhead. It
stays green **only** if `rest_m` is born at the bite distance and the joint is a *maximum*, never a
fixed length. **It is the gate on attempt 3.**

### ASSUMPTION the work will continue under

**`Drive` gets the joint**, born at the bite distance, `limits = (0, rest_m)`, `shorten_ropes` left
as the sole writer and `rope_winch`'s `Ctrl` folded into it as the reel it already is. §3D's R1-R4
then come from the solver rather than from a hand-rolled projection.

**Rollback point:** `player::rope::attach_ropes` — the one branch that decides whether a `Drive`
rope gets a `DistanceJoint`. Removing it takes the game back to today's jointless `Drive`, and
`rope_force_model: Pendulum` in `game.ron` remains the other escape hatch (`Q-049`).

**What settles it in thirty seconds of play:** fly straight at an anchor, pass it, and keep holding
`W`. **With the joint you get swung around it. Without it you fly on.** If being turned feels like
the reference, the assumption was right; if it feels like a leash, the answer is no and §3D needs a
third design.

**Related:** `FIND-186` · `FIND-152` · `FIND-149` · `B-004` · `B-005` · `Q-049` · `Q-057` ·
`docs/NEXT.md` §3D · `F-004` `F-005` `F-006`

## Q-060 — what should the hub line say when it genuinely cannot tell you which door opens?

**2026-08-27 · `src/hud/hub_prompt.rs` · decided under an ASSUMPTION, `FIND-188`**

Closing `FIND-188` (the line named one door and the walk opened another) left one geometry with no
true answer. A deployment pad whose chord along the walk is **shorter than one simulation step** —
0.1 m, `game.ron: player.run_speed_m_s / simulation_hz` — may or may not be sampled by
`mission::hub::deploy_on_contact`: it is a coin flip decided by where the player's acceleration
happened to put the ticks. Naming that pad promises a sortie that may not start; naming the pad
behind it promises a sortie the graze may steal.

Measured over a ±20 m / 0.5 m / 1° grid of the hub floor: **33 of 918 719 stances that start a
sortie (0.0036 %)**.

**ASSUMPTION:** the line hedges rather than guesses, in three lines with **no metre count**,
because a distance is a claim about where the sortie begins:

```
Deploy: Ashgate Breach / Recruit  or  Ashgate Skirmish / Elite
the walk clips an edge - turn to pick a door
Esc: Mission select
```

The alternatives, both rejected: *name the graze anyway* (it is the shape `FIND-178` and
`FIND-188` are both about — a screen saying something the game does not do), and *name the door
behind it* (same, with the sign flipped). A fourth option nobody has costed is making the pads
overlap-free by construction so the case cannot arise, which is `missions.ron` level design and
not this element's to decide.

**What to roll back if he decides otherwise:** the `Ahead::Edge` arm of
`hud::hub_prompt::hub_prompt_text` and the `grazed` branch of `pick_door` — one enum variant and
one `if`. `pick_door` would go back to a single `Aim::Door`, and the sweep test's `hedged` counter
and its `< 0.1 %` assertion come out with it. Nothing else in the game reads either.

**Why it matters today:** it is the second sentence this game has ever shown a player outside a
fight, and it is the first time the HUD admits it does not know something. Whether that reads as
honest or as broken is a taste call, and it is his.

### ⚠️ 2026-08-27, AMENDED — the hedge is gone, and so is the question it answered

**The `ASSUMPTION` above has been rolled back, and the rollback is bigger than the question.**
`FIND-190`: the rule the hedge belonged to — *name the pad the forward walk crosses first* — is
not a rule that can be written. Its miss distance is three-dimensional, so it is a function of
the player's `y`, and `hub::open_hub` warps every player to `y = 2.0` on arrival; walking bobs him
36 mm, `Space` puts him 1.056 m up, the ray sees through 73 solid props and models a straight walk
that `W`+`D` is not. And from the cold spawn, yaw 115 and yaw 116 render **byte-identical screens**
while one of them deploys and the other walks 60 m into nothing — **2.5 cm apart**.

So the line no longer predicts the walk, and the *"the walk clips an edge"* sentence — the thing
this question asked him to judge — **cannot be produced any more**. `Ahead::Edge`, the `grazed`
branch and `pick_door` are deleted. Nothing needs deciding here; the entry stays as the record of
a wording that existed for one day.

**What replaced it, and the one taste call that is left.** Two sentences:

```
Deploy: Ashgate Skirmish / Recruit          <- the PROMISE: a pad is under his feet, and
on the pad - the sortie is starting            `deploy_on_contact` is starting it this tick
Esc: Mission select

Ashgate Skirmish / Veteran                  <- the POINTER: where a door IS. No verb, no
25 m in front of you - amber pad               "walk onto", no "ahead", no claim at all
Esc: Mission select
```

**Q-060b — is a pointer that names a door the walk does not reach good enough?** Standing at the
hub's landing point at yaw 140 the pointer reads *Ashgate Skirmish / Veteran, 18 m in front of
you*, and holding `W` lands him on **Ashgate Breach** — where the line then says
`Deploy: Ashgate Breach / Recruit` on the tick the sortie starts. Nothing lies: the pointer
promised nothing and the promise named what started. But a player may still feel pointed at the
wrong door.

**ASSUMPTION:** that is acceptable, because the alternative is the thing three rounds proved
impossible — no sentence can carry 2.5 cm of ray geometry, and a screen that guesses is
`FIND-178` again. **What to roll back if he disagrees:** only the pointer's *choice* of pad, which
is the `faced` half of `hud::hub_prompt::aim` (smallest `|bearing|`, then nearer, then name) — swap
it for "the nearest pad" or "the nearest pad within 45°" and nothing else in the file moves. The
promise half may not be touched: it is `deploy_on_contact`'s own test and it is what makes the
element unable to lie.

**And one consequence of the promise being true by construction, so nobody discovers it by
surprise: it is on screen for exactly one frame.** `deploy_on_contact` judges the same position on
the same tick, so the instant the promise can be shown is the instant the sortie starts —
**3946 amber px in the banner band at tick 335 of `scripts/f177-door.txt`, 0 at tick 336.** In
practice a player reads the pointer for the whole walk and sees the promise as a flash. The
alternative — showing it a step early, *"you are about to be on the pad"* — is a prediction again,
and at 0.1 m per tick it is wrong in exactly the cases that matter. **ASSUMPTION:** the flash is
right, and if he wants a lasting confirmation it belongs to the deploy banner
(`MissionPhase::Deploying`), not to this line; **rollback point:** nothing in `hub_prompt.rs`.

### ⚠️ 2026-08-27, AMENDED AGAIN — the promise is gone, Q-060b is ANSWERED by measurement, and one new taste call

**Both `ASSUMPTION`s above have been rolled back, and this time by a measurement and not by a
preference.** Two independent adversaries broke the promise/pointer split on two distinct
defects (`docs/FINDINGS.md` FIND-193):

1. **The promise lied on every homecoming.** `hub::open_hub` sends the player home by writing
   `WarpPlayer` in `StateTransition`; `player::apply_warps` applies it in `FixedUpdate`. Every
   arrival renders at least one `Update` frame from the position the finished sortie left behind,
   and the promise was computed from it — *"Deploy: … the sortie is starting"* with no deployment
   behind it, 5/5, at all six pads. The claim this question's amendment rested on — *"the promise
   is `deploy_on_contact`'s own test and it is what makes the element unable to lie"* — **was
   false.**
2. **`Q-060b` is answered, and the answer is no.** *"Is a pointer that names a door the walk does
   not reach good enough?"* — measured over the yard: the bearing-first ranking named a pad that
   was **not the nearest in 71.5 % of stances**, median 11.7 m farther; 9.3 % of the stances whose
   line said *"in front of you"* open a **different** door and 47.7 % open **nothing** in 60 m;
   and it points through walls. The rollback this question itself named — *"swap the `faced` half
   for the nearest pad"* — is exactly what was done.

**What is on screen now, and it is one sentence:**

```
Ashgate Breach / Recruit
nearest amber pad: 13 m to your left
Esc: Mission select
```

It names the **nearest known pad** — the door `deploy_on_contact` would fire if the player stood
there — how far away it is in 3D, and in which direction. It contains no verb about walking
(`hud::hub_prompt::INSTRUCTION_WORDS`), and it renders **nothing at all** until the first fixed
step of a visit to the hub has actually put the player where the hub wants him.

**Q-060c — the line no longer says what makes a sortie start, and nobody has decided whether it
should.** A first-time player reads *"nearest amber pad: 13 m to your left"* and is told where a
thing is, not that standing on it deploys. The obvious third line — *"stand on the pad to
deploy"* — is a **true, unconditional statement of the game's rule** and not a prediction about
this walk, so it does not repeat any of the four refuted mistakes; it was left out only because
the acceptance criterion for this round said the wording must contain **no instruction**, and a
mechanically checkable rule beat a judgement call.

**ASSUMPTION:** the amber pad is its own affordance and the mission list behind `Esc` is the
fallback, so no instruction is needed. **What to roll back if he disagrees:** the format string
in `hud::hub_prompt::hub_prompt_text` (one line), the `"start"` entry in
`hub_prompt::INSTRUCTION_WORDS`, and the three-line assertions in
`tests/hud.rs::f177_the_line_stands_above_the_keep_out_box_and_never_beside_the_objective` — a
fourth line does not fit the box the element is laid out in, so a third line that instructs has
to replace `Esc: Mission select`, not join it.

**And what may no longer be rolled back without a red test:** the pointer's ranking. *Nearest*
is not a taste call any more; it is the only ranking under which the line and
`mission::hub::deploy_on_contact` cannot name different doors, and
`tests/hud.rs::f177_no_stance_in_the_hub_names_a_door_that_is_not_the_nearest` reports
**6 802 164 wrong doors** the moment it is put back.

## ✅ ANSWERED 2026-08-27 — the interactive run-through, batch 1 of 6

**Q-063 · gravity — „Erst spielen, dann entscheiden."**
`-32` stays **provisionally**. The script corpus re-aim **waits for his verdict**, so it is not
paid twice. ⚠️ **Consequence: `f-007-boost`, `f003-ashgate`, `f025-chain` and the other ~10 stay
red until he has played.** That is deliberate, not neglect.

**Q-064 · Shift/boost — „erinnere mich später beim play test."**
Not decided. **But a play test of gravity is worthless while Shift nets +2 m/s²**, so `boost_m_s2`
is restored to the *old net lift* as a **provisional** value — `34 − 20 = +14`, so `32 + 14 = 46` —
and it is labelled provisional in `game.ron`. **The supervisor owes him this question at the play
test.** → carried into `docs/NEXT.md` as a standing item.

**Q-065 · Ctrl with two ropes — „Beide Seile bleiben, du hängst fest."**
He chose the physically honest reading: two ropes that contradict each other **hold** you.
⚠️ **That is NOT what the game does today, so this is still work.** Today one rope ends up
**50.167 m past its own maximum** and the solver pins him at 0.000 m/s — a constraint *violation*,
not a stand-off. His answer means: **the reel stops when the next step would be infeasible**
(`L_left + L_right >= anchor_separation`), both maxima stay satisfied, and the player hangs between
them. **He has accepted being stuck; he has not accepted a broken rope.**

**Q-066 · hub spawn — „Lass es, ich dreh mich um."**
The spawn facing stays. ⚠️ **This promotes the hub line from nice-to-have to load-bearing**: if the
player is not turned toward a door, the only thing that can name one is the text. So `F-177`'s
line — and `Q-059`, how long it stays up — is now on the critical path rather than beside it.
`docs/NEXT.md` §3E is superseded by this answer; the two rewrite options there are **not** to be
built.

## ✅ ANSWERED 2026-08-27 — batch 2 of 6

**Q-067 · the anchor field — 🔴 „es soll auf jeglicher oberflqche einhaken. nicht an hardcoded
punkten etc!"**

**This retires a whole subsystem, and the raycast the game already uses is the RIGHT design.**
- `world::AnchorField` — 787 lines, **1564 authored `hook.*` points + 8108 generated**, rebuilt at
  every load — is **the wrong idea**, not an unfinished one. Hooking is a property of *surfaces*,
  not of a curated point list.
- **`F-024` (snap to candidates) is not to be built.** `F-026`/`F-027` (the marker field) lose
  their subject. `B-011` closes as *won't fix*: the `Q`/`E` letters were withdrawn for the right
  reason and the rings should now follow them off the screen.
- ⚠️ **What SURVIVES and must not be deleted with it:** the **aim assist**
  (`assist_catch_pct` / `assist_strength_pct`, `vector::aim::probe_dirs`). That is a *sideways
  sweep of the ray to find a surface* — it never used the authored points — and he asked for it
  twice, including *„es soll in der ui angezeigt werden von wo bis wo gesearched wird"*. **The
  search band stays. The point list goes.**
- ⚠️ Check before deleting: `maps.ron`'s `hook.gesims_*` ladders and whatever else names
  `hook.*`, plus `tests/world.rs`'s `best_of` caller. Deleting a Resource that scripts assert on
  is its own round.

**Q-069 · sound — „Ich liefere die Sounds."**
He supplies the audio files; **the project builds the system and the trigger points only**. So the
work is `src/sound/` (24 lines, empty `Plugin::build`), the `audio` cargo feature (already exists,
needs `alsa` on machine B), and the events: gas, hook fire, hook bite, blade, footstep, cortex kill.
**No agent chooses a sound.** ⚠️ Ask him for the files before building the loader, or it loads
nothing and reports success (`prompts/init.md` §3: without `wav` a file plays silence and says
nothing).

**Q-047 · the nape — „Nur von hinten."**
A cortex kill counts **only from behind**. That is the design's own pillar — movement is the
combat — and it means the seven per-kind kill photographs must be taken against a rear-arc gate.
**Build the gate before photographing**, not after.

**Q-061 · `S` on the ground — „S spannt nur, bewegt nicht."**
His original words, restored: *„mit s »spannt« man nur das seil"*. `S` moves the player **neither
toward nor away** — it is tension, not thrust. That settles the one red line in
`scripts/f176-pull.txt`.

## ✅ ANSWERED 2026-08-27 — batch 3 of 6

**Q-062 · the gate — 🔴 „mach 2 tore daraus. erst du danach ein weicheres tor. ich sag wenn es
passt!"**

**The gate becomes two, in sequence, and the design bible's ten-tester sentence is superseded.**
- **Gate 1 — him, alone.** He plays and says yes or no in one sentence. That is the gate that
  unlocks work.
- **Gate 2 — a softer gate**, a few more people, after Gate 1 has passed.
- **„ich sag wenn es passt"** — ⚠️ **the gate is never declared passed by an agent, and never
  inferred from a green suite.** It is passed when he says the words, and only then. Whatever he
  says gets written down here with a date.
- This replaces `docs/gameplay/pillars.md`'s *"ten testers, blind, at least level with Attack on
  Titan Revolution"* as the operative rule. The bible's sentence stays as the *ambition*; this is
  the *procedure*.

**Q-071 · the maps — „Karten kriegen Zeilen."**
`M-001..M-012` get real feature rows so they enter `TODO.md` and `STATUS.md`. **He has authorised
touching `gameplay/features.xlsx`**, which is his file and the generator source. ⚠️ Run
`python3 tools/features.py --check` after, and regenerate — the ledger has not been regenerated in
15 days, which is the root cause named by four of six readers.

**Q-070 · Ashgate — „Intakte Stadt behalten."**
The town stays intact and inhabited. ⚠️ **So `docs/gameplay/world.md` is now the thing that is
wrong**, not the map: it says Ashgate has long since fallen and the Vanguard runs salvage into its
own ruins. **The document gets adapted to the game, not the game to the document** — and that is a
real edit somebody has to make, not a note. His 2026-08-18 *„das ist nicht die echte map!"* is
hereby answered as *the map is right, the story was wrong*.
⚠️ `docs/NEXT.md` §3C's queue item 1 (re-cut the wall so the ruin tile set can dress it) loses its
reason. The **untextured** hand-placed geometry — the wall, the gates, the HQ, the gantries, 203
blocks that are the whole silhouette — is a **separate** and still-open problem.

**Q-051 · progression — „Ganz ausbauen."**
Full build: the debrief shows what the sortie earned, levels and rank land somewhere visible, the
loadout screen lets the ~200 earned gear points be spent, and the difficulty ladder becomes real
(in 321 sorties only Recruit has ever been cleared).
⚠️ **This deliberately overrides `docs/PLAN-GAME.md` §10**, which forbids all of `progress/` until
the gate passes. **He is the gate now (`Q-062`) and he has chosen.** Recorded as an override with a
date rather than a quiet exception. **`Q-072` is answered by this too** — the debrief shows the
earnings.

## ✅ ANSWERED 2026-08-27 — batch 4 of 6

**§4A/1 · heights — „Das Gelände selbst — Hügel, Terrassen."**
The **ground** gets real relief, not the silhouette. That means `terrain.step_m` (1.50 m) and/or
`cell_m` (42 m) move, and `FIND-091`'s trade comes due: *a 0.36 m tread is a wall with a texture*.
⚠️ **`plan_terrain`'s stair asserts constrain this** — the fix is a number **plus** those asserts,
and `scripts/f003-ashgate.txt`'s 40 asserts are aimed at today's heights. **This lands with the
corpus re-aim, not before it.**

**§4A/2 · the map edge — 🔴 „unsichtbare wand + wenn man runterfällt wegen bug teleport man
zurück!"**
**Both, and they are two different mechanisms:**
1. an **invisible wall** so you cannot leave in normal play;
2. a **safety net** — if you end up below the world anyway (a bug, a seam, a bad warp), you are
   **teleported back** rather than falling forever.
⚠️ (2) is the one that must not be forgotten: it is a *recovery* rule, and it has to work even when
(1) has already failed, otherwise it is the same wall twice. A kill-plane far below that respawns
at the last safe grounded position.

**Q-068 · Traversal Trial — „Später, nach dem Kampf."**
Stays behind combat. Build order rank 11 moves after rank 8. ⚠️ **Consequence: Gate 1 (`Q-062`) is
judged in a normal sortie, not in a time trial** — so there is no number to improve against when
he judges the movement. Accepted; noted so nobody re-proposes it.

**Q-046 · gas tank — „zum testen. später etwas runter. aber nicht soo viel."**
`gas_tank: 15000` is a **test value**, not balance. It comes down **somewhat** later — explicitly
**not** all the way. ⚠️ For scale: the reference measures ~400 s per tank at ordinary flight
(`FIND-150`), and the pre-test value was 300. **"Etwas runter, aber nicht soo viel" is nowhere near
300** — do not read this answer as a mandate to restore it. Ask him for the number when boost is
working and flights are measurable again.

## ✅ ANSWERED 2026-08-27 — batch 5 of 6

**Q-073 · flying past an anchor — „Herumschwingen ist richtig."**
The arc is the verb. **`Q-058`'s stated cost is hereby accepted knowingly**: a hard maximum length
turns you at `L`, and that is different from the reference's pure drive (`FIND-149`). *A rope is a
rope.* Carrying speed through the turn becomes the skill, which is exactly what `F-014`
(momentum-chaining) has to measure.

**Q-059 · the hub line — 🔴 SUPERSEDED. „wenn man in der hub auf ein board drückt (F) dann kommt
man in eine mission übersciht in der man auswählen kann was man machen will!"**

**He did not pick any of the four options — he replaced the feature.** The answer is a **mission
board**: a physical object in the hub, `F` to use, which opens a mission overview where he chooses.

⚠️ **This retires the thing that was refuted four times.** `src/hud/hub_prompt.rs` and the whole
promise/pointer design exist to solve *"the player cannot find a door"* — and a board he walks up
to and presses `F` on solves it **without any predictive text at all**. No bearing rule, no walk
model, no 25 mm knife-edge, no ray. **The four refutations were all attacks on a mechanism he never
asked for.**
- ⭐ **And the object already exists**: the survey photographed *"a blank green signpost with no
  writing on it"* standing in the hub, right of the spawn view. It has been waiting for a job.
- **`Screen::Lobby` is the overview** — it is already built, lists `missions.templates`, has a
  `Deploy` button and passing tests. The board is the **door to it that the hub never had**.
- The six walk-on pads stay (they work, 35 asserts). The board is the *second* way in, and it is
  the discoverable one.
- ⚠️ **`F` is currently unbound in the script driver** (`debug::script::parse_key` knows
  W A S D Q E C F F3 Space Shift Ctrl Tab — `F` is there, good) but **no script can press `Esc` or
  click a menu**, so the board must be exercisable by `F` alone or its evidence dies the same way
  (`FIND-189`).

**Q-019 · the cortex — „nicht direkt sichtbar. man muss es wissen!"**
**No marker.** The nape is not highlighted at any range; the player learns where it is. That
settles `Q-026` with it, and it means the per-kind kill evidence is photographed against an
*unmarked* target. ⚠️ It also raises the bar on the **titan silhouette**: if nothing points at the
nape, the model has to read clearly enough that a player can find it. That is a modelling
requirement nobody has written down.

**Q-004 · Vessel Forms — „Spätere Version."**
Nine features out of scope for this version. The Vector Gear must convince first; a second movement
system beside it dilutes both.

## ✅ ANSWERED 2026-08-27 — batch 6 of 6, the run-through is complete

**Q-029 · the 26 invented numbers — „Später, beim Balancing."** Noted as open. They stay as they
are and the project keeps stating that they are derived, not chosen.

**Q-028 · the bellower — „Er soll vorkommen."** The 21 m kind gets unblocked and put into a
mission. ⚠️ `assets/data/scale.ron: max_spawnable_class` is what forbids it, and it was set for a
reason — check what that reason was before raising it. **21 m is a different fight**: you have to
go *up*, and with `Q-047` (rear only) plus `Q-019` (no marker) that is a real design problem, not a
spawn-table edit.

**Q-075 · multiplayer — „Richtig bauen."** World-state replication gets built, so two people can
actually fly together. The socket is input-only today. ⚠️ Rule 4 says the architecture has been
kept multiplayer-ready since day 1 — **this is where that claim gets tested**, and it is a large
piece of work. It does **not** go before the gate.

**Q-074 · the hit marker — „Beides."** Closing speed *and* damage, side by side.

---

# 🔴 THE RUN-THROUGH IS COMPLETE — 23 questions, all answered 2026-08-27

**What is now DECIDED and must not be re-opened by any round:**

| | decision |
|---|---|
| the gate | **two gates: him first, then a softer one.** *„ich sag wenn es passt"* — **no agent ever declares it passed** |
| the anchor field | **deleted.** *„auf jeglicher oberfläche einhaken. nicht an hardcoded punkten"* — `F-024` is not built |
| finding a mission | **a board in the hub, `F`** — not a predictive HUD line. `hub_prompt` is retired |
| the rope | hard maximum length, the arc is right, `A`/`D` tangential to the **anchor**, `S` tensions only |
| the nape | **rear only, and never marked** — *„man muss es wissen"* |
| the ground | **real relief** — hills and terraces, not a flatter grade |
| the map edge | **invisible wall + a teleport back** if you fall through anyway |
| Ashgate | **stays an intact town**; `docs/gameplay/world.md` is what gets corrected |
| progression | **built out fully** — this overrides `PLAN-GAME.md` §10, knowingly |
| sound | **he supplies the files**; the project builds the system and the triggers |
| multiplayer | **built properly**, after the gate |
| gravity · boost · gas | **decided at the play test.** He must be asked. |

**Still open, deliberately:** `Q-063` gravity, `Q-064` Shift, `Q-046` gas tank — all three wait for
him at the controller. `Q-029` the 26 numbers, at balancing.

---

## Q-076 — ✅ ANSWERED 2026-08-27: the player gets **real mass**. Forces, not accelerations.

**He asked the right question and it was not the one he thought:** *„kann es sein, dass der player
bei der gravitation nur 1kg wiegt und deshalb so leicht fällt?"*

**Mass cannot affect falling** — gravity is an *acceleration*, `a = g`, in this engine and in
reality. So the answer to the literal question is no. **But his instinct was right one level over**,
and `src/player/locomotion.rs:701` already says so, quoting him:

> *„die masse von dem character … es fühlt sich zu leicht an"* … **A body like that has no
> inertia**; `Forces::apply_linear_acceleration` ignores mass on purpose, so nothing else in the
> game supplies any either.

Every force in the game — `vector::boost`, `vector::dodge`, `player::locomotion` (both the ground
and the rope drive) — calls `apply_linear_acceleration`, **documented by avian as "ignoring mass"**.
So a boost changes your velocity by the same amount whether you are standing still or flying at
70 m/s. **That is the "too light", and it is a decision, not a bug.**

**He chose: build real mass.** `apply_force` instead.

### ⚠️ What that costs, stated before anyone starts

1. **Every tuning number in `game.ron` changes meaning.** `boost_m_s2: 34` is an acceleration you
   can check against `gravity_m_s2: -32` in your head — *"the boost beats gravity by 6 %"*. As a
   force it is newtons, and the sentence becomes `F/m` against `g`. The file's whole style of
   self-checking comments is built on the current form.
2. **It is measured how big the difference is** (`src/vector/boost.rs:38`, `tests/vector_boost.rs`):
   with `apply_force` a **10 kg** player reaches **−7.68 m/s** where a **0.6 kg** one reaches
   **−112.79**; with the acceleration both reach **−68.002785, bit for bit**. So mass becomes the
   single most load-bearing number in the file, and it does not exist yet.
3. **It is not one edit.** `boost.rs:307`, `dodge.rs:179`, `locomotion.rs:1208` and the rope drive
   all write through the same door, and the rope **joint** now solves against the body too — a
   mass the solver disagrees with is a different kind of bug.
4. **Every measured number in `docs/FINDINGS.md` that quotes an acceleration becomes historical.**

### The sequencing, and it matters

**Mass lands BEFORE the play test**, not after — otherwise he tunes gravity, Shift and the gas tank
against numbers that are about to change meaning underneath him (`Q-063`, `Q-064`, `Q-046`).
⚠️ But **Shift must be restored first** (`boost_m_s2` nets +2 m/s² today), or there is nothing to
feel the mass with.

**Rollback point:** the `apply_force` / `apply_linear_acceleration` call sites named in (3), plus
the mass value itself. Reverting is mechanical; re-tuning afterwards is not.

---

## Q-077 — 🔴 „zu leicht" is STILL OPEN, and `Q-076` is not its answer

**2026-08-27.** He was told, before any work started, that *real mass cannot fix "too light"*:
`a = F/m`, so the resulting acceleration is velocity-independent whether the game applies a force
or an acceleration. A 100 kg body and a 1 kg body change direction equally fast when the force
scales with the mass. The measured pair in `src/vector/boost.rs:38` (10 kg → −7.68 m/s, 0.6 kg →
−112.79) only says what happens if the **same number** is re-read as newtons instead of m/s² — an
arbitrary rescale, not weight.

**He chose real mass anyway, and explicitly as its own task:** *"auch wenn es das Gefühl nicht
ändert — z.B. damit Titanen dich schubsen können und Kollisionen Gewicht haben. Dann aber als
eigene Aufgabe, nicht als Lösung für 'zu leicht'."* So `Q-076` is now a **collision and
push-back** feature and it is **not** the fix for the complaint.

**Therefore the complaint is unresolved and stays on the books.** `src/player/locomotion.rs:701`
already names the real cause and it is not mass:

> `(v* − v)/τ` … **replaces the whole velocity in the same ~3τ however fast the player was
> going. A body like that has no inertia.**

The drives are **velocity chases**. `clamp_length_max` bought some of it back — measured, 15 ticks
from rest to 90 % of drive speed but **27** to turn a flight around — and that is the only weight
in the game today.

**The three candidates that would actually produce weight**, none chosen:
1. **Turning costs time in proportion to speed** — real inertia, and the one that matches the
   complaint most directly. Straight-line response stays instant.
2. **Drag**, a braking force growing with `v²` — makes high speed expensive to hold and gives the
   gas an opponent.
3. **Lower acceleration ceilings** — cheapest, but it makes everything sluggish, including the
   small corrections `FIND-153` says must stay instant (*„man macht was und man merkt es direkt"*).

**ASSUMPTION the work continues under:** nothing is built for this yet. The heavier world
(`gravity_m_s2: -32`) and the restored Shift may already have changed how it feels, and asking him
to judge a mechanism before he has flown the new numbers would waste the round.
**He gets asked at the play test, with `Q-063`, `Q-064` and `Q-046`.**

**Rollback point:** none — nothing was built.

---

## Q-078 — ✅ ANSWERED 2026-08-27: **everything is hookable.** The tag system goes too.

He was asked how far *„es soll auf jeglicher oberflqche einhaken"* reaches, given that
`F-003 Getaggte Ankerflaechen` is a feature in his own spreadsheet whose stated purpose is
*"Verhindert Physik-Exploits, macht Leveldesign steuerbar und definiert ueber die Flaechendichte
die Traversal-Schwierigkeit einer Map"*, and whose acceptance is *"Kein Haken auf ungetaggten
Parts moeglich"*. He chose **"Wirklich ALLES hakbar"**.

**So this cancels `F-003` as well as `F-021`–`F-025`.** Hooking becomes a property of *existing*,
not of *tagging*.

### What that touches — check every one before calling it done

| | |
|---|---|
| `maps.ron: anchorable` | becomes **dead data** on every block. Main head's file |
| `tests/vector_aiming.rs::f002_an_untagged_wall_in_front_of_a_roof_is_not_hookable_and_not_transparent` | **inverts** — the wall must now hold a hook. Its *other* half (the wall is not transparent) stays |
| the cast filter in `vector::aim` | the `anchorable` predicate comes out; the mask keeps only what is physically solid |
| `F-003`'s acceptance sentence | cancelled. Its 63-tagged-surface evidence becomes historical |
| **titan bodies** | now hookable **for free** — which is `F-029 Dynamische Ankerpunkte`, and it arrives as a side effect rather than as a feature. ⚠️ A rope anchored to a walking titan is a moving constraint; that is not free at all, it is `F-029`'s whole difficulty |
| the ground | hookable. A hook into the street under your feet is now legal and nobody has felt it |

### 🔴 The reason `F-003` existed does not disappear because the feature does

*"Verhindert Physik-Exploits"* — the tag was the guard. With everything hookable, the guards that
remain are `vector.max_speed_m_s` (75, an avian `MaxLinearSpeed`), `min_rope_m` (3.0) and the
joint's own limits. **Nobody has checked whether those are sufficient on their own**, and
`F-012 Velocity-Clamp gegen Fling` is still marked Unbuilt in the ledger while being the exact
feature that would cover this. **That row should be re-checked against the tree before anyone
relies on it.**

**ASSUMPTION the work continues under:** the existing speed clamp and rope floor are the whole
guard, and no new one is added until something is measured flinging a player.
**Rollback point:** the `anchorable` predicate in `vector::aim`'s cast — one condition. Restoring
it brings the tag rule back; the RON data is not deleted, only ignored, so it stays reversible.

### 🔴 AND HE ADDED THE PART THAT DECIDES THE ARCHITECTURE, minutes later:

> *„später soll man auch bestimmte sachen toggeln könenn. also an bestimmte sachen ran haken an
> andere nicht aber grundsetzlich erstmal ales!"*

**So the tag data is NOT dead and must NOT be deleted.** It becomes a **filter he can switch**,
and today every switch is on. That changes the shape of the work from *remove a condition* to
*replace one boolean with a set of categories, all enabled*:

- **`maps.ron: anchorable` stays**, and whatever category vocabulary the blocks already carry
  (class, palette, model) is what a future toggle will select on. **Deleting it would destroy the
  thing the toggle needs.**
- The cast predicate becomes *"is this category currently hookable?"* with every answer `true`,
  rather than *"is this block tagged?"*.
- ⚠️ **Do not build the toggle UI now.** He said *„später"* and *„grundsätzlich erstmal alles"* —
  the requirement today is that the future toggle is **cheap**, not that it exists. A single
  predicate with one call site is enough; a settings screen is not asked for.

**This also rescues the reason `F-003` existed.** Level design keeps its lever — it is simply
*off by default* instead of *on by default*, which is the inversion the third option in the
question described. He picked "everything" and then described the third option himself.

---

## Q-078 (addendum) — ✅ BUILT 2026-08-28. The rollback point moved: it is a **value**, not a condition

The answer above ended with *"Rollback point: the `anchorable` predicate in `vector::aim`'s cast —
one condition."* That is no longer where it lives, and whoever needs to undo this has to know:

**The rollback is now `HookableSurfaces::TAGGED_ONLY` instead of `HookableSurfaces::default()`** —
one value in `src/vector/hookable.rs`, no code change, and `maps.ron: anchorable` still carries
every bit `F-003` needs. It is exercised by a test rather than argued about
(`tests/vector_aiming.rs::f003_the_tag_survives_as_a_switch_that_can_take_the_untagged_surfaces_back_out`
drives the real cast on the real map, flips the resource, and flips it back).

**What was built:** `src/vector/hookable.rs` — `SurfaceKind::{Tagged, Untagged}`,
`HookableSurfaces` (a bit set, `EVERYTHING = u32::MAX` so a kind added later arrives hookable),
and `is_hookable`, called from the **two** places that used to read the `ANCHORABLE` bit
separately. **No settings screen** — he said *„später"*, and the requirement today was that the
toggle is cheap.

**The two categories are the reach, not the ambition.** *Titan bodies* and *the ground* are the
switches he will most likely want, and neither can be told apart at the cast site today: a titan's
root capsule and a house both arrive as a `shared::Body` with `SOLID | ANCHORABLE`, and `vector`
has no allow-list edge to `titan`. Adding either is one variant plus one line in `SurfaceKind::of`
— but it needs a marker in `shared` or an edge with a reason first.

**The guard question in the answer above is now measured, not open:** `scripts/q078-fling.txt`,
30 asserts, exit 0. `max_speed_m_s` (75) is a hard clip that nothing reached from below;
`min_rope_m` plus the fade band held a drive into an anchor to 4.9 m/s; a rope on a walking titan
against a rope on a static roof held for 5.5 s with **7 cm** of height drift. Full table in
`docs/FINDINGS.md` FIND-199. **`F-012 Velocity-Clamp gegen Fling` is not urgent.**

⚠️ **The one thing that needs HIM and not a test:** a hook into the pavement under your own feet
now **adds to gravity** — 21.3 m/s where free fall gives 15.5 — because the pull points at the
anchor and the anchor is straight down. It terminates on the floor and it is not an exploit; it is
a yank nobody has played. **ASSUMPTION:** it stays as it is until he says it feels wrong.
**Rollback point:** none needed — no code was written for it; it falls out of the rope pull.

**Related:** `FIND-199` · `FIND-200` · `F-003` `F-012` `F-029`

---

## Q-062 — the board is built; `hold F` is my choice and not yours, and `Q-059`/`Q-060` are now moot (2026-08-28)

> *„wenn man in der hub auf ein board drueckt (F) dann kommt man in eine mission uebersciht in der
> man auswaehlen kann was man machen will!"* — you, 2026-08-27.

Built and measured (`docs/FINDINGS.md` FIND-201, `scripts/f177-board.txt`: 17 asserts, exit 0). Walk
to the signpost right of the spawn view — you already said *„Lass es, ich dreh mich um."*, so
nothing turns you round and this is the door you find — and:

| you press | it does |
|---|---|
| `F`, standing at the signpost | the mission overview opens |
| `F` again, a quick tap | one sortie on, through all 13 of `missions.ron` |
| `F` **held** (0.35 s) | deploys the one it is showing |
| `F` anywhere else | **nothing.** It stays the left blade |
| `F` in a sortie | **nothing.** The board only exists in the hub |

With a window that overview is the mission plate you can also click; the keyboard route above works
either way, and walking away shuts it.

### ⚠️ The one thing that is yours and not mine: **the deploy key**

I gave `F` two meanings — **tap = next, hold = fly** — because you named exactly one key and a
sortie needs two verbs. A hold is not a double-tap: it has a floor and no ceiling, so there is no
timing window to miss (`src/net/local.rs` refuses gestures for the dodge on that argument, and this
is the other side of it). But it is still one key doing two things, and if it feels wrong the
alternatives are cheap: a second key for deploy, or `Deploy` as the last row of the cycle.

**ASSUMPTION the work continued under:** tap steps, hold deploys, `hold_s = 0.35`.
**Rollback point:** `assets/data/missions.ron: hub.board.hold_s` for the feel, and the
`just_released` / accumulator branch in `src/menu/board::work_the_board` for the scheme. Nothing
outside that function and that one file field decides it — `menu::lobby`, the plate, the six pads
and `shared::DeployRequest` are untouched by either answer.

### ⛔ And `Q-059` and `Q-060` no longer have a subject

Both ask what the **hub line** should say — should it step aside once you know it, and what should
it say when it cannot tell which door opens. That line was `hud::hub_prompt`, it was refuted four
times, and it is deleted. **The board answers neither question and does not need to:** it says what
one key does while you stand in one circle, so it has nothing to predict and nothing to be wrong
about. Nothing is being rolled back for them; they are simply about a thing that is gone.

**Related:** `FIND-201` · `FIND-189` · `Q-059` · `Q-060` · `F-177`

---

## ✅ ANSWERED 2026-08-29 — the four he could answer without the controller

**B-020 · three husks kill you at `-32` — „Erst beim Playtest spüren."**
`scripts/f032-swords.txt` stays **red on purpose** with `assert health > 0` and `B-020` open. Not a
calibration, not yet a bug — his call, at the controller. ⚠️ **Do not re-aim it, and do not raise
`player.health`.**

**The script invariant — 🔴 „Je nachdem, was das Skript behauptet."**
This becomes a rule, and it exists because two groups in one round pinned **different quantities
for the same physical act** — combat pinned the *pass speed at the nape* (*"the game reads speed"*),
world pinned the *fall time* — and both satisfied the same validity control. The consequence was
measured: the hub loop's closing speed went **20.67 → 33.07 m/s (+60 %)**, into a different damage
regime (`gear.ron: feel.strong_hit_m_s` 18.0 splits CUT from GRAZE), without anybody deciding it.

> **Every evidence script states in its own header WHICH QUANTITY IT PINS when a constant moves** —
> the impact speed, the fall time, the height, the tick gap — and every re-derivation in that file
> holds that one and lets the others follow. **"It reproduces the old number at the old gravity" is
> satisfied by any parameterisation that pins one quantity; it cannot say which.**

**Next work — „Geländehöhen und Kartenrand fertig."**
The ground gets real relief (his earlier *„Das Gelände selbst — Hügel, Terrassen"*), and the map
edge's loose ends are closed.

**Play test — „Später — bau weiter."** `Q-063` gravity, `Q-064` Shift, `Q-046` gas and `Q-077`
"too light" stay open and provisional. **The supervisor owes him all four the moment he plays.**

---

## ✅ ANSWERED 2026-08-29 — the water and the wall

**Water on contact — „Man schwimmt / wird langsam."**
You fall in, lose speed and gas, and work your way out. **Not lethal.** So water is *terrain with a
cost*, which means it needs a volume, an entry rule, a drag and an exit — not just a coloured
plane. ⚠️ It also needs an answer for the Vector Gear: a hook fired *from* the water, and gas
spent while in it.

**Hookable — „Nein — Wasser hält keinen Haken."**
Unchanged from today's channel: `anchorable: false`. ⚠️ **This is the one deliberate exception to
`Q-078`'s "everything is hookable"** — and it is exactly the toggle he asked for there
(*„später soll man auch bestimmte sachen toggeln können"*). Water is the first category that is
switched **off**, and that is what the switch was built for.
The bridges and the new towers stay the crossing, which is what makes the river cost something.

**The entrance gates — 🔴 „Echte AoT-Mauer mit Toren drin."**
The wall becomes the structure and the gate a passage through it, as in the reference. **The swing
anchors come from the wall's own ledges and cornices instead of from freestanding crossbeams.**

⭐ **AND THAT IS ALREADY THE PROJECT'S OWN NUMBER ONE, measured and then filed as cosmetics.**
`FIND-134`, 2026-08-19, closed with a queue whose first item reads:

> **Re-cut the wall into modules** so the tile set can dress it — biggest aerial win, and **it
> unlocks `hook.gesims_*` anchors along the wall as a side effect.**

Those `gesims` (cornice) anchors are *precisely* the swing points his answer needs, and the same
finding measured why the wall cannot be dressed today: the model pack's wall vocabulary
(`a-095`, `a-096`, `a-101`) is a **tile set at one module — 11.20 m wide, 120 m tall** — while
Ashgate's wall is **monolithic 700 / 336 / 285 m bands**, and `fit_to_class` scales uniformly: it
can fit a tile to a box, it cannot repeat one along it (700 / 11.2 = 62.5, and the runs do not even
divide).

**So one job answers three of his complaints at once** — the entrance shape, the swing anchors, and
the bare aerial silhouette — and it was sitting in the queue described as a level-design chore.
⚠️ It is expensive: every collider in the silhouette, the 40 asserts of `scripts/f003-ashgate.txt`,
and the `hook.*` ladders move with it. **It goes after the terrain, which is reshaping the ground
those bands stand on.**

**Related:** `FIND-134` · `Q-078` · `docs/NEXT.md` §5A · `F-003` `F-004` · `M-002`

