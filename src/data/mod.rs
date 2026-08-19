//! data — load the RON files. **Runs before everything else.**
//!
//! > **Numbers belong in RON, not in Rust.** A new Titan kind, a blade tier, a gas cost
//! > figure: file work, not Rust. Only *units* and *mechanics* stand in the code
//! > (`prompts/init.md` §4).
//!
//! Why that is not optional: **balancing is the work that happens most often.** If it needs a
//! rebuild, it does not happen. And another agent can change one RON line without
//! understanding this code.
//!
//! **No `serde(default)` for game values.** A missing value is meant to crash at load time,
//! not to quietly slip a zero in — otherwise you hunt the bug in the code while it sits in the
//! file. That is also why loading happens **synchronously during setup** and not through the
//! `AssetServer`: an error is meant to be loud **at startup**, with file name and line, and
//! not three systems later as an empty screen (§9d).
//!
//! **And `#[serde(deny_unknown_fields)]` on every type in here.** Leaving a field out always
//! crashed; *adding* one did not — serde reads past an unknown field without a word. Measured
//! on 2026-08-09: an `erfunden_m: 42.0` in `scale.ron` and a `gewicht_kg: 70.0` under
//! `game.ron: player` both loaded without a word. That is exactly the trap for the file the
//! user's numbers are going to be added to from now on: an addition at the wrong nesting level
//! vanishes silently, and the number you typed in is not in the game.
//!
//! **This is the only place that knows file names.** Everybody else asks for the logical name;
//! `tools/norms.py` falls over when a path stands in the code anywhere else.

use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GameData::load(&assets_root()));
    }
}

/// Where `assets/` lives — **absolute**, and found from any working directory.
///
/// **This is the one place that answers the question**, for the RON files *and* for Bevy's
/// asset server (`crate::base_plugins` hands the result to `AssetPlugin::file_path`). Two
/// answers to it is exactly how the repository spent 2026-08-18 with a game that read its
/// numbers from the repository and looked for its models in `target/debug/assets/`.
///
/// The order is: **the working directory first** — that is what `cargo run` gives, and it is
/// what a mirror asset root in a scratch directory relies on (`docs/lessons/workflow.md`) —
/// and then **the crate the binary was built from**. That second one is the compile-time
/// `CARGO_MANIFEST_DIR`, not the environment variable: it stands in the binary, so the bare
/// `./target/debug/…` finds the repository no matter where it is started from and no matter
/// where the executable has been copied to.
pub fn assets_dir() -> PathBuf {
    let here = PathBuf::from("assets");
    if here.join("data").is_dir() {
        return here.canonicalize().unwrap_or(here);
    }
    let at_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    if at_crate.join("data").is_dir() {
        return at_crate;
    }
    panic!(
        "assets/data/ not found — neither at {:?} nor at {:?}.\n\
         The binary carries the path of the crate it was built from; if the repository has \
         moved, rebuild it (prompts/init.md §3).",
        here.canonicalize().unwrap_or(here.clone()),
        at_crate
    );
}

/// Where the RON files live. Derived from [`assets_dir`] — never resolved a second time.
fn assets_root() -> PathBuf {
    assets_dir().join("data")
}

/// Everything that comes out of `assets/data/`. One resource, many readers, **no writer**.
#[derive(Resource, Debug, Clone)]
pub struct GameData {
    pub game: Game,
    pub gear: Gear,
    pub titans: Titans,
    pub art: Art,
    pub missions: Missions,
    pub traits: Traits,
    pub maps: Maps,
    /// The **one truth about sizes**, laid down by the user. Every other file only mirrors it;
    /// `tests/data.rs` falls over the moment one of them deviates from this.
    pub scale: Scale,
}

impl GameData {
    pub fn load(dir: &Path) -> Self {
        GameData {
            game: load_ron(dir, "game.ron"),
            gear: load_ron(dir, "gear.ron"),
            titans: load_ron(dir, "titan.ron"),
            art: load_ron(dir, "art.ron"),
            missions: load_ron(dir, "missions.ron"),
            traits: load_ron(dir, "traits.ron"),
            maps: load_ron(dir, "maps.ron"),
            scale: load_ron(dir, "scale.ron"),
        }
    }

    /// A map by its logical name. `None` means: it is not in `maps.ron` — and the caller
    /// reports that loudly instead of building an empty world.
    pub fn map(&self, id: &str) -> Option<&Map> {
        self.maps.maps.get(id)
    }

    /// The map that is built at startup (`maps.ron: current`).
    /// `tests/data.rs` pins down that it exists.
    pub fn current_map(&self) -> Option<&Map> {
        self.map(&self.maps.current)
    }

    /// A color out of the one palette. `None` means: the key does not stand in `palette` — no
    /// silent substitute, or sooner or later a signal color slips in.
    pub fn color(&self, name: &str) -> Option<[f32; 3]> {
        self.maps.palette.get(name).map(|(r, g, b)| [*r, *g, *b])
    }

    /// A Titan kind by its logical name. `None` means: it is not in the RON — and the caller
    /// reports that loudly instead of inventing a stand-in Titan.
    pub fn titan(&self, kind: &str) -> Option<&TitanKind> {
        self.titans.kinds.get(kind)
    }

    pub fn model(&self, name: &str) -> Option<&Model> {
        self.art.models.get(name)
    }

    /// A size class by its logical name (`scale.ron: titan.classes`). `None` means: the class
    /// does not stand in the RON — `tests/data.rs` catches that before a Titan of height 0
    /// stands in the ground.
    pub fn size_class(&self, name: &str) -> Option<&SizeClass> {
        self.scale.titan.classes.get(name)
    }

    /// The height of a Titan kind. It does **not** stand in `titan.ron` — only the class does,
    /// and the height comes out of `scale.ron`. One number, one place.
    pub fn titan_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.size_class(&kind.size_class).map(|k| k.height_m)
    }

    /// Where the Cortex sits — **the user's figure in meters**, not `height_m * 0.89`.
    ///
    /// **The only lethal weak point** (`F-030`). Until 2026-08-09 it was computed here out of
    /// the fraction; that was convenient and wrong: the user names five Cortex heights in
    /// meters, and *a direct figure in meters beats every derivation*. For the small class the
    /// calculation was off by 4 cm. The fraction is now what it is for the user — a rule that
    /// `tests/data.rs` checks the five numbers against.
    pub fn titan_cortex_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.size_class(&kind.size_class).map(|k| k.cortex_height_m)
    }

    /// The largest head height the user's head rule allows for this kind
    /// (`height_m * scale.titan.max_head_fraction`, that is 1/9 of the body height).
    ///
    /// It is the **geometric upper bound for `cortex_radius_m`**: a hit zone whose diameter is
    /// larger than the whole head cannot be the base of a neck. Before this guard, `scuttler`
    /// (0.80 m Cortex on a 0.47 m head) and `weaver` (0.90 m) carried exactly that.
    pub fn titan_max_head_height_m(&self, kind: &TitanKind) -> Option<f32> {
        self.titan_height_m(kind).map(|h| h * self.scale.titan.max_head_fraction)
    }
}

/// Reads a RON file, or aborts with a message that points at the error **in the file** instead
/// of in the code.
fn load_ron<T: for<'a> Deserialize<'a>>(dir: &Path, file: &str) -> T {
    let path = dir.join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("{}: cannot be read — {e}", path.display())
    });
    ron::de::from_str(&text).unwrap_or_else(|e| {
        // ron names line and column; those are exactly what you want to see.
        panic!("{}: not valid RON — {e}", path.display())
    })
}

