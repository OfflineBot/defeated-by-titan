# Defeated by Titan

A 3D low-poly action game about the fight against Titans, built in **Bevy (Rust)**.

You are a Vanguard salvager with the **Vector Gear**: two grappling hooks, two gas tanks, two
blades. You hook in, you swing, you accelerate on gas — and you kill a Titan **only** with a
fast cut into the **Cortex**. Anything else costs him a leg and costs you time.

> **The game in one sentence:** a movement game with a high mastery ceiling, in which fighting
> is the side effect of moving well.

The war is already lost. Ashgate has fallen; the Vanguard runs salvage missions into its own
ruins. The tone is muted and grown-up, never cynical — Titans vaporize instead of bleeding.

## Where it stands

**Setup.** There is no playable game yet: the project tree, the tools, the data extraction and
the documentation stand, and the build-up plan is at step 0/1.

The honest state of each individual thing is in **[`docs/STATUS.md`](docs/STATUS.md)**, with
one of four stages per row:

| | Stage | means |
|---|---|---|
| ⬜ | unbuilt | does not exist, or only as a stub. Also: "there is code, but it does nothing" |
| 🟨 | built | built, **not tested, not seen**. It compiles. Nothing more than that is claimed |
| 🟧 | proven | built **and** covered by tests that go red **and** seen in the running game (screenshot) |
| ✅ | accepted | **the user looked at it and signed off** — only he sets this |

## Running it

Rust 1.85+ (edition 2024). On machine A the toolchain lives in `~/.cargo`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

**Which window system gets linked is decided by the machine** — Bevy's umbrella features pull
`wayland` in hard, and that does not build everywhere:

```bash
cargo run                     # x11 (default) — builds everywhere, even without Wayland libraries
cargo play                    # = cargo run --features wayland,audio   (Wayland compositor, e.g. niri)
```

### Startup flags

For a tool, a main menu is a wall without a door. So there are ways past it:

| Flag | what for |
|---|---|
| `--sandbox` | empty field, one titan, unlimited gas — for looking at things |
| `--mission <name>` | straight into a sortie, no menu |
| `--headless` | **no window**, fixed tick, runs N ticks and exits with an exit code. The only way on a machine without a display |
| `--script <file>` | play the game without typing — and with `assert` the run becomes a test |
| `--novsync` | for measuring. Under vsync every frame time is 16.6 ms, so you measure the same ceiling six times |
| `--lag <ms>` | simulated latency. **Every movement feature is checked at 200 ms too**, not only locally |

## Key bindings

**Not settled yet, because none of it is built yet.** What is settled, out of the design
bible: **PC only, keyboard and mouse as the sole input device** — no gamepad, no touch. And:
rebindable keys are a requirement, not a nicety.

As soon as step 1 stands, the table comes here. Until then it would be a claim.

## The layout

```
src/        one domain = one folder = one plugin (vector, titan, combat, world, net …)
assets/     data/ (the RON numbers)  3d/  textures/  audio/  vfx/  extern/
tools/      builds things: features.py, norms.py, blend/, atlas/, sound/
scripts/    plays the game: --script runs
docs/       the mirror: STATUS, TODO, architecture, conventions, case histories
tests/      tests/<domain>.rs — plus domains.rs, multiplayer.rs, models.rs
```

**Balancing is file work, not Rust.** A new titan kind, a blade tier, a gas cost: all of it in
`assets/data/*.ron`. The code holds units and mechanics only — otherwise the most frequent
work in the project needs a rebuild, and therefore does not happen.

## Contributing

- **What is to be built** is in [`docs/TODO.md`](docs/TODO.md) (generated, in buildable order)
  — the source is `gameplay/features.xlsx` with 687 ticket rows.
- **How the work is done** is in [`CLAUDE.md`](CLAUDE.md) and
  [`docs/README.md`](docs/README.md).
- **Wishes and design** go into the inbox `gameplay/` until further notice.
  *(Once the bootstrap scaffolding is dissolved it is `docs/gameplay/` plus
  `docs/TODO.md` — `prompts/init.md` §18.)*

**The reference:** [Attack on Titan Revolution](https://www.roblox.com/games/13379208636/Attack-on-Titan-Revolution)
(Roblox). What is taken from it is the *feel* of the gear, not the platform: this here is a
standalone game in Bevy/Rust.