// ---------------------------------------------------------------------------
// game.ron — tuning: Vector Gear, camera, physics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub simulation_hz: f64,
    /// Solver substeps per simulation step — avian's [`SubstepCount`](avian3d::prelude::SubstepCount),
    /// whose own default is 6. **Measured, not guessed:** 24 is the smallest value that holds
    /// both the swing loss (4.26 %/s against 8.97 %/s at 6) and the wall. It belongs here and
    /// not in Rust, because it is the price of the simulation and thus a number you tune (§4).
    pub substeps: u32,
    pub gravity_m_s2: f32,
    pub player: PlayerTuning,
    pub vector: VectorTuning,
    pub camera: CameraTuning,
    pub world: WorldTuning,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerTuning {
    pub height_m: f32,
    pub radius_m: f32,
    /// `P5`. **⚠️ UNTUNED.** The second of the two ways to lose. At zero the player is
    /// [`MovementState::Downed`] — **a state with a timer, never a despawned entity.**
    ///
    /// [`MovementState::Downed`]: crate::shared::MovementState::Downed
    pub health: f32,
    pub run_speed_m_s: f32,
    pub jump_speed_m_s: f32,
    /// `F-006`. **⚠️ UNTUNED.** How hard WASD pushes in flight, in m/s²
    /// (`player::locomotion::air_control`).
    ///
    /// Stood in Rust as `-gravity_m_s2 / 2` until 2026-08-12 (`docs/FINDINGS.md` FIND-051) and
    /// is a key now because §4 says a game value belongs in the file. The derivation it came
    /// from is still the bound: **strictly below `-gravity_m_s2`**, or WASD alone holds you up
    /// and gasless hovering is free — and well below [`VectorTuning::boost_m_s2`], or `Shift`
    /// stops being the strong option. `tests/data.rs` holds the first of those.
    pub air_accel_m_s2: f32,
    /// `F-006`. **⚠️ UNTUNED.** What is left of [`air_accel_m_s2`](Self::air_accel_m_s2) with an
    /// empty tank. Dimensionless, so no unit suffix.
    ///
    /// **Not a derivation — the user's spec:** *„ohne gas kann man immernoch w a d nutzen um
    /// etwas movement aufzubauen (aber hälfte ca)"* (`docs/NEXT.md` §1e). The air control is
    /// therefore not gated on gas; it only gets weaker.
    pub air_accel_empty_fraction: f32,
    /// **⚠️ UNTUNED, and read by nothing yet** — `W4` (`player::locomotion`) bills it. What `W`
    /// adds **on top of** [`air_accel_m_s2`](Self::air_accel_m_s2) along the rope while a hook
    /// is anchored, in m/s².
    ///
    /// The user, 2026-08-12 (`docs/NEXT.md` §1A): *„wenn ich mit seilen festhake … und w in die
    /// richtung drücke will ich dass man deutlich mehr geboosted wird"*. Two bounds, both in
    /// `tests/data.rs`: strictly **above** `air_accel_m_s2` (or "deutlich mehr" is untrue and
    /// the key buys nothing), and at most [`VectorTuning::boost_m_s2`] (or free thrust beats
    /// the thrust you pay gas for).
    ///
    /// No `serde(default)`: at `0.0` this is the old behaviour, and that has to be a decision
    /// somebody wrote into the file rather than a value nobody noticed was missing.
    pub air_pull_m_s2: f32,
    /// **⚠️ UNTUNED, and read by nothing yet.** What `A`/`D` add on top while a hook is
    /// anchored, in m/s² — on the **horizontal right vector**, never the rope tangent, which
    /// flips sign when the anchor passes beside you (`docs/NEXT.md` §1B).
    ///
    /// The bound is a **sum**: `air_accel_m_s2 + air_lateral_m_s2 <= boost_m_s2`, so that
    /// holding one strafe key can never beat one boost.
    pub air_lateral_m_s2: f32,
    /// **⚠️ UNTUNED, and read by nothing yet.** Over how many metres of rope length the rope
    /// pull fades out above [`VectorTuning::min_rope_m`].
    ///
    /// Exists because of a **measured** cliff, not a feeling: `docs/FINDINGS.md` FIND-035 —
    /// at `min_rope_m` the length constraint takes 17 m/s out of the player in one tick, and
    /// thrusting at an anchor you are already next to feeds exactly that. Bounds:
    /// `>= 2 * min_rope_m` (or the fade is shorter than the cliff) and `<= 0.1 * hook_range_m`
    /// (or a near-anchor special case runs over a tenth of every rope in the game).
    pub air_pull_fade_m: f32,
    pub eye_height_m: f32,
    /// Largest distance per substep of the integrator. Has to be strictly smaller than
    /// [`WorldTuning::min_wall_m`], or the player tunnels through the thinnest wall.
    pub max_substep_m: f32,
    /// Coefficient of friction of the player's capsule against everything else.
    ///
    /// Combined with the surface's own value by `CoefficientCombine::Min`, so that a wall
    /// cannot brake the player — a measurement showed a combined 0.65 eating 75 % of the
    /// speed per second when a swinging player grazes a wall in a 7 m alley.
    pub friction: f32,
    /// Coefficient of restitution of the player's capsule. `0.0` = no bounce.
    pub restitution: f32,
    /// How steep a surface may be and still count as ground. Degrees in the RON, radians in
    /// the code (`docs/conventions.md`). Decides whether [`MovementState::Grounded`] holds —
    /// and with it whether you may jump off it.
    ///
    /// [`MovementState::Grounded`]: crate::shared::MovementState::Grounded
    pub max_ground_slope_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorTuning {
    pub hook_range_m: f32,
    pub hook_speed_m_s: f32,
    /// How fast a **missed** hook comes back, in m/s. The bound is not "faster than the
    /// outward flight" any more — since 2026-08-12 both are 500 — but a player-visible one that
    /// `tests/data.rs` holds: `hook_range_m / this <= 1.0 s`, i.e. a miss at maximum range is
    /// cleared inside a second, because §1A's first requirement is that firing again is never
    /// blocked.
    pub hook_retract_speed_m_s: f32,
    /// **⚠️ UNTUNED, and read by nothing yet** — `W3` (`vector::aim`) and `W2` (the wheel)
    /// consume it. Half-angle between the two arms' aim rays, in degrees off the look
    /// direction; degrees in the RON, radians in the code (`docs/conventions.md`).
    ///
    /// The user, 2026-08-12: *„und es muss mehr rechts und links spreaden!! (mit mausrad soll
    /// man einstellen können wie weit auseinander es gehen darf!)"* — so this is the **starting
    /// value of a number the player then sets at runtime**, and the next three keys are the
    /// window he sets it in. ⚠️ The wheel carries the **absolute** angle, never a delta: a
    /// delta desyncs over the network and never re-converges (`docs/multiplayer.md`).
    pub aim_spread_deg: f32,
    /// Floor of the wheel's window. **Strictly above 0** — at 0 both arms share one ray again,
    /// which is the state `F-023` exists to end (`docs/FINDINGS.md` FIND-039).
    pub aim_spread_min_deg: f32,
    /// Ceiling of the wheel's window. **At most 60°**: past that the side ray leaves the
    /// horizontal frustum and the marker the user asks for would be drawn off-screen.
    pub aim_spread_max_deg: f32,
    /// One notch of the wheel, in degrees. `(max - min) / step` has to be at least 8 notches,
    /// or the wheel is a three-position switch a player reads as broken.
    pub aim_spread_step_deg: f32,
    /// **The hard floor of the DYNAMIC angle, below the wheel's own floor** (`F-023`).
    ///
    /// The wheel's window is what the player is allowed to ask for; this is the narrowest the
    /// game may resolve to on his behalf. Strictly above `0` — at 0 both arms fire along one
    /// ray again, the state `F-023` exists to end (`docs/FINDINGS.md` FIND-039) — and strictly
    /// **below** [`VectorTuning::aim_spread_min_deg`], or the model can never narrow past the
    /// wheel and the whole feature is dead.
    pub aim_spread_floor_deg: f32,
    /// How fast the effective half-angle may change, in degrees per second — the outer safety
    /// clamp on a single-tick depth blip, on top of the distance filter below.
    pub aim_spread_slew_deg_s: f32,
    /// Time constant of the low-pass on **log2 of the aim distance**, in seconds.
    ///
    /// Log space and not metres: the angle is a function of `1/d`, so a constant relative rate
    /// behaves the same at 12 m and at 300 m, where a constant metric rate does not.
    pub aim_spread_settle_s: f32,
    /// The wheel notch at which the metre targets below apply **unscaled**, in degrees.
    ///
    /// Separate from [`VectorTuning::aim_spread_deg`] on purpose: that key has one job, the
    /// value the wheel starts at. This one is the scale anchor, `k = wheel_deg / this`. They
    /// happen to be the same number today and nothing requires them to stay so.
    pub aim_sep_neutral_deg: f32,
    /// The smallest separation that is still **two** anchors, in metres. Below one house
    /// frontage both arms are on the same facade, which is FIND-039 again.
    pub aim_sep_floor_m: f32,
    /// Target separation while a rope holds, in metres. Mid-swing the second hook is a
    /// **chain**, near your line — the bible's traversal tech is hook switching, not holding
    /// two wide anchors (`docs/gameplay/references.md` §5).
    pub aim_sep_tether_m: f32,
    /// Target separation on the ground or on a wall, in metres: standing still and picking a
    /// route, the two rays land on opposite edges of the block in front of you.
    pub aim_sep_stand_m: f32,
    /// Target separation airborne and untethered, in metres — one block **pitch**: falling
    /// with nothing attached, the two rays may not both land on the same block.
    pub aim_sep_search_m: f32,
    /// **The distance at which the metre budget above is fully available, in metres.**
    ///
    /// Nearer than this the budget scales with `d / this`, which makes the near field a
    /// constant angle per state instead of a block-scale nonsense the wheel's ceiling has to
    /// catch. Without it the whole metre model is a measured no-op under ~38 m — the range at
    /// which every flight's first hook is fired (`docs/FINDINGS.md` FIND-096).
    pub aim_sep_full_reach_m: f32,
    /// At or below this **horizontal** speed the state target applies in full, in m/s.
    pub aim_sep_calm_speed_m_s: f32,
    /// At or above this **horizontal** speed the target is pinned to
    /// [`VectorTuning::aim_sep_floor_m`], in m/s.
    pub aim_sep_fast_speed_m_s: f32,

    // -----------------------------------------------------------------------------------
    // `F-024` / `F-025` — the anchor candidate system. Every weight below is the backlog's
    // own number (`docs/backlog/gameplay.ron`, F-025), and it lives here so that the user can
    // retune the feel without a rebuild — which is the whole reason he asked for the two
    // sliders (*„damit ich testen kann was am besten wäre"*).
    // -----------------------------------------------------------------------------------
    /// How many rings of probe rays the candidate sweep casts **per hemisphere**.
    ///
    /// The sweep is the candidate query, and it is a BVH walk and not an iteration over the
    /// world (§6 rule 6): `rings * probes_per_ring` extra `SpatialQuery::cast_ray` calls per
    /// hemisphere, at the 0.21 us per ray `vector::aim`'s header measured. **Only cast while
    /// the assist is on** — at 0 % the game casts exactly the three rays it always did.
    pub assist_probe_rings: u32,
    /// How many probes sit on one ring, per hemisphere. Together with
    /// [`VectorTuning::assist_probe_rings`] this is the resolution of the candidate set: 2x4
    /// is 8 probes a side, 16 extra rays for a player, ~3.4 us a tick.
    pub assist_probes_per_ring: u32,
    /// `F-025`: *"Winkelabweichung zum Fadenkreuz (Hauptgewicht 45 Prozent)"*.
    pub assist_score_angle_w: f32,
    /// `F-025`: *"Momentum-Erhalt (25 Prozent, bevorzugt Punkte, die die aktuelle Flugbahn
    /// fortsetzen statt sie zu bremsen)"*.
    pub assist_score_momentum_w: f32,
    /// `F-025`: *"Hoehenvorteil relativ zur Bewegungsrichtung (15 Prozent)"*.
    pub assist_score_height_w: f32,
    /// `F-025`: *"Distanz im nutzbaren Mittelbereich (10 Prozent)"*.
    pub assist_score_distance_w: f32,
    /// `F-025`: *"Abwertung des zuletzt genutzten Punktes (5 Prozent, verhindert Pendeln
    /// zwischen zwei Punkten)"* — subtracted, not added.
    pub assist_score_recent_w: f32,
    /// Below this speed, in m/s, the momentum term carries no information and scores the
    /// midpoint of its own axis. A standing player has no trajectory to preserve.
    pub assist_momentum_min_speed_m_s: f32,
    /// The height difference, in metres, at which the height term saturates.
    pub assist_height_full_m: f32,
    /// The centre of `F-025`'s *"nutzbarer Mittelbereich"*, in metres.
    pub assist_dist_ideal_m: f32,
    /// Half the width of that band, in metres: at `ideal +/- this` the distance term is 0.
    pub assist_dist_span_m: f32,
    /// **How much better a candidate has to be than the point you are actually aiming at,
    /// at the very lowest assist strength.** The required margin is
    /// `this * (1 - strength_pct / 100)`, so 100 % strength is `F-024`'s SNAP mode (the best
    /// candidate always wins) and anything in between is ASSISTIERT. 0 % never reaches this
    /// line at all — `PlayerSettings::assist_is_on` is false and the free ray is the whole
    /// answer, which is FREI and `F-002`'s guarantee.
    pub assist_margin_full: f32,
    pub reel_speed_m_s: f32,
    pub min_rope_m: f32,
    /// Gauss-Seidel iterations over both rope constraints (`shared::rope::rope_step`).
    pub rope_iterations: u32,
    pub gas_tank: f32,
    pub gas_boost_per_s: f32,
    pub gas_reel_per_s: f32,
    /// **⚠️ UNTUNED, and nothing spends it yet.** What the rope-pull thrust
    /// ([`PlayerTuning::air_pull_m_s2`] / [`PlayerTuning::air_lateral_m_s2`]) will cost per
    /// second once `W4` bills it in `vector::gas`.
    ///
    /// It exists because that thrust would otherwise be **free**, which every one of the nine
    /// judges of the `docs/NEXT.md` §1B plan named as its biggest flaw independently. The
    /// number is solved, not chosen, out of the same ratio that decides
    /// [`gas_dodge`](Self::gas_dodge) — **gas per m/s of speed bought**:
    /// `gas_steer_per_s / air_pull_m_s2` = 16/30 = 0.533 against the held boost's
    /// `gas_boost_per_s / boost_m_s2` = 18/34 = 0.529. The two thrusts cost the same per metre
    /// per second on purpose, and `tests/data.rs` pins the difference at `<= 0.15`.
    ///
    /// Spent since 2026-08-13 by [`GasConsumer::Steer`], booked in `vector::gas::book` and read
    /// by `player::locomotion::air_control` — a rope term that costs nothing was what the nine
    /// judges refused, and it does not.
    pub gas_steer_per_s: f32,
    /// `F-008`. **⚠️ UNTUNED.** What **one** dodge costs — a flat amount, not a rate, and the
    /// only gas number in this block without a `_per_s`. That is the whole difference between
    /// the two boosts the user asked for (`docs/NEXT.md` §1c): `Shift` bills a rate for as long
    /// as you hold it, the dodge bills once for one impulse.
    ///
    /// **The comparison that makes `gas_boost_per_s` the cheap one is per m/s of speed bought,
    /// not per second**: the dodge pays `gas_dodge / dodge_impulse_m_s`, the held boost pays
    /// `gas_boost_per_s / boost_m_s2`. `tests/vector_boost.rs` holds the ratio, and the numbers
    /// and the argument stand in `assets/data/game.ron`.
    pub gas_dodge: f32,
    // ⚠️ **There is deliberately no `gas_regen_per_s` / `gas_regen_delay_s` here** — the tank
    // has no regeneration rate, because gas refills only at the stations of the main building
    // (`docs/QUESTIONS.md` Q-033, the user on 2026-08-12). `deny_unknown_fields` above is what
    // makes that stick: a `game.ron` that still carries either key now **crashes on load**
    // instead of quietly being ignored, which is the behaviour §4 asks for and the reason this
    // note sits in the struct rather than only in the file.
    /// Who pays first when the tank does not cover both. **A game-value decision**, which is
    /// why it stands here and not as an `if` in `vector/gas.rs`.
    pub gas_priority: Vec<GasConsumer>,
    pub boost_m_s2: f32,
    /// How much of the boost **direction** is taken from the rope instead of from the look
    /// direction (`F-007`, user 2026-08-10). `0.0` = pure look direction, `1.0` = straight at
    /// the anchor. Dimensionless, so no unit suffix — `_fraction` like
    /// [`TitanScale::cortex_fraction`] and `maps.ron: layout.anchorable_fraction`.
    ///
    /// **⚠️ UNTUNED.** It changes only the direction; the strength stays
    /// [`boost_m_s2`](Self::boost_m_s2) at every value, and `tests/vector_boost.rs` holds that.
    /// No `serde(default)`: at `0.0` this is the old behaviour, and that has to be a decision
    /// somebody wrote into the file, not a value nobody noticed was missing.
    pub boost_rope_fraction: f32,
    /// `F-008`. **⚠️ UNTUNED.** How much speed **one** dodge adds, in m/s — a velocity change,
    /// not an acceleration and not a force.
    ///
    /// **m/s and not m/s² on purpose.** `vector::boost::gas_boost` divides it by the fixed
    /// timestep to get the acceleration avian wants, so the number in the file is what a player
    /// can actually check against `max_speed_m_s` and against a swing (17–21 m/s, `Q-018`) —
    /// and it stays the same speed if `simulation_hz` ever moves. Mass never enters it, for the
    /// reason [`boost_m_s2`](Self::boost_m_s2) is an acceleration.
    pub dodge_impulse_m_s: f32,
    /// `F-008`. **⚠️ UNTUNED.** How many **ticks** may lie between the two `Space` presses for
    /// them to count as one double-tap (`net::local::read_input`).
    ///
    /// **Ticks and not seconds, because the tick counter is the clock that measures it.**
    /// `Time<Virtual>` is not usable here — it is what `--ticks` and the seeded rng are held
    /// steady against — and a window in seconds would have to be divided back into ticks with
    /// a rounding nobody can see. The conversion belongs in the comment, not in the code:
    /// `18` is 0.300 s at `simulation_hz: 60`.
    pub dodge_double_tap_window_ticks: u64,
    pub max_speed_m_s: f32,
}

/// Who spends gas. Its own type instead of `String`, so that a typo in the RON **crashes at
/// load time** instead of silently losing a consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum GasConsumer {
    /// `F-007` gas boost.
    Boost,
    /// `F-005` reel-in.
    ReelIn,
    /// `F-006` rope steering — the `W`/`A`/`D` thrust that an **anchored** rope adds on top of
    /// the free-air control (`docs/NEXT.md` §1B, `player::locomotion::rope_steer`). A rate,
    /// billed at [`VectorTuning::gas_steer_per_s`] per second for as long as a hook holds and a
    /// movement key is down. **Not** the free-air control itself: that one is never gated on gas,
    /// it only halves (`PlayerTuning::air_accel_empty_fraction`).
    Steer,
    /// `F-008` dodge. **The one consumer that is not a rate** — it bills
    /// [`gas_dodge`](VectorTuning::gas_dodge) once, on the tick the double-tap lands, and
    /// nothing on any other tick.
    Dodge,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraTuning {
    /// Field of view in **ground combat** — the base, not the ceiling. Has to lie inside the
    /// window `scale.ron: camera.min_ground_fov_deg ..= max_ground_fov_deg`.
    pub fov_deg: f32,
    /// Field of view at `vector.max_speed_m_s`. `F-017` will interpolate between the two
    /// later; the **curve** belongs in the code, the two **ends** belong in the RON.
    pub fov_max_speed_deg: f32,
    pub mouse_deg_per_px: f32,
    pub pitch_limit_deg: f32,
    pub smoothing_half_life_s: f32,
}

/// What the world itself costs: spatial index and collision.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldTuning {
    /// Edge length of one grid cell (`T-036a`). **Unmeasured** —
    /// `docs/lessons/performance.md`, `docs/QUESTIONS.md` Q-014. (Stood here as Q-013 until
    /// 2026-08-09; Q-013 is the question about the maximum rope length.)
    pub cell_m: f32,
    /// Half the edge length of the grid. Has to cover the map **and** the hook range.
    pub half_extent_m: f32,
    /// How many occupied cells it takes before a body goes into the linear large-body list.
    pub large_body_cells: u32,
    /// The thinnest wall allowed. Calibrates [`PlayerTuning::max_substep_m`].
    pub min_wall_m: f32,
    /// Collision margin against jitter in contact.
    pub collision_margin_m: f32,
}

// ---------------------------------------------------------------------------
// maps.ron — the city as data (E13, F-003, Q-010)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Maps {
    /// Logical name of the map that is built at startup.
    pub current: String,
    /// The one palette, linear RGB. No RGB triple per block — or sooner or later a signal
    /// color slips in (`docs/conventions.md`).
    pub palette: BTreeMap<String, (f32, f32, f32)>,
    /// The three signal colors — cyan, amber, crimson — linear RGB.
    ///
    /// **Their own map, apart from [`Self::palette`], and that is the point.** The rule from
    /// `docs/conventions.md` §3 is that these three appear nowhere else: no cyan set dressing,
    /// no amber lanterns, no red roofs. Split in two, that rule becomes a test — no map block
    /// may name a key from here. In one map it would be a sentence in a document again.
    ///
    /// Until 2026-08-09 they existed as prose and as a number nowhere, so there was nothing to
    /// paint the cortex with.
    pub signals: BTreeMap<String, (f32, f32, f32)>,
    pub maps: BTreeMap<String, Map>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Map {
    pub name: String,
    /// Edge length in X and Z, in meters.
    pub size_m: (f32, f32),
    /// Seed for `shared::Rng`. Part of the state, **never** `rand::random()`.
    pub seed: u64,
    pub layout: Layout,
    /// How the ground under the layout is stepped.
    ///
    /// **Explicit, never defaulted** (§4): a map that forgets the key fails to load. A map
    /// that wants no terrain writes `levels: 1, step_m: 0.0` and says so — which is exactly
    /// what `graybox` does, and it has to, because eight tests in `tests/vector_aiming.rs`
    /// and four in `tests/player.rs` reason about `y = 0` on that fixture.
    pub terrain: Terrain,
    /// Explicitly placed boxes. They beat the generated layout.
    pub blocks: Vec<MapBlock>,
    /// The lamps that hang **inside** this map's buildings.
    ///
    /// **Explicit, never defaulted** (§4, and `#[serde(default)]` is forbidden for game
    /// values): a map that forgets the key fails to load. A map with no interiors writes
    /// `lights: []` and says so — an empty list is a statement, a missing field is a
    /// silence that looks exactly like a bug when the room turns out dark.
    ///
    /// Why it is per **map** and not per `art.ron`: a lamp is part of a building, it stands
    /// at a coordinate, and `docs/FINDINGS.md` FIND-078 measured why the global fill cannot
    /// stand in for it — ambient has no direction, so 5x the brightness buys 5.8 sRGB levels
    /// and costs the exterior its shadows 1:1, and the boxes in the room stay flat
    /// rectangles because nothing gives them a lit face and a dark one.
    pub lights: Vec<MapLight>,
}

/// The stepped ground under the district — every number the plateaus and their stairs need.
///
/// The user, 2026-08-13: *„adde verschiedene höhen vom boden her! lass es wie die echte stadt
/// aussehen! aktuell kann man es noch nicht erkennen!"*. Until that day the ground was one
/// 700 x 700 m slab at `y = -0.1` and every house in the district stood on `y = 0`.
///
/// The generator is [`shared::TerrainField`](crate::shared::TerrainField); what stands here is
/// only the shape of the result. **Everything is a length or a count** — the code holds the
/// mechanics and not one of these figures (`docs/conventions.md` §4).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terrain {
    /// Edge of one terrain cell. **An exact multiple of `Layout::lot_m + Layout::street_m`**,
    /// and `world::map` asserts it: the cell boundary then always falls in the middle of a
    /// street and never through a house, which is the only reason the terraces can have a
    /// cliff at all.
    pub cell_m: f32,
    /// How much one level is worth. `0.0` together with `levels: 1` means "no terrain".
    pub step_m: f32,
    /// How many levels there are, the flat ground counted. `1` = flat.
    pub levels: u32,
    /// The radius around the origin that stays at level 0. The player spawns there, and a
    /// terrace under a spawn point is a player standing inside the ground.
    /// `>= Layout::clear_radius_m`, or the pads reach into the space the layout keeps free.
    pub flat_radius_m: f32,
    /// One riser of the stairs that lead up a terrace. `step_m` has to be a whole multiple of
    /// it, or the last step of a flight does not land on the plateau.
    pub stair_rise_m: f32,
    /// One tread. Bounded from above by `street_m / 2 - cell_jitter_m`, because the flight is
    /// cut into the terrace's own edge and must not undercut the house standing on it —
    /// `world::map` asserts that arithmetic rather than clamping it.
    pub stair_tread_m: f32,
    /// Anything hand-placed in `Map::blocks` whose top is **at or below** this is ground —
    /// paving, the slab itself — and a terrace may cover it. Anything **above** it is a
    /// structure, and its cell is pinned flat: a terrace cannot bury a door, a market stall or
    /// a quay wall. It is also where a pad's underside sits, so the pad touches the paving
    /// instead of intersecting it.
    pub paving_top_m: f32,
    /// How far a terrace stops short of a **pillar** it is cut around.
    ///
    /// A hand-placed block that stands on the ground and reaches a door's height above the
    /// terrain's ceiling — a tree, a bell tower, a gantry leg, the church — does not pin its
    /// cell flat: the terrace is cut around its footprint and the pillar itself plugs the hole,
    /// because it is solid from the ground up. Without a gap the cut edge would sit exactly on
    /// the pillar's face, and `world::map::overlaps` — two different float sums of the same
    /// algebra — would decide `tests/world.rs::f003_no_grid_house_stands_inside_a_placed_block`
    /// by one ULP. One centimetre is invisible and a 0.35 m capsule cannot fall into it.
    pub pillar_gap_m: f32,
    /// The colour key of the terrace top, out of [`Maps::palette`].
    pub color: String,
    /// The colour key of the stairs. Its own key, and not the terrace's: a flight you cannot
    /// tell from the plateau is a step the player walks into.
    pub stair_color: String,
}

/// The rule `world` deterministically generates buildings from.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    /// Edge of one grid cell's BUILT area. With `perimeter: None` that is the footprint of
    /// the single house on it; with `Some` it is the outer edge of the closed block.
    pub lot_m: f32,
    pub street_m: f32,
    pub min_height_m: f32,
    pub max_height_m: f32,
    /// Fraction of lots that get built on, 0..1.
    pub density: f32,
    /// Fraction of anchorable buildings, 0..1.
    pub anchorable_fraction: f32,
    /// Radius around the origin that stays clear.
    pub clear_radius_m: f32,
    /// How a built cell is filled.
    ///
    /// `None` — one detached box that fills the whole lot. That is a suburb: the gap between
    /// two houses is `street_m` in one direction and `street_m` in the other, and every lot
    /// is an island.
    ///
    /// `Some` — a **closed block perimeter**: a ring of touching row houses around a
    /// courtyard, which is how a walled old town is actually built (the reference survey
    /// measures party walls, 12-14 m frontages and 39 % ground coverage). The street then is
    /// the gap between two rings and it is `street_m` wide, facade to facade, everywhere.
    ///
    /// **Explicit, never defaulted** (`docs/conventions.md` §4): a map that forgets to say
    /// which of the two it is does not silently become a suburb, it fails to load.
    pub perimeter: Option<Perimeter>,
    /// Allowed color keys out of [`Maps::palette`].
    pub colors: Vec<String>,
    /// Into how many stacked, progressively pulled-in caps the roof rise is cut.
    ///
    /// `1` is the flat cap the district had until 2026-08-13. Above that the roof reads as a
    /// **stepped pitch** from the air, which is the half of *„aktuell kann man es noch nicht
    /// erkennen"* the ground cannot answer: a roofscape of equal flat lids is a mosaic
    /// whatever the ridge heights do. `2 * Roof::inset_fraction * roof_steps < 1`, or the top
    /// step has no extent left — `world::map` asserts it.
    pub roof_steps: u32,
    /// The fraction of generated houses that are built to [`Self::tall_height_key`] instead of
    /// out of `min_height_m..max_height_m`, 0..1.
    ///
    /// **The answer to `Q-036`**, which named the gap: `scale.ron: architecture.heights_m`
    /// stopped at 11.5 m for a house and the next entry was the 35 m church, so a district
    /// built only out of the residential band has 5 m of spread over 700 m of town and reads
    /// flat from anywhere. A few per cent of a taller class is what gives the block means
    /// something to differ by.
    pub tall_fraction: f32,
    /// Which key of `scale.ron: architecture.heights_m` a tall house is built to. A name and
    /// not a number, so the height itself stays where every other height of this world lives.
    pub tall_height_key: String,
    /// **How far this district has fallen** — or `None` for a district that still stands.
    ///
    /// `docs/gameplay/world.md`: *"the war is already lost … Ashgate has long since fallen;
    /// the Vanguard runs salvage missions into its own ruins"*. Until 2026-08-19 the
    /// generator had no notion of damage at all and built that ring as an intact market
    /// town, which is what the user saw: *„das ist nicht die echte map!"*.
    ///
    /// **Explicit, never defaulted** (`docs/conventions.md` §4): the `graybox` fixture writes
    /// `damage: None` and means it — eight aiming tests in `tests/vector_aiming.rs` are
    /// pinned to boxes on that map and a ruin would move them.
    pub damage: Option<Damage>,
}

/// **The fall of a district, as a distribution.** Every number is a fraction, a metre or a
/// palette key; which model a given ruin wears is `world::map`'s kit (a measurement of the
/// files, not a tuning number), and *whether* a given house wears one is decided here.
///
/// ## Why a gradient and not a rate
///
/// A fallen ring is not uniformly destroyed, and a district that is damaged at one flat rate
/// everywhere reads as a **texture** over a market town rather than as a history. So the
/// severity of a house is three terms added together:
///
/// 1. the **gradient** — [`Self::core_severity`] at the origin rising to
///    [`Self::edge_severity`] at the map edge. The Vanguard held the middle longest and the
///    outer edge is where the wall went;
/// 2. a draw per **block** ([`Self::block_spread`]) — that is what makes whole streets stand
///    and whole streets fall, which is the thing worth flying through;
/// 3. a draw per **house** ([`Self::house_spread`]) — so that a standing row still has gaps
///    in it and a razed one still has a gable left.
///
/// The same shape as `Perimeter::house_spread_m`, and for the same reason: one draw per house
/// over the whole window is white noise, and white noise averages out over a hundred metres.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Damage {
    /// Severity at the origin, 0..1. The player spawns here.
    pub core_severity: f32,
    /// Severity at the map edge, 0..1. Above `core_severity`, or there is no gradient — and
    /// `tests/world.rs::f003_the_damage_is_a_gradient_and_the_core_is_what_is_left` measures
    /// the district that comes out, not this pair.
    pub edge_severity: f32,
    /// How far a whole block may sit off the gradient, drawn per **lot**.
    pub block_spread: f32,
    /// How far one house may sit off its block, drawn per **house**.
    pub house_spread: f32,
    /// Severity from which a house is a standing ruin instead of a house.
    pub ruin_at: f32,
    /// Severity from which it has collapsed altogether. `> ruin_at`.
    pub rubble_at: f32,
    /// The tallest a ruin may stand, as a fraction of the ridge the intact house would have
    /// had. The remnant is **cut out of** the house, never added to it — nothing in a fallen
    /// district is taller than it was standing.
    pub ruin_height_fraction: f32,
    /// Below this a remnant is not a ruin but a mound: a ruin model that cannot reach this
    /// height inside the lot is not used, and the house collapses instead.
    pub ruin_min_height_m: f32,
    /// How tall a rubble mound is drawn, in metres — `(min, max)`.
    pub rubble_height_m: (f32, f32),
    /// **How far a collapsed house spills past its own frontage line into the road.**
    ///
    /// This is the one number in this block that is about traversal and not about looks: a
    /// mound is under 3 m in a 6 m street, so it takes the **ground** away and leaves the air
    /// alone. What changes the swing lane is the ruin beside it, which is a 4 m stump where a
    /// 9 m wall used to hold a rope.
    pub spill_m: f32,
    /// Palette key for a standing remnant, and for a mound. Only visible where no model is
    /// worn (`art.ron` on `Primitive`) — the mesh covers the box everywhere else.
    pub ruin_color: String,
    pub rubble_color: String,
}

/// The closed block: how one grid cell is turned into a ring of houses.
///
/// The courtyard is `lot_m - 2 * wing_depth_m` on a side and it is a **gap between blocks**,
/// not a hole cut into one — this world has no subtraction (`docs/FINDINGS.md` FIND-056).
///
/// ## Everything below `wing_depth_m` exists because the first version of this ring was
/// ## judged and failed
///
/// The user, 2026-08-12: *„häuser sind alle ineinander! keine unterschiedliche höhen! es
/// sieht überhaupt nicht aus wie eine attack on titan map! viel zu kompakt!"* — a ring of
/// identically wide, identically deep, identically tall houses with **zero** gap between
/// them, repeated on a perfect square grid, reads as one merged mass with a flat top. Every
/// number here is one axis of irregularity, and every one of them is drawn from the seed:
/// per **cell** (`cell_jitter_m`, `house_spread_m`), per **wing** (`frontage_spread_m`) or
/// per **house** (`gap_fraction`, `setback_max_m`, `depth_spread_m`, `roof`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Perimeter {
    /// Target width of one row house along the street. The run is divided into whole houses,
    /// so the built width is the nearest divisor of the run — never a gap.
    pub frontage_m: f32,
    /// How deep the ring is. `2 * wing_depth_m < lot_m`, or there is no courtyard left.
    pub wing_depth_m: f32,
    /// Full width of the window the frontage is drawn from, per **wing**: one run is divided
    /// into houses of `frontage_m ± frontage_spread_m / 2`. Two runs facing each other then
    /// no longer have the same number of houses at the same joints.
    pub frontage_spread_m: f32,
    /// The alley, as a fraction of one slot, drawn per **house**: `0` is a party wall,
    /// `gap_fraction` is the widest gap. A fraction and not a metre so that a narrow flank
    /// house cannot be given a gap wider than itself.
    pub gap_fraction: f32,
    /// How far a house may stand back from the street edge, drawn per **house**. This is what
    /// makes the frontage line ragged instead of flush.
    pub setback_max_m: f32,
    /// How much depth a house may lose behind the frontage, drawn per **house**.
    /// `setback_max_m + depth_spread_m < wing_depth_m`, or a house would have no depth left.
    pub depth_spread_m: f32,
    /// How far the whole ring may be shifted in x and z against its grid cell, drawn per
    /// **cell**. This is the only thing that breaks the perfect orthogonal grid — no block is
    /// ever rotated (`world::map`), so a wobbling street is the available substitute.
    /// `2 * cell_jitter_m < street_m`, or two rings grow into each other.
    pub cell_jitter_m: f32,
    /// How much the houses of **one** block differ in height. The rest of
    /// `min_height_m..max_height_m` is the block's own level, drawn per cell: houses next to
    /// each other differ a little, quarters next to each other differ a lot. One draw over
    /// the whole window instead gives white noise, which from a distance is a flat average —
    /// exactly the "keine unterschiedliche höhen" the user saw.
    pub house_spread_m: f32,
    /// The pitched cap on top of the walls. `None` = flat roofs.
    pub roof: Option<Roof>,
}

/// The roof, as the one block a box world can build it from: a smaller cuboid on the eaves.
///
/// `Layout::min_height_m..max_height_m` stays the **ridge** — the total height of the house,
/// the number `scale.ron: architecture.heights_m` names and `tests/data.rs` holds to the
/// residential band. The roof is cut **out of** it, not added on top: eaves = ridge - rise.
/// Nothing gets taller because the houses grew roofs.
///
/// The rise is a **fraction of the house**, because that is the shape the user's own numbers
/// have: `scale.ron: architecture.eaves_m` gives 3.0 m of eaves on a 4.5 m house (rise 1/3)
/// and 6.0 m on an 8.0 m house (rise 1/4). A fixed rise in metres would put a 1/3 roof on a
/// small house and a 1/8 roof on a large one.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roof {
    pub min_rise_fraction: f32,
    pub max_rise_fraction: f32,
    /// How far the cap is pulled in on each side, as a fraction of the house's footprint.
    /// `< 0.5`, or the cap has no extent. The remainder is the ledge you land on.
    pub inset_fraction: f32,
    /// Its color key out of [`Maps::palette`] — the roofscape is the route, and a route the
    /// player cannot tell apart from the wall below it is not a route.
    pub color: String,
}

/// One explicitly placed box. Origin at the center, like `shared::Block`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapBlock {
    pub center_m: (f32, f32, f32),
    /// Full edge length. The index keeps the half internally.
    pub size_m: (f32, f32, f32),
    pub color: String,
    /// Gets `shared::AnchorSurface` and `BodyMask::ANCHORABLE` (`F-003`).
    pub anchorable: bool,
    /// Stops a body — `BodyMask::SOLID`.
    pub solid: bool,
    /// `false` = housing, and it has to stay below `scale.ron: architecture.heights_m`
    /// `house_large` (11.5 m). `true` = church, watchtower, tree, wall — **they carry the
    /// vertical** and are allowed above it.
    ///
    /// Without this field the flatness rule only held for `layout`, and the explicitly placed
    /// boxes of the graybox stood at 12, 14 and 18 m ridge height outside the guard that is
    /// supposed to keep the city flat.
    pub landmark: bool,
}

/// One lamp inside a building — spawned by `render::light::setup_interior_lights` as a
/// **`PointLight`**, placed by hand exactly like a [`MapBlock`].
///
/// ## Why a point light and not a spot
///
/// A hall lamp hangs under a roof and throws light in every direction; that IS a point light.
/// A [`SpotLight`](bevy::light::SpotLight) would need an aim vector and two cone angles —
/// three more numbers per lamp that can be got wrong — and to cover a 29 x 23 m hall from
/// 8.5 m up its outer angle would have to be near-hemispherical, at which point it is a point
/// light with extra fields. A spot is the right shape when a pool of light on **one** object
/// is wanted; here the whole room has to become readable.
///
/// ## Why a lamp inside a room does not brighten the world
///
/// Bevy's point light is clustered forward and unoccluded: with [`Self::shadows`] off it
/// reaches every fragment inside [`Self::range_m`], wall or no wall. That is safe here and it
/// is Lambert that makes it safe — **the outer face of the wall a lamp stands behind points
/// away from that lamp**, so `NdotL <= 0` and it receives nothing however long the range. The
/// only faces that could leak are the ones pointing **up** (street, aprons, quays) and the way
/// to keep those out is the range: `attenuation = (1 - (d/r)^4)^2 / d^2`
/// (`bevy_pbr .. lighting.wgsl`) is a **hard** cut-off at `d = r`, not an asymptote.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapLight {
    /// Where the lamp hangs, in metres.
    pub center_m: (f32, f32, f32),
    /// A key from [`Maps::palette`] — **not** an RGB triple, and for the same reason a block
    /// may not carry one: the moment a light may name its own colour, one of the three signal
    /// colours becomes an amber lantern (`docs/conventions.md` §3).
    pub color: String,
    /// Luminous power in **lumens** — Bevy divides by `4*pi` to get candela
    /// (`bevy_pbr-0.19.0/src/render/light.rs:530`).
    ///
    /// ⚠️ **The numbers here are in the tens of millions and that is not a typo.**
    /// `art.ron: exposure_ev100 = 12.85` is stopped down for a 52 000 lux sun, and under that
    /// exposure a real 1 500 lm bulb delivers 1.2 lux at 10 m — five stops under a shadow. A
    /// lamp that is *legible* against a sunlit street has to be a floodlight; a lamp that is
    /// photometrically honest leaves the room exactly as unreadable as FIND-078 found it.
    pub intensity_lm: f32,
    /// Hard cut-off in metres. Nothing outside it is touched at all, which is what keeps a
    /// lamp's effect inside its own building — see the type docs.
    pub range_m: f32,
    /// **The expensive switch** (`docs/lessons/performance.md` rule 5). A shadow-casting point
    /// light costs a **cube** map — six depth passes over everything in range, per light, per
    /// frame — where the sun costs one pass per cascade. Never switched on without a measured
    /// number.
    pub shadows: bool,
}

// ---------------------------------------------------------------------------
// gear.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gear {
    pub blades: BladeTuning,
    pub resupply: ResupplyTuning,
    pub feel: FeelTuning,
}

/// `F-034` hit stop and camera kick. **⚠️ UNTUNED — `F-034` had no numbers at all anywhere in
/// the repository before 2026-08-09.**
///
/// **Seconds in the file, ticks in the code.** The hit stop is counted in fixed ticks and never
/// against a clock ([`crate::shared::HitStop`]): `Time<Virtual>::set_relative_speed` would slow
/// the tick rate itself, and the tick carries the rng seed. The conversion
/// `round(s * simulation_hz)` happens once, at the boundary.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeelTuning {
    /// The lethal hit. A husk cortex is crossed in 36.7 ms at 30 m/s — 2.2 frames — so
    /// without this the player never sees the kill that the whole game is built around.
    pub hit_stop_cortex_s: f32,
    pub hit_stop_normal_s: f32,
    pub camera_kick_deg: f32,
    pub camera_kick_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BladeTuning {
    pub start_pairs: u8,
    /// What one **reported** hit costs the pair in the harness, in sharpness. The cortex cut is
    /// this number; a graze is this number times [`Self::wear_torso_factor`].
    pub wear_per_hit: f32,
    /// The graze's share of [`Self::wear_per_hit`]. **Below 1.0 on purpose** — a pass that ends
    /// in a nape reports `[Torso, Cortex]` on every titan, because every titan is wider than his
    /// own neck. See the comment in `gear.ron`, which carries the argument.
    pub wear_torso_factor: f32,
    pub damage_per_m_s: f32,
    pub min_speed_m_s: f32,
    /// **Decides whether a cut lands at all**, and did not exist before 2026-08-09.
    /// **⚠️ UNTUNED**, like the five below it.
    pub reach_m: f32,
    /// Radius of the swept capsule. **Never zero** — a zero-radius sweep is a ray, and a ray
    /// between two ticks at 75 m/s threads a 0.46 m cortex.
    pub thickness_m: f32,
    pub swing_s: f32,
    /// The window inside the swing during which the blade actually cuts. Without it the blade
    /// cuts during its own wind-up, which is the shortcut that turns a slash into a hitbox.
    pub active_from_s: f32,
    pub active_to_s: f32,
    /// Without it the slash is autofire.
    pub cooldown_s: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResupplyTuning {
    pub gas_per_s: f32,
    pub range_m: f32,
    /// Whole blade pairs per second, handed back at a rack of the main building.
    ///
    /// Fractional, and the accumulator that turns it into whole pairs lives in
    /// [`crate::blades::resupply`] — `Blades::pairs_left` is a `u8` and a rate that is not a
    /// whole number per tick has to be carried somewhere.
    pub blade_pairs_per_s: f32,
    /// How fast the pair in the harness is honed back towards `sharpness == 1.0`.
    pub sharpen_per_s: f32,
}

// ---------------------------------------------------------------------------
// titan.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titans {
    pub kinds: BTreeMap<String, TitanKind>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanKind {
    /// Key in `scale.ron: titan.classes` (`F-064`). **No height per kind** — height and Cortex
    /// height come out of the scale table through [`GameData::titan_height_m`] and
    /// [`GameData::titan_cortex_height_m`], so that the two cannot drift apart.
    ///
    /// Was called `groesse` until 2026-08-09; a field name ending in "-size" reads like a
    /// length (`docs/conventions.md` §5), but what stands here is a key.
    pub size_class: String,
    pub speed_m_s: f32,
    pub cortex_radius_m: f32,
    pub regen_per_s: f32,
    /// Windup of every attack. **At least 0.4 s** — Bible, pillar P4 (readability before
    /// realism). `tests/data.rs` falls over when a kind drops below it.
    pub windup_s: f32,
    /// How fast the kind may turn, in degrees per second. **⚠️ UNTUNED, and the most
    /// important feel number in this file.**
    ///
    /// Without it a titan snaps to face the player, and the husk's entire lesson — "the
    /// fundamentals of the approach angle", Bible §4 — ceases to exist: there is no angle to
    /// approach from if he is always facing you.
    pub turn_deg_per_s: f32,
    /// Ground acceleration toward the target, m/s². `speed_m_s` is the ceiling, this is how
    /// fast he reaches it.
    pub accel_m_s2: f32,
    /// The second and third third of an attack. `windup_s` existed; these did not.
    ///
    /// **`recover_s` IS the punish window** — it is the whole reason a telegraphed attack is
    /// an opportunity and not just a warning.
    pub strike_s: f32,
    pub recover_s: f32,
    /// How close the target has to be before `Pursue → Windup` fires. Roughly arm reach:
    /// `arm_fraction × height + width/2`.
    pub attack_range_m: f32,
    /// **Half the width of the arc a blow can be landed in**, in degrees off the titan's own
    /// forward vector, measured on the ground plane. Together with `attack_range_m` it is the
    /// strike volume: a cone, not a cylinder.
    ///
    /// **This is what makes `turn_deg_per_s` a number and the nape a design.** Until `Q-031`
    /// was answered on 2026-08-13 the strike had no angle at all
    /// (`docs/FINDINGS.md` FIND-012) — a player in the titan's back took exactly what a player
    /// in his face took, and coming from behind bought nothing.
    ///
    /// ⚠️ **UNTUNED**, and uniform across all kinds on purpose: 55° is a guess that has never
    /// been played. Range `[30, 90]`, guarded by
    /// `tests/combat.rs::every_kind_carries_a_strike_half_angle_in_range` — under 30° a titan
    /// whiffs at a player standing straight in front of him, at 90° the cone is a half-space
    /// and the approach angle stops meaning anything again.
    pub strike_half_angle_deg: f32,
    pub attack_cooldown_s: f32,
    /// How close the target has to be before `Idle → Pursue` fires. Stands in for the whole
    /// perception model `F-051`, which is not built.
    pub aggro_radius_m: f32,
    /// How long `Death` lasts: the body scales to zero over this time. The collider is
    /// dropped on tick one regardless, so a corpse is never a wall.
    pub death_s: f32,
    /// **⚠️ UNTUNED, and until 2026-08-09 there was no titan health anywhere in the
    /// repository** — while `regen_per_s` was already regenerating it. Not needed for the
    /// cortex kill, which is a rule and not a threshold; needed the moment anything else
    /// does damage.
    pub health: f32,
    /// What one landed strike takes off the player. **⚠️ UNTUNED, and until 2026-08-09 it did
    /// not exist** — so a titan could wind up, strike and land, and nothing happened.
    ///
    /// Calibrated against `game.ron: player.health` (100): a husk needs **three** strikes.
    /// That is what makes reading a telegraphed wind-up worth more than tanking it.
    pub damage: f32,
    pub model: String,
}

// ---------------------------------------------------------------------------
// scale.ron — the one truth about sizes (laid down by the user, 2026-08-09)
// ---------------------------------------------------------------------------

/// The user's size table as a type.
///
/// **These numbers are not untuned, they are laid down.** Everything else in `assets/data/` is
/// anybody's to change; whoever changes something here changes a decision of the user's. The
/// precedence rule that goes with it: **a direct figure in meters from the user beats every
/// derivation** — including the conversion out of the backlog (`docs/QUESTIONS.md` Q-002).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scale {
    /// The world is **deliberately not scaled uniformly**: architecture 1.0, Titans 1.4, walls
    /// 2.4. A Titan reads bigger than "realistic", a wall monumental — that is the visual
    /// language, not sloppiness. Whoever levels the three destroys it.
    pub architecture_factor: f32,
    pub titan_factor: f32,
    pub wall_factor: f32,
    pub reference: Reference,
    pub architecture: ArchitectureSizes,
    pub titan: TitanScale,
    pub wall: WallSizes,
    pub camera: CameraScale,
    pub vector: VectorScale,
}

/// What the eye measures everything else against.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// "Check the capsule exactly!" — `game.ron: player.height_m` has to be **exactly** this.
    pub human_height_m: f32,
    pub door_height_m: f32,
    /// Window for `maps.ron: layout.street_m`. "Keep them tight."
    pub min_street_m: f32,
    pub max_street_m: f32,
    /// 1/7.5. The **yardstick** for [`TitanScale::min_head_fraction`]: the Titan head is
    /// relatively smaller, and that is exactly what makes the eye read "huge" instead of
    /// "close".
    pub human_head_fraction: f32,
}

/// Building heights. The housing is deliberately flat; the vertical comes from the wall, the
/// church, the watchtower and the trees.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureSizes {
    /// Logical name -> total height in meters.
    pub heights_m: BTreeMap<String, f32>,
    /// Logical name -> eaves height (top edge of the outer wall, where the roof starts).
    /// **Filled in only where the user named a number** — every key has to stand in
    /// [`Self::heights_m`] as well, and `tests/data.rs` checks that.
    pub eaves_m: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanScale {
    /// The fraction of the body height the Cortex sits at (0.89) — **a check rule, not a
    /// source.** The five Cortex heights stand as figures in meters in
    /// [`SizeClass::cortex_height_m`]; this value is what the user wrote next to them, and
    /// `tests/data.rs` uses it to hold the five inside the window 0.88..0.90.
    ///
    /// It serves as a source only where the user named no Cortex height — today exactly once:
    /// for the Ashwalker (150 m ⇒ 133.5 m).
    pub cortex_fraction: f32,
    /// Head height as a fraction of the body height: 1/10 to 1/9.
    pub min_head_fraction: f32,
    pub max_head_fraction: f32,
    /// Body width as a fraction of the body height. **⚠️ UNTUNED** — invented, not laid
    /// down, unlike the fields above it. Until 2026-08-09 nothing in the repository said how
    /// wide a titan is, so there was no collider and no answer to "does he fit through the
    /// alley".
    pub width_fraction: f32,
    /// The box rig, stacked as fractions of the body height. **⚠️ UNTUNED.**
    /// `leg_fraction + torso_fraction` has to equal [`Self::cortex_fraction`] — the cortex is
    /// the seam between torso and head. `tests/data.rs` holds that.
    pub leg_fraction: f32,
    pub torso_fraction: f32,
    /// Shoulder height as a fraction of the body height — **below** the cortex, or the arm
    /// hinges at the nape and cannot swing in front of the body.
    pub shoulder_height_fraction: f32,
    /// Arm length from the shoulder, as a fraction of the body height. Decides how far the
    /// hand travels in a wind-up, which is what `F-053`'s telegraph is measured on.
    pub arm_fraction: f32,
    /// Wind-up and strike pose angles in degrees (`F-053`). **⚠️ UNTUNED.**
    ///
    /// Properties of the **rig**, not of a kind — one rig, three numbers, instead of
    /// twenty-seven per-kind numbers nobody would ever tune. Degrees in the file, radians in
    /// the code; the conversion happens at the boundary (`docs/conventions.md`).
    pub windup_arm_deg: f32,
    pub windup_lean_deg: f32,
    /// Negative: the strike carries the arm back down past the rest pose.
    pub strike_arm_deg: f32,
    /// The largest class that may spawn this session. A key in [`Self::classes`].
    ///
    /// **A user decision made in his absence** (`docs/QUESTIONS.md` Q-028): `huge` is 5.25 m
    /// wide in a 7.0 m street and `boss` is exactly 7.00 m, which is not a tight alley but a
    /// silent wall. Taking the decision back is this one line in `scale.ron`.
    pub max_spawnable_class: String,
    /// The five size classes (`F-064`). `titan.ron` points here with `size_class`.
    pub classes: BTreeMap<String, SizeClass>,
    /// The 150 m boss. **Outside the classes**: not a scaled enemy kind but a structure with a
    /// face. The user's own check: 150 − 120 = 30 m above the wall.
    pub ashwalker_height_m: f32,
}

/// One size class. Its own type instead of a bare `f32`, so that later per-class values (range,
/// hit points, stride length) land here instead of being duplicated per kind.
///
/// **Both fields are figures in meters from the user** — `cortex_height_m` is not computed from
/// `height_m`. The fraction ([`TitanScale::cortex_fraction`]) checks them instead of producing
/// them.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeClass {
    pub height_m: f32,
    /// Height of the Cortex above the ground. The **only lethal weak point** (`F-030`).
    pub cortex_height_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallSizes {
    pub height_m: f32,
    pub top_thickness_m: f32,
    /// Larger than [`Self::top_thickness_m`]: the wall tapers toward the top.
    pub base_thickness_m: f32,
    /// A staging point at half height. Without it the crown of the wall cannot be reached from
    /// below with a 90 m anchor range (120 > 90).
    pub platform_height_m: f32,
    /// **The ladder of scale.** A 0.6 m stone course and a band every 15 m are the reason a
    /// 120 m wall looks big instead of gray.
    pub stone_course_m: f32,
    pub band_spacing_m: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraScale {
    /// Eye height. `game.ron: player.eye_height_m` has to be this.
    pub height_m: f32,
    /// Window for `game.ron: camera.fov_deg` — **ground combat**, "the biggest lever".
    pub min_ground_fov_deg: f32,
    pub max_ground_fov_deg: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorScale {
    /// 90 m, straight from the user. `game.ron: vector.hook_range_m` has to be this — and
    /// **not** the 112 m out of the conversion (Q-002).
    pub anchor_range_m: f32,
    /// "x1.5 vs. standard". **Is not applied to anything** as long as the reference is missing
    /// (`docs/QUESTIONS.md` Q-018).
    pub speed_factor: f32,
}

// ---------------------------------------------------------------------------
// art.ron — the registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Art {
    pub models: BTreeMap<String, Model>,
    /// How far a `cortex` anchor read out of a swapped model may sit away from the height
    /// `scale.ron` names for that size class before `render::model` shouts.
    ///
    /// **The number is here and not in Rust** (rule 2), and it is an *art-pipeline* tolerance,
    /// not a game value — `scale.ron` stays the one truth about where the cortex is; this only
    /// says how much sloppiness in a `.glb` still counts as "the modeler meant that spot".
    /// A model that misses it by more than this breaks `F-030`: the cut lands where the
    /// silhouette says and the kill zone is somewhere else.
    pub cortex_tolerance_m: f32,
    /// Sun, sky, fog and exposure — `render::light` reads nothing else.
    pub lighting: Lighting,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Where the geometry comes from. **The switch** (§7) — and the reason it is an enum and
    /// not a `bool` plus a path: "no model configured" has to be *expressible*, and a
    /// `serde(default)` path field would turn a typo into silence instead of a crash (§4).
    pub source: ModelSource,
    pub scale: f32,
    /// Set only on third-party material: URL · date · license · what it is meant to replace.
    /// That makes the replacement list a `grep` (§7).
    pub attribution: Option<String>,
    /// Game state (`idle`, `walk`, `windup`, `strike`) -> the name of the clip **inside the
    /// glTF file**. Empty is a legal answer and means "this model animates nothing".
    ///
    /// Resolved once at load; a name that is not in the file is a **loud warning plus that
    /// state having no clip**, never a silent substitute (`docs/models.md`, glTF traps).
    pub animations: BTreeMap<String, String>,
}

/// Primitive or file — the one decision that makes a swap one line.
///
/// [`ModelSource::Primitive`] is what the whole game is today: cuboids out of `maps.ron` and
/// out of the titan rig. **The game has to run with not a single `.glb` in the repository**,
/// which is why the absent case is a named variant and not a missing field.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ModelSource {
    /// Build the procedural placeholder out of Bevy primitives.
    Primitive,
    /// Load this file. The path is **relative to `assets/`** — that is the asset server's root,
    /// and it is written here so that no file name ever stands in Rust (§7,
    /// `tools/norms.py`).
    Gltf(String),
}

/// The sun, the sky and the fog — **every number of the look that is not a hue**.
///
/// The hues stay where they were (`maps.ron: palette` and `maps.ron: signals`); this is light
/// and depth. It lives in `art.ron` and not in Rust for the same reason a gas cost does
/// (rule 2), and it is one struct and not six loose fields because the values are only
/// meaningful **against each other**: `ambient.brightness` says nothing without
/// `sun.illuminance_lux` and `exposure_ev100`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub sun: Sun,
    pub ambient: Ambient,
    pub sky: Sky,
    pub fog: Fog,
    /// Camera exposure. The scale everything else is solved against — see `art.ron`.
    pub exposure_ev100: f32,
}

/// The one directional light. **Its position is spherical, not cartesian**, because that is
/// how a sun is actually chosen: you pick where it stands, not a point in metres.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sun {
    /// Linear RGB.
    pub color: (f32, f32, f32),
    pub illuminance_lux: f32,
    /// Compass degrees in **our yaw convention**: 0 = -Z, +90 = +X
    /// (`docs/conventions.md`). This is the direction the light comes **from**.
    pub azimuth_deg: f32,
    /// Degrees above the horizon.
    pub elevation_deg: f32,
    /// **The expensive switch** (`docs/lessons/performance.md` rule 5). A `bool` in a file so
    /// that the cost can be measured with two runs of the *same binary*.
    pub shadows: bool,
    /// Edge of one cascade's shadow map, in texels.
    pub shadow_map_size: usize,
    pub cascades: usize,
    pub first_cascade_far_bound_m: f32,
    pub shadow_distance_m: f32,
    pub cascade_overlap: f32,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
}

/// The fill. Cool against a warm sun — that split is what makes a shaded face read.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ambient {
    /// Linear RGB.
    pub color: (f32, f32, f32),
    /// cd/m². Only meaningful as a ratio against [`Sun::illuminance_lux`].
    pub brightness: f32,
}

/// The sky dome — three stops interpolated over the vertical, not one `ClearColor`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sky {
    /// Linear RGB, straight up.
    pub zenith: (f32, f32, f32),
    /// Linear RGB, at eye level. **Has to equal [`Fog::color`]** or the horizon is a seam.
    pub horizon: (f32, f32, f32),
    /// Linear RGB, straight down.
    pub nadir: (f32, f32, f32),
    /// Metres. Has to stay inside the camera's far plane — the dome is pinned to the eye.
    pub radius_m: f32,
    pub segments: u32,
    pub rings: u32,
}

/// Distance fog. Linear falloff: two numbers, both in metres, both walkable.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fog {
    /// Linear RGB. = [`Sky::horizon`].
    pub color: (f32, f32, f32),
    pub start_m: f32,
    pub end_m: f32,
}

// ---------------------------------------------------------------------------
// A map that does not reorder the file
// ---------------------------------------------------------------------------

/// A RON map read **in the order it was written**.
///
/// ## Why this exists instead of a `BTreeMap`
///
/// `missions.ron` lists the difficulty levels `recruit → veteran → elite` and the templates
/// `tutorial → skirmish`. That order is a design decision — easiest first, tutorial first — and
/// a `BTreeMap` throws it away, because a `BTreeMap` sorts by the *key* and the key is a
/// spelling. The lobby therefore offered `Elite | Recruit | Veteran`: the hardest level first
/// and the easiest one in the middle, with the tutorial behind the real mission
/// (FIND-092 §4, `docs/images/f175-lobby.png`).
///
/// **The fix belongs here and not in the UI.** A screen that re-sorts what it was handed is one
/// screen doing it; the next consumer — a save file, a log line, a mission-select on a
/// controller — gets the alphabet again. The file is the authority on order for the same reason
/// it is the authority on the numbers (§6 rule 2), so the container the file lands in has to be
/// able to *hold* an order.
///
/// The alternative was an explicit `order: u8` per entry in the RON. It was not taken: it adds
/// a number that can disagree with the thing it orders (two entries with `order: 2`, an entry
/// whose order was never updated after a move), it has to be typed correctly by hand on every
/// future entry, and it answers a question the file already answers by being a list of lines.
/// This container needs **no RON change at all** — `missions.ron` is byte-identical across this
/// fix, which is also what makes the red test above it trustworthy.
///
/// ## What it is not
///
/// Not a general-purpose map: lookup is a linear scan over a handful of entries read once at
/// startup, and it is deliberately not indexed. A **duplicate key is a load error**, not a
/// silent overwrite — the same choice as the missing-value rule (§6 rule 2): a file that says a
/// thing twice is a file somebody edited wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl<K: Eq, V> OrderedMap<K, V> {
    /// The value under `key`, or `None`. Linear — see the type's note.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.entries.iter().find(|(k, _)| k.borrow() == key).map(|(_, v)| v)
    }

    /// The value under `key`, to be changed. Only tests do this — they bend one number of a
    /// loaded file rather than shipping a second copy of it.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: std::borrow::Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.entries.iter_mut().find(|(k, _)| k.borrow() == key).map(|(_, v)| v)
    }

    /// Whether `key` is in the file.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: std::borrow::Borrow<Q>,
        Q: Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// The keys, **in file order**.
    pub fn keys(&self) -> impl DoubleEndedIterator<Item = &K> + ExactSizeIterator {
        self.entries.iter().map(|(k, _)| k)
    }

    /// The values, **in file order**.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &V> + ExactSizeIterator {
        self.entries.iter().map(|(_, v)| v)
    }

    /// Pairs, **in file order**.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (&K, &V)> + ExactSizeIterator {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<'a, K: Eq, V> IntoIterator for &'a OrderedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (K, V)>,
        fn(&'a (K, V)) -> (&'a K, &'a V),
    >;

    fn into_iter(self) -> Self::IntoIter {
        fn split<K, V>(pair: &(K, V)) -> (&K, &V) {
            (&pair.0, &pair.1)
        }
        self.entries.iter().map(split as fn(&'a (K, V)) -> (&'a K, &'a V))
    }
}

impl<K, Q, V> std::ops::Index<&Q> for OrderedMap<K, V>
where
    K: Eq + std::borrow::Borrow<Q>,
    Q: Eq + ?Sized + std::fmt::Debug,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).unwrap_or_else(|| panic!("no entry {key:?} in this map"))
    }
}

impl<K: Eq, V> FromIterator<(K, V)> for OrderedMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut entries: Vec<(K, V)> = Vec::new();
        for (k, v) in iter {
            match entries.iter_mut().find(|(existing, _)| *existing == k) {
                Some(slot) => slot.1 = v,
                None => entries.push((k, v)),
            }
        }
        Self { entries }
    }
}

impl<'de, K, V> Deserialize<'de> for OrderedMap<K, V>
where
    K: Deserialize<'de> + Eq + std::fmt::Debug,
    V: Deserialize<'de>,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct InOrder<K, V>(std::marker::PhantomData<(K, V)>);

        impl<'de, K, V> serde::de::Visitor<'de> for InOrder<K, V>
        where
            K: Deserialize<'de> + Eq + std::fmt::Debug,
            V: Deserialize<'de>,
        {
            type Value = OrderedMap<K, V>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map whose order is meant to survive")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries: Vec<(K, V)> = Vec::with_capacity(map.size_hint().unwrap_or(0));
                while let Some((k, v)) = map.next_entry::<K, V>()? {
                    if entries.iter().any(|(existing, _)| *existing == k) {
                        return Err(serde::de::Error::custom(format!(
                            "the key {k:?} is in this map twice — one of the two is a typo, \
                             and a silent overwrite would hide it"
                        )));
                    }
                    entries.push((k, v));
                }
                Ok(OrderedMap { entries })
            }
        }

        deserializer.deserialize_map(InOrder(std::marker::PhantomData))
    }
}

// ---------------------------------------------------------------------------
// missions.ron / traits.ron — still nearly empty, but present and loaded
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Missions {
    /// The place the game is played **out of** — the main building (user, 2026-08-12).
    pub hub: HubLayout,
    /// **Ordered, and that is load-bearing**: the lobby's mission row is this list, left to
    /// right, and `missions.ron` puts the tutorial first on purpose.
    pub templates: OrderedMap<String, MissionTemplate>,
}

/// Where the hub's furniture stands. **Layout, not tuning** — every number here is a place in
/// meters, and the two *rates* the hub needs (how fast a station fills a tank, how far it
/// reaches) stay in `gear.ron: resupply`, where they already were.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubLayout {
    pub name: String,
    /// Where a player stands when he enters the hub — at launch and after every sortie.
    pub spawn_m: (f32, f32, f32),
    /// How long the verdict stays on screen before the hub takes you back. Seconds in the
    /// file, ticks in the code (`docs/conventions.md`).
    pub debrief_s: f32,
    /// The doors. **One per difficulty** — a game with mouse-look and no cursor cannot offer
    /// a menu here, so the choice is a place you walk to.
    pub deployments: Vec<DeploymentPad>,
    /// Where gas comes back. The **only** thing in the game that ever refills a tank
    /// (`docs/QUESTIONS.md` Q-033).
    pub refuel_stations: Vec<StationPad>,
}

/// One door: which mission at which difficulty, and the circle you have to stand in.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeploymentPad {
    /// A key in [`Missions::templates`].
    pub mission: String,
    /// A key in that template's [`MissionTemplate::difficulties`].
    pub difficulty: String,
    pub center_m: (f32, f32, f32),
    pub radius_m: f32,
}

/// One refuel station. **Only a place** — `gear.ron: resupply` says how fast and how far.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationPad {
    pub center_m: (f32, f32, f32),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionTemplate {
    pub name: String,
    pub map: String,
    /// The mission arc runs 5–7 min (Bible 5, change 10).
    ///
    /// ⚠️ This and the two fields below are the **direct-entry** numbers: what
    /// `--mission <name>` flies when nobody picked a difficulty at a hub door. A sortie
    /// deployed out of the hub always flies a [`Difficulty`] instead, and never these.
    pub target_duration_s: f32,
    /// How many titans have to die for `F-071` to flip the mission to `Won`. **⚠️ UNTUNED.**
    /// A number the mission counts to belongs in the file, not in Rust.
    pub kill_target: u32,
    pub waves: Vec<Wave>,
    /// What a difficulty level **is**: the same three numbers, once per level. The code holds
    /// only the mechanism (`mission::run::resolve`), so a fourth level is file work.
    ///
    /// May be empty — the tutorial has no levels, it is the tutorial. A hub door that names a
    /// difficulty which is not in here refuses to deploy, loudly.
    ///
    /// **Ordered, and that is load-bearing**: this list *is* the lobby's difficulty row, and
    /// the file runs easiest → hardest. See [`OrderedMap`].
    pub difficulties: OrderedMap<String, Difficulty>,
}

/// One difficulty level of one mission. **A set of numbers, never an `if`** (§4).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difficulty {
    /// What the HUD and the log call it: `Recruit`, `Veteran`, `Elite`.
    pub name: String,
    /// The deadline. Longer at the low end — the same fight with more air in it.
    pub target_duration_s: f32,
    /// How many cortex kills win it.
    pub kill_target: u32,
    /// Which kinds come, when, and how many. **This is the third lever**: an elite sortie is
    /// not a husk with more hit points, it is more titans and worse ones.
    pub waves: Vec<Wave>,
}

// `PartialEq` since 2026-08-12: `mission::run::SortieNumbers` compares two wave lists to say
// "the difficulty really replaced the template's waves and did not merely tweak a number".
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub at_s: f32,
    pub kind: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Traits {
    pub entries: BTreeMap<String, TraitDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitDef {
    pub name: String,
    pub cost: u32,
    pub description: String,
}
