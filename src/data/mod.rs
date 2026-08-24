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
    /// `progress.ron` — the progression spine (`F-120`, `F-121`, `F-122`): what a sortie
    /// earns, what a level costs, what a gear budget buys and which rank it is worth.
    pub progress: Progress,
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
            progress: load_ron(dir, "progress.ron"),
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
    pub net: NetTuning,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerTuning {
    pub height_m: f32,
    pub radius_m: f32,
    /// `F-010` — the speed a slide **guarantees**, in m/s. *„Momentum-Erhalt"* is the other
    /// half: `player::locomotion::slide` takes the larger of this and the speed the player
    /// already had, so a slide out of a fast landing never slows him down.
    pub slide_speed_m_s: f32,
    /// `F-010` — how long a slide lasts, in seconds. Seconds in the file, ticks in the code.
    pub slide_duration_s: f32,
    /// `F-010` — the i-frames a slide buys, in seconds. Deliberately **shorter than**
    /// [`Self::slide_duration_s`]: the tail of the slide is the part that carries you out, and
    /// invulnerability that lasts as long as the movement is a dodge you never have to time.
    pub slide_iframes_s: f32,
    /// `F-010` — the floor between two slides, in seconds, measured from the tick a slide
    /// **starts**. Without it a held `C` is permanent invulnerability.
    pub slide_cooldown_s: f32,
    /// `F-010` — below this horizontal speed there is no slide at all, in m/s. You cannot
    /// slide from standing; a slide is momentum you already have, redirected.
    pub slide_min_speed_m_s: f32,
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
    /// How much of [`gravity_m_s2`](Game::gravity_m_s2) the rope pull cancels **at
    /// perfect alignment**, dimensionless in `0..1`.
    ///
    /// Rides the same `max(0, look · rope)` and the same near-anchor fade as
    /// [`air_pull_m_s2`](Self::air_pull_m_s2), so it is a function of the angle and not a
    /// second constant: *„wenn man da hin schaut … dass man gerader hingezogen wird … aber
    /// wenn man nicht hinschaut man auch gut kreise schwingen kann"* (the user, 2026-08-20).
    /// Bounds in `tests/player.rs`: `> 0.5` (or the aligned haul still droops — measured
    /// 1.996 m of climb in four seconds at 0.0) and `< 1.0` (or the player is weightless
    /// while looking at his own anchor and there is no arc left to swing in).
    pub air_pull_lift_fraction: f32,
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

/// **Which force a hooked rope applies — the one switch behind `FIND-149`.**
///
/// The user played *Attack on Titan Revolution* on 2026-08-23 and reported, verbatim:
/// *„wenn ich mich hooke: dann werde ich direkt rangezogen wenn ich ran gehe. mit a und d kann
/// man zur seite gehen. aber sonst wird man direkt hingezogen! **wenn ich nichts drucke dann
/// wird auch nicht rangezogen!**"* — and immediately after: *„aber es ist ein etwas smoother
/// übergang! aber recht schnell!"*
///
/// **The reference drives. It does not swing.** The rope supplies a *direction*, the key
/// supplies the *force*, and a hooked player who holds nothing is not pulled at all. Ours is
/// the opposite by construction: a [`DistanceJoint`](avian3d::prelude::DistanceJoint) plus
/// gravity **is** a pendulum, and a pure swing runs 17–21 m/s while
/// [`VectorTuning::max_speed_m_s`] (75) needs gas.
///
/// **This is an enum and not a `bool`, and both variants are live**, because nobody in this
/// repository can decide which one is right — only the player can, and only by feeling both in
/// one session. So the swap is one line in `game.ron` and the pendulum stays bit-for-bit what
/// it was (`tests/player.rs::f149_the_two_force_models_are_not_the_same_thing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RopeForceModel {
    /// **Today's game.** The rope is a distance constraint; gravity and tension make an arc,
    /// and letting go of every key still carries you. Everything measured before 2026-08-23 —
    /// `FIND-035`, `FIND-041`, `FIND-045`, `B-005`'s ratchet, every number in
    /// `tests/vector_rope.rs` — is a statement about **this** variant.
    Pendulum,
    /// **The reference's model.** No joint at all: the rope is a line with a direction, and
    /// `player::locomotion::rope_drive` chases a target velocity along it while a movement key
    /// is held. No key, no target, no drive — gravity is then the only thing acting, exactly as
    /// if no hook existed.
    Drive,
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
    /// **A ceiling on the whole flight, in seconds.** The tip flies at
    /// `max(hook_speed_m_s, distance / this)`, so [`hook_speed_m_s`](Self::hook_speed_m_s)
    /// still decides every shot short enough to fit inside it and this key decides the rest.
    ///
    /// *„und time to hook also e drücken zum connecten geht zu lang! das muss schneller
    /// gehen."* (the user, 2026-08-20). Measured before it existed: press to `Anchored` is
    /// `1 + ceil(d / (hook_speed_m_s / hz))` ticks — 3 ticks at 18 m, **61 ticks at 500 m**.
    /// A cap in TIME and not a higher speed, because the other half of his 2026-08-12
    /// sentence is *„aber man soll sehen wie es aufspannt"*: 0.10 s is six frames of line at
    /// every range. Held by `tests/vector_hooks.rs`: `hook_range_m / hook_speed_m_s > this`
    /// (or the ceiling never binds and nobody could tell it was broken) and `<= 0.15 s`.
    pub hook_flight_max_s: f32,
    // -----------------------------------------------------------------------------------
    // `F-024` / `F-025` — the anchor candidate system. Every weight below is the backlog's
    // own number (`docs/backlog/gameplay.ron`, F-025), and it lives here so that the user can
    // retune the feel without a rebuild — which is the whole reason he asked for the two
    // sliders (*„damit ich testen kann was am besten wäre"*).
    // -----------------------------------------------------------------------------------
    /// How many probe rays the candidate sweep casts **per side of the crosshair**, along the
    /// screen-horizontal line through it.
    ///
    /// The sweep is the candidate query, and it is a BVH walk and not an iteration over the
    /// world (§6 rule 6): `this` extra `SpatialQuery::cast_ray` calls per side, at the
    /// 0.21 us per ray `vector::aim`'s header measured. **Only cast while the assist is on** —
    /// at 0 % the game casts exactly the one centre ray it always did.
    ///
    /// ⚠️ **It replaced `assist_probe_rings` × `assist_probes_per_ring` on 2026-08-19**, when the
    /// user asked for the search to be locked to the horizontal (*„nur auf der x achse … also
    /// seitlich"*). A ring has no meaning on a line, and a key whose name describes a shape the
    /// code no longer has is a lie the next reader pays for. The ray budget did not move: 2 × 4
    /// was 8 a side and this is 8 a side, so the resolution along the one axis that is left went
    /// from 2 samples to 8 for the same 16 rays. See [`vector::aim::probe_dirs`].
    pub assist_probe_steps: u32,
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
    /// **`B-003` — how much LONGER than it is a `warp` may leave a rope and still keep it,
    /// in metres.**
    ///
    /// A [`DistanceJoint`](avian3d::prelude::DistanceJoint) with `limits = (0, L)` corrects
    /// only when the distance *exceeds* `L`, so a teleport that leaves the rope no longer than
    /// it already is has nothing to correct at all: a warp toward the anchor, along it, or onto
    /// the same spot keeps the rope, and the length ratchets down to the distance that now
    /// really exists (`B-004`).
    ///
    /// ⚠️ **The other direction has no usable budget.** Measured 2026-08-19 on a 9.00 m rope:
    /// the solver corrects the whole excess inside one *substep*, so it leaves as a velocity of
    /// `excess * simulation_hz * substeps` — 0.01 m of excess is **14.40 m/s**, 0.05 m is
    /// **72.00 m/s**. So this is a float tolerance, not a distance a player may be moved, and
    /// the bound in `tests/vector_rope.rs::b003_the_warp_slack_is_bounded_by_the_files_own_numbers`
    /// says so: `0 < this <= player.run_speed_m_s / (simulation_hz * substeps)`, i.e. the kick a
    /// kept warp may deliver stays below the speed the player walks at.
    pub warp_rope_slack_m: f32,
    pub min_rope_m: f32,
    /// **Pendulum or drive — `FIND-149`.** See [`RopeForceModel`]; there is no default and a
    /// `game.ron` without this key crashes on load, because "which physics does the rope have"
    /// is not a question a missing line may answer.
    pub rope_force_model: RopeForceModel,
    /// **⚠️ UNTUNED — the first of the three numbers the user is asked to feel.** The speed
    /// [`RopeForceModel::Drive`] chases along the rope while `W` is held, in m/s. Ignored
    /// entirely by [`RopeForceModel::Pendulum`].
    pub drive_speed_m_s: f32,
    /// **⚠️ UNTUNED — the second, and the one the user complained about first.** The drive's
    /// time constant in seconds: the velocity closes `1 − 1/e` = 63 % of the gap to the target
    /// in this long, 95 % in three of them. *„ein etwas smoother übergang! aber recht
    /// schnell!"* is the whole specification of this number. Must be `> 0`.
    ///
    /// **It is also the straightness knob**, and that is not obvious from the name: the
    /// steady-state sag under gravity is `atan(ramp · g / speed)`, and the crossing momentum
    /// that bends a flight into a curve decays with this same constant. `FIND-153`.
    pub drive_ramp_s: f32,
    /// **⚠️ UNTUNED — the third.** What `A`/`D` add across the rope under
    /// [`RopeForceModel::Drive`], in m/s. *„das a d sorgt dafür dass man nicht immer direkt zum
    /// seil gezogen wird"* — this is how far off the anchor line the player can hold himself,
    /// and since `FIND-153` it is a **steering authority only**: `A`/`D` on their own chase this
    /// speed on their own axis and no longer brake the flight down to it (`Q-050`).
    pub drive_lateral_m_s: f32,
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
    /// `F-008` — **how many dashes in a row**, i.e. the "Anzahl der Dashes" the backlog row
    /// calls a stat. A float because [`DodgeCharges`](crate::shared::DodgeCharges) refills
    /// fractionally and one number is better than a number plus an accumulator.
    pub dodge_charges: f32,
    /// `F-008` — seconds for **one** charge to come back. The magazine refills continuously,
    /// so a full one from empty takes `dodge_charges * this`.
    pub dodge_recharge_s: f32,
    /// `F-008` — the floor between two dashes, in seconds. The charge count says how many, this
    /// says how fast; without it a full magazine empties in three ticks and the stat means
    /// nothing.
    pub dodge_cooldown_s: f32,
    /// `F-009` — how much **sideways** speed one flip adds, in m/s. A velocity change like
    /// [`Self::dodge_impulse_m_s`], divided by the timestep in `vector::dodge`.
    pub flip_impulse_m_s: f32,
    /// `F-009` — how much **upward** speed rides along with a flip, in m/s. A flip that is
    /// purely lateral drives you into the wall you were trying to leave.
    pub flip_up_m_s: f32,
    /// `F-009` — the i-frames a flip buys, in seconds. Seconds in the file, ticks in the code.
    pub flip_iframes_s: f32,
    /// `F-009` — how many ticks may lie between the two `A` (or two `D`) presses for them to be
    /// one flip. Ticks, for [`Self::dodge_double_tap_window_ticks`]'s reason.
    pub flip_double_tap_window_ticks: u64,
    /// `F-009` — what one flip costs, flat, like [`Self::gas_dodge`] and for the same reason:
    /// it is an impulse, not a rate, so it must never grow a `_per_s`.
    pub gas_flip: f32,
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
    /// `F-009` flip. **The second consumer that is not a rate** — it bills
    /// [`gas_flip`](VectorTuning::gas_flip) once, on the tick a double-tap of `A` or `D`
    /// lands in the air, and nothing on any other tick.
    ///
    /// It is **last** in `game.ron: vector.gas_priority`, and unlike the dodge's position that
    /// is worth arguing about: a flip costs 20 flat, so being served last can cost it at most
    /// the 0.4 gas the three rates take in one tick — 2 % of its own price. What decides its
    /// place is the sentence the list already makes: the explicit presses come before the
    /// ambient ones, and among the explicit presses the one that keeps you alive is the one you
    /// can least afford to have refused. That would argue for putting it FIRST — and it is not,
    /// because on a tank that thin the flip is not what saves you anyway, and moving `Boost`
    /// off the front would overturn the user's own answer to `Q-017` as a side effect of adding
    /// a verb. It is `docs/QUESTIONS.md` Q-052 and the assumption is written there.
    Flip,
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
    /// `F-017` — **where the speed curve starts**, in m/s. Below it the image stays at
    /// [`Self::fov_deg`]; from here to `vector.max_speed_m_s` the field of view opens linearly
    /// to [`Self::fov_max_speed_deg`].
    ///
    /// It is not `0.0` on purpose: walking is 6 m/s and a walk that already widens the lens
    /// sells nothing — the effect has to mean *fast*, and the number is what says where fast
    /// begins.
    pub fov_speed_from_m_s: f32,
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

/// The session: the port, the seat, and how long a dropped connection keeps its chair.
///
/// **Numbers, not mechanics** (§4). What a transport *is* stands in `src/net/`; how long it
/// waits stands here, because a timeout is a thing you tune against a real line and not a
/// thing you rebuild for.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetTuning {
    /// The UDP port `--host` opens when it is given no number of its own.
    pub port: u16,
    /// How far apart joining players are seated, in meters, along +X from the origin. Small
    /// enough to be one squad, big enough that nobody spawns inside the deployment pad the
    /// player before him is standing on.
    pub seat_spread_m: f32,
    /// After how many seconds of silence a peer counts as **disconnected**. Not "gone" — his
    /// seat is still his; see [`NetTuning::slot_hold_s`].
    pub peer_timeout_s: f32,
    /// **How long a dropped connection holds its slot** (bible F-158a: 120 s). The session
    /// outlives the connection: his body, his gas and his kills hang on a `PlayerId`, and
    /// reconnecting inside this window puts him back in the same chair.
    pub slot_hold_s: f32,
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
    /// `F-019` — **the refuel points that stand out in the field**, placed by hand exactly
    /// like a [`MapBlock`] and a [`MapLight`].
    ///
    /// **Explicit, never defaulted** (§4): a map that forgets the key fails to load. A map
    /// with no supply writes `supply_stations: []` and says so — and `graybox` does exactly
    /// that, because it is the fixture a dozen tests reason about at `y = 0` and a station is
    /// a thing in the world.
    pub supply_stations: Vec<SupplyPoint>,
}

/// One `F-019` refuel point — **a place, and a number of reloads.**
///
/// Everything else about it (how far it reaches, how long a reload takes, what one reload is
/// worth) is the same for every station in the game and lives in `gear.ron: resupply`. What is
/// per-station is where it stands and how much is in it, and that is what a map author places.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyPoint {
    /// World centre. The visible marker is drawn around it and the trigger circle is measured
    /// from it in 3D — a player 40 m above a station is not standing at it.
    pub center_m: (f32, f32, f32),
    /// How many reloads **this** station holds. Per station and not per map, because a depot
    /// beside the wall and a lone pole in the ruins are not the same promise.
    pub uses: u32,
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
    /// `F-031` + `F-041` + `F-044` — **what a landed hit is worth**, 2026-08-25.
    ///
    /// The blade's own numbers stay in [`BladeTuning`] (`damage_per_m_s`, `min_speed_m_s`);
    /// what stands here is everything that turns one `TitanHit` into a number of wound
    /// points: the per-zone factors, the flat ground attack, the collapse, and the combo.
    pub damage: DamageTuning,
}

/// `F-031` — **the damage formula's file half.**
///
/// `damage = blades.damage_per_m_s x closing_m_s x zone factor x combo multiplier`, and every
/// factor in that line except the closing speed lives in a file. `blades.damage_per_m_s` had
/// **no reader anywhere in `src/`** until [`crate::combat::damage`] landed — the same shape
/// `docs/FINDINGS.md` FIND-075 records for `wear_per_hit`, and `titan.ron: <kind>.health` was
/// the second half of the same hole: `titan::rig` inserted a `Health` nothing ever wrote.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageTuning {
    /// The chest and everything the cast could not resolve to a limb.
    pub zone_torso_factor: f32,
    pub zone_head_factor: f32,
    pub zone_eye_factor: f32,
    /// Arms and legs share one factor: `F-032` gives them separate zones, not separate worth.
    pub zone_limb_factor: f32,
    /// `F-044` — **the flat worth of a ground attack**, in wound points, with no speed term
    /// at all. Below `damage_per_m_s * min_speed_m_s` by construction, and
    /// `tests/combat.rs::f044_a_ground_attack_is_never_the_better_choice` is what says so.
    pub ground_damage: f32,
    /// `F-031` — how long a titan whose wound pool has just been emptied spends on the floor,
    /// in seconds. Ticks in the code.
    pub collapse_s: f32,
    /// `F-031` — **the whole no-stun-lock claim.** A titan who has just been floored cannot be
    /// floored again inside this window, however hard he is cut. A refractory period and not an
    /// inequality between numbers: the arithmetic version depends on `damage_per_m_s`,
    /// `combo_max`, the zone factor and every kind's `health` at once, and it is already false
    /// for the scuttler at the shipped values (`gear.ron` carries the measurement).
    pub collapse_refractory_s: f32,
    /// `F-041` — what one further airborne hit adds to the multiplier. The first hit of a
    /// chain is always `1.0`; the `n`-th is `1 + combo_step * (n - 1)`, capped.
    pub combo_step: f32,
    pub combo_max: f32,
    /// How long a chain survives without a further hit, in seconds. Ticks in the code.
    pub combo_window_s: f32,
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
    /// `F-043` — how long the hit mark stays on screen, in seconds.
    ///
    /// **`0.0` switches the whole element off**, which is the row's *"vollstaendig
    /// abschaltbar"* answered in data instead of in a settings screen
    /// ([`crate::hud::hit_mark`]). Read on the frame clock and never converted to ticks: it is
    /// view state, not simulation.
    pub hit_mark_s: f32,
    /// `F-043` — the closing speed at or above which a body cut reads as `CUT` and below which
    /// it reads as `GRAZE`.
    ///
    /// A **feedback** threshold and not a damage threshold: `F-031` (the damage formula) is
    /// unbuilt, and `blades.damage_per_m_s` still has no reader. It lives in `feel` for the
    /// same reason the hit stop does — what the player is told about a hit is not what the hit
    /// does.
    pub strong_hit_m_s: f32,
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

    // -----------------------------------------------------------------------------------
    // F-019 — the stations that stand OUT IN THE FIELD, and they are a different thing from
    // the racks of the hub above.
    // -----------------------------------------------------------------------------------
    /// `F-019` — **how many reloads one field station has before it is empty**, and it is the
    /// number that closes `Q-044`: a tank that buys ~16.7 s of held boost against a 330 s
    /// sortie, with no refuel anywhere in the world, is a sortie you cannot fly.
    ///
    /// Finite, because the reference's are (`docs/references.md`, `FIND-150`) and because an
    /// infinite one turns every fight into "fly back to the pole". A `u32` and not a `u8`: the
    /// count is per station **per sortie** and a map may one day want a depot.
    pub station_uses: u32,
    /// `F-019` — **how long one reload takes**, in seconds. The acceptance sentence names it
    /// outright: *„Nachladen dauert 1,5 s"*. It is what makes a refill a decision — 1.5 s
    /// standing still is an eternity with a titan in the street.
    pub station_refill_s: f32,
    /// `F-019` — how far a field station reaches, in meters. Wider than the hub's
    /// [`Self::range_m`] on purpose: you arrive at a hub on foot and at a field station at
    /// 40 m/s, and a 4 m circle at 40 m/s is 6 ticks wide.
    pub station_radius_m: f32,
}

// ---------------------------------------------------------------------------
// titan.ron
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Titans {
    pub kinds: BTreeMap<String, TitanKind>,
    /// `F-051` — **how a titan notices you at all**, and it is game-wide on purpose: how fast
    /// attention builds and how loud a player is are feel numbers, while how far a kind sees
    /// and hears is that kind's identity and stands per row (`sight_half_angle_deg`,
    /// `hearing_radius_m`).
    pub perception: TitanPerception,
    /// `F-055` — the ring of standing places a group of titans divides between them.
    pub crowd: TitanCrowd,
    /// `F-054` — how often a titan's brain runs, by distance.
    pub lod: TitanLod,
}

/// `F-051` — **the perception model**, the half of it that is not a kind's own.
///
/// Two channels, and they are not the same shape:
///
/// * **the eye** is a cone — `aggro_radius_m` long, `sight_half_angle_deg` wide — and it is
///   *instant*. A titan that has you in front of him at 30 m has seen you, and pretending
///   otherwise would only make him look broken.
/// * **the ear** is a circle and it *accumulates*. What it hears is the player's own noise
///   radius, which is a function of how he is moving: standing still is [`Self::quiet_m`],
///   and hanging on a rope with the gas open is that plus his speed, times
///   [`Self::rope_factor`].
///
/// **That asymmetry is the whole feature.** The acceptance sentence of `F-051` is *"a player
/// who acts quietly is discovered later than one who boosts"*, and it can only be true for a
/// titan who is **not** looking at you — so the number that decides it has to be the ear's.
///
/// ⚠️ **UNTUNED.** Nobody has played any of these seven numbers.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanPerception {
    /// The noise radius, in meters, of a player who is doing **nothing**. Not zero: a man in
    /// a harness standing in a street is not silent, and a zero here would make a motionless
    /// player literally undetectable by ear at any range, which is a stealth game nobody
    /// asked for.
    pub quiet_m: f32,
    /// Meters of extra noise radius per m/s of ground speed. **This is the gas**: every m/s
    /// the vector gear buys is bought with a jet, and the jet is what the bellower is
    /// listening for.
    pub noise_per_speed_m: f32,
    /// What hanging on a rope multiplies the noise radius by
    /// ([`MovementState::Tethered`](crate::shared::MovementState::Tethered)). Above 1.0: a
    /// tethered player is under power, a falling one is only fast.
    pub rope_factor: f32,
    /// Hard ceiling on the noise radius, so a 90 m/s dive does not wake the whole map.
    pub max_noise_m: f32,
    /// How fast a heard player turns into a target, in units of awareness per second, at a
    /// stimulus of 1.0 (a noise source standing on top of the titan). Awareness runs 0..1 and
    /// 1.0 is "detected", so 1.5 means a maximal noise needs two thirds of a second.
    pub hearing_gain_per_s: f32,
    /// How fast awareness drains when there is neither sight nor sound.
    pub forget_per_s: f32,
    /// The **hysteresis floor**: a titan that has detected you stays detected until awareness
    /// falls back to this. Without it a player who steps out of the cone for one tick resets
    /// the whole chase, and the titan flickers between `Pursue` and `Idle`.
    pub lose_level: f32,
}

/// `F-055` — **the ring**. Six titans on one player stand in six places, not one.
///
/// A slot is not a formation the AI negotiates; it is a **rank**. `titan::perception::
/// claim_slots` sorts the titans that share a target by [`TitanId`](crate::shared::TitanId)
/// and hands out `0..n`, which is deterministic on every machine (`docs/multiplayer.md`
/// rule 4) and costs no arbitration message.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanCrowd {
    /// How many bearings the ring is divided into. Above this many titans the slots repeat.
    pub slots: u32,
    /// How far off the player his attackers aim their approach, in meters, at full strength.
    /// It **fades to nothing** the same way `behaviour.flank_offset_m` does — full beyond
    /// twice a kind's own `attack_range_m`, zero at the range itself — or a ring of titans
    /// would orbit forever and never reach the man in the middle.
    pub ring_radius_m: f32,
}

/// `F-054` — **how often a titan thinks**, by how far away he is from the nearest player.
///
/// ⚠️ **The near tier is every tick (60 Hz) and not the feature row's 20 Hz.** The row was
/// written against a server that ticks slower than this one does; here the wind-up of `F-053`
/// is *measured evidence* at 36 ticks of 60 Hz, and a near tier that only looked every third
/// tick would move the state edges the picture was taken against. 20 Hz is therefore the
/// **mid** tier, exactly as written, and near means "full rate".
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanLod {
    /// Inside this distance the brain runs every tick.
    pub near_m: f32,
    /// Between [`Self::near_m`] and this, the brain runs at [`Self::mid_hz`].
    pub mid_m: f32,
    /// Beyond [`Self::mid_m`] the brain runs at [`Self::far_hz`]. There is no fourth tier that
    /// stops thinking altogether: a titan who never re-checks is a titan who never notices the
    /// player who walked up to him, and "position interpolation only" is what `brain::walk`
    /// already is — it runs every tick for everybody and carries the gait the brain last set.
    pub mid_hz: f32,
    pub far_hz: f32,
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
    /// **Half the arc the CORTEX can be cut in**, in degrees off the titan's own **backward**
    /// vector, measured on the ground plane. The mirror of [`Self::strike_half_angle_deg`], and
    /// the same shape: together with the cortex sphere it makes the kill zone a **cone opening
    /// backwards** instead of a floating bullseye.
    ///
    /// **Why it exists** — `docs/PLAN-GAME.md` §3.4 point 3: *"A 360° sphere makes the titan a
    /// floating bullseye and deletes the approach-angle skill F-030 exists to create. A rear
    /// hemisphere is the design."* Until 2026-08-20 that rule was produced by **geometry
    /// alone**, and the whole of it was **0.211 m of blade** on a husk (`docs/FINDINGS.md`
    /// FIND-147: the rear pass has the blade 0.131 m inside the cortex, the front pass is
    /// 0.080 m short). Any growth of `reach_m` or `cortex_radius_m` past 0.08 m spends that
    /// accident — so the rule had to become a rule before either could move.
    ///
    /// **Why the numbers are above 90 and not below.** 90° would be the literal rear
    /// hemisphere, and it is unplayable: a titan turns toward you at
    /// [`turn_deg_per_s`](Self::turn_deg_per_s) while your swing is in the air, so a player who
    /// presses at 85° is at 95° when the blade lands and the kill he aimed at silently becomes
    /// a torso graze. Every value is therefore `90 + turn_deg_per_s × 0.15 s + 15°` rounded to
    /// the nearest 5 — **the gate hands back exactly what the titan's own turn takes**, plus a
    /// tick and a half of margin. The 0.15 s is the swing's own press-to-contact time,
    /// `(active_from_s + active_to_s) / 2` out of `gear.ron`.
    ///
    /// A rejected cortex is **not a whiff**: `blades::cut::sweep` falls through to the body
    /// layer, so a blade that reaches the neck from the front books `Torso` and the pass still
    /// costs the titan a stagger. The reference does the same — off-target hits there do 0.8x
    /// damage rather than nothing (`docs/gameplay/references.md`).
    ///
    /// ⚠️ **UNTUNED.** Range `[45, 180]`, guarded by
    /// `tests/combat.rs::every_kind_carries_a_cortex_half_angle_in_range`. At 180 the gate is
    /// gone and the titan is a bullseye again; below 45 the nape is unreachable by anything
    /// that is not a stationary approach from dead astern.
    pub cortex_half_angle_deg: f32,
    pub attack_cooldown_s: f32,
    /// **What a non-lethal cut into this kind's body costs him, in seconds of standing still**
    /// — `F-032`'s *"Kein Kill, sondern Stagger, Bewegungs-Debuff oder Blendung"*.
    ///
    /// It is a **movement debuff and nothing more**: `combat::hitstop::begin` puts a
    /// [`HitStop`](crate::shared::HitStop) on the body, and the only two systems that read one
    /// on a titan are `titan::brain::walk` (his advance stops) and `titan::brain::dissolve`.
    /// His state clock, his wind-up and his pose keep running — `titan::brain::advance` does
    /// **not** read `HitStop` — so a cut can never interrupt an attack that is already
    /// telegraphed, and no amount of slashing turns a titan into a harmless statue.
    ///
    /// ⚠️ **There is an upper bound and it is not a matter of taste.** One player's two blades
    /// land a hit every `(swing_s + cooldown_s) / 2` = 0.325 s (`gear.ron: blades`). A
    /// `stagger_s` at or above that number is a permanent lock: the titan never gets a tick to
    /// move in. `tests/combat.rs::f032_no_kind_can_be_tuned_into_a_permanent_stagger_lock`
    /// falls over on it, and it reads both numbers out of the files rather than repeating them.
    ///
    /// ⚠️ **UNTUNED**, and differentiated by mass on purpose: the scuttler is 4.2 m and gets
    /// thrown off his line, the bellower is 21 m and barely notices. Nothing here is measured.
    pub stagger_s: f32,
    /// **How far this kind's eye reaches**, in meters — the *length* of the sight cone of
    /// `F-051`.
    ///
    /// Until 2026-08-25 this was a 360° circle and the whole of the perception model: a titan
    /// with his back to you acquired you at exactly the distance one facing you did, and the
    /// design's stealth layer — *"a player who acts quietly is discovered later than one who
    /// boosts"* — had nowhere to live. The number itself did not move on any kind; what
    /// changed is that it is now one side of a cone whose other side is
    /// [`Self::sight_half_angle_deg`], and that a player outside the cone has to be **heard**
    /// instead ([`Self::hearing_radius_m`]).
    pub aggro_radius_m: f32,
    /// **Half the width of the sight cone**, in degrees off the titan's own forward vector, on
    /// the ground plane — the same shape as [`Self::strike_half_angle_deg`] and
    /// [`Self::cortex_half_angle_deg`], and deliberately so: a titan's eye, his blow and his
    /// nape are three cones on one body, and a player who has learned to read one has learned
    /// the shape of the other two.
    ///
    /// Sight is **instant** inside it: the accumulation of `F-051` is the ear's, never the
    /// eye's. A titan looking straight at a man 30 m away who "has not noticed him yet" is not
    /// a stealth mechanic, it is a bug the player will report.
    ///
    /// ⚠️ **UNTUNED.** Range `[20, 170]`, guarded by
    /// `tests/titan.rs::f051_every_kind_carries_a_sight_cone_and_an_ear_in_range` — at 180 the
    /// cone is a circle again and the feature is deleted, below 20 a titan walks past a player
    /// standing in front of him.
    pub sight_half_angle_deg: f32,
    /// **How far this kind's ear reaches**, in meters, at most — the ear is a circle and it
    /// has no angle.
    ///
    /// It is a *ceiling*, not a trigger: what the ear actually hears is the smaller of this
    /// and the player's own noise radius (`titans.perception`), so a quiet player at 30 m is
    /// inaudible to a kind with a 160 m ear. **That is the bellower's whole design** — he is
    /// the kind that reacts to the sound of gas (`docs/gameplay/enemies.md`, `F-062`), and
    /// until this field existed he called on sight like everyone else.
    ///
    /// ⚠️ **UNTUNED.** Range `[5, 300]`, guarded with the cone above.
    pub hearing_radius_m: f32,
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
    /// **What this kind does that no other kind does** (`F-057`..`F-063`).
    ///
    /// Until 2026-08-19 every kind was the husk with different numbers: one brain, one walk,
    /// one way to die. `docs/gameplay/enemies.md` says the opposite — *"at least half of all
    /// enemy kinds carry an anti-autopilot property"* — and a property that is only a number is
    /// a reskin. This block is where the eight identities stop being prose.
    pub behaviour: TitanBehaviour,
}

/// The per-kind behaviour switches. **Every field on every kind, no `serde(default)`** — a kind
/// that forgets one has to crash on load, not quietly inherit the husk (§4 rule 2).
///
/// Zero means "this kind does not do this", and that is deliberate rather than an `Option`: the
/// husk's row reads as eight zeros and one `Always`, which is exactly what the husk is — the
/// teaching piece with nothing on top.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitanBehaviour {
    /// `F-057` — how far off the straight line to the player the walk swings, in degrees to
    /// each side. 0 walks straight at you, which is what makes a husk leadable.
    pub swerve_deg: f32,
    /// How long one swing lasts before the heading flips to the other side. With `swerve_deg`
    /// it is the whole of the errant's *"unpredictable changes of direction"*: the player has
    /// to lead a target that is not where the line says.
    pub swerve_period_s: f32,
    /// `F-058` — how fast the body carries itself **forward through its own `Strike`**. 0 is a
    /// blow struck from a planted stance; anything above it is a leap, and a player who dodges
    /// sideways at range still gets caught. The scuttler's answer is up, not sideways.
    pub lunge_m_s: f32,
    /// `F-063` — how far to the side of the player this kind aims its approach, in meters.
    /// Two chorus with opposite signs arrive from two sides and cannot both be kept in front.
    pub flank_offset_m: f32,
    /// `F-062` — the radius in which this kind wakes every idle titan the moment it acquires a
    /// target. 0 is silent. This is the *call*, not the ear: the ear is `F-051` and it does not
    /// exist, so a bellower today calls when it sees you rather than when it hears your gas.
    pub call_radius_m: f32,
    /// How long a titan that answered the call keeps coming, in seconds — **the number that
    /// makes the call worth anything.** Without it a woken titan falls straight back to `Idle`
    /// on the next tick, because `titan::brain::decide` sends anything outside its own
    /// `aggro_radius_m` home again; the call would then be a one-tick flicker nobody sees.
    pub call_hold_s: f32,
    /// `F-061` — an ambusher never pursues. It stands still until you come inside
    /// `attack_range_m`, strikes, and goes back to standing still. It cannot be outrun because
    /// it never ran; it can only be avoided, which is the attention the lurker is for.
    pub ambush: bool,
    /// When the nape can be cut at all. See [`CortexGuard`].
    pub cortex_guard: CortexGuard,
    /// `F-059` — how long the whole backward roll lasts, in seconds. **0 = this kind does not
    /// roll**, and seven of the eight do not.
    pub roll_s: f32,
    /// How much of [`roll_s`](Self::roll_s) is the **readable startup**, during which the nape
    /// is still a target. The rest is the guaranteed invulnerability the design asks for. A
    /// startup of 0 would be i-frames with no tell, which is the one thing pillar P4 forbids —
    /// `tests/data.rs` falls over on it.
    pub roll_startup_s: f32,
    /// How fast the body carries itself **backwards** through the roll, in m/s. It is the half
    /// that makes the roll cost the player position and not just time.
    pub roll_speed_m_s: f32,
}

/// **When a kind's cortex is a target.** The cortex is the only lethal spot in the game
/// (`docs/gameplay/enemies.md`), so this is the strongest per-kind lever there is — and it is
/// the reason the warden and the weaver are not the husk with other numbers.
///
/// It is enforced by taking the cortex sensor **out of the world** while the nape is covered
/// (`titan::brain::guard_the_cortex`), not by throwing a `TitanHit` away afterwards. That
/// matters: a hit thrown away in `titan/` would still have been counted as a kill by
/// `mission::count_kills`, which reads the message and not the corpse.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum CortexGuard {
    /// The nape is always open. Husk, errant, scuttler, lurker, chorus, bellower.
    Always,
    /// `F-059` — the nape is open **only while the kind is committed to its own attack**
    /// (`Windup`, `Strike`, `Recover`). Spamming the approach finds nothing; baiting the blow
    /// and cutting inside the window is the only way in. That is the weaver's *"timing instead
    /// of spam"*.
    WhenCommitted,
    /// `F-060` — the hand covers the nape until a cut into the **body** knocks it away, and it
    /// stays away for this many seconds. The warden's two-stage attack: body first, then the
    /// cortex.
    WhenOpened(f32),
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
    /// `F-059` — how far the torso tips **forward** through the roll's startup. Negative, and
    /// that is the sign convention of `titan::rig`: `windup_lean_deg` is positive and tips the
    /// shoulders back, so a crouch is the other way round.
    pub roll_lean_deg: f32,
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
    /// How long `WON`/`LOST` stands over the field the sortie was decided on, before the
    /// **debrief** comes up. Seconds in the file, ticks in the code (`docs/conventions.md`).
    ///
    /// ⚠️ **This field carried the name `debrief_s` until 2026-08-24** and it meant "verdict to
    /// hub". It was split when the debrief became a phase of its own (`F-175`): the verdict is
    /// one hold and the report the player reads is another, and a single number cannot be both
    /// without one of the two being wrong.
    pub verdict_s: f32,
    /// How long the **debrief** stands before the hub takes you back — in a run that has no
    /// screen to hold it.
    ///
    /// ⚠️ A windowed run never spends this: `menu::Screen::Debrief` comes up with the phase and
    /// a screen stops `Time<Virtual>`, so the ticks this is counted in do not happen until the
    /// player has clicked out of it (`src/menu/debrief.rs`). It is what a `--headless` or
    /// `--script` run — which has no window and therefore no menu at all — waits instead, and
    /// it is what `scripts/f175-loop.txt` asserts the phase inside of.
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
    /// ⭐ **What this mission IS** — the mode, as data (`F-072`, `F-073`, `F-185`, 2026-08-25).
    ///
    /// Until today there was exactly one way to decide a sortie: kill [`Self::kill_target`]
    /// titans before [`Self::target_duration_s`] runs out. That was not a design decision, it
    /// was the only branch anybody had written — `mission::decide` reads a tally and a clock and
    /// nothing else.
    ///
    /// **On the template and deliberately not on the [`Difficulty`].** A mode is what the
    /// mission *is*; a difficulty is how hard that same mission is. A level that could turn a
    /// breach into a cull would make "which mission am I flying" a question with two answers,
    /// and the lobby shows the two rows side by side. The three numbers a level does own
    /// (deadline, kill target, waves) are enough to build a ladder out of any mode — for a
    /// [`Objective::Breach`] the deadline **is** the difficulty, because holding longer is
    /// harder.
    pub objective: Objective,
}

/// **How a sortie is decided.** One variant per mode, and the numbers each mode needs.
///
/// ⚠️ **The clock does not mean the same thing in every variant, and that is the point.** In
/// [`Objective::Cull`] running out of time is how you *lose*; in [`Objective::Breach`] it is how
/// you *win*. `mission::objective::verdict` is the one function that knows which, and
/// `src/mission/objective.rs` holds its unit tests — an inversion there is the whole feature
/// silently backwards, so it is a pure function with a red test and not an `if` inside a system.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Objective {
    /// **Kill them.** [`MissionTemplate::kill_target`] cortex cuts before the deadline; the
    /// deadline loses it. The only mode that existed before 2026-08-25, and the one every
    /// script and every test in the repository was written against.
    ///
    /// It carries no numbers of its own on purpose: the count is `kill_target`, which `hud`,
    /// `menu::lobby` and `progress` already read off the template, and a second place to write
    /// it would be a second answer.
    Cull,
    /// **Hold the gate** (`F-072`). Survive to the deadline and you have won; let
    /// `breaches_allowed` titans reach the gate and you have lost.
    ///
    /// ⚠️ Titans in this game walk at the nearest **player**, not at a place
    /// (`titan::brain::nearest_player`) — so the gate is defended by standing between it and
    /// them, which is what a defence mission is. It is written down here because a reader who
    /// expects pathing to a goal will look for it and not find it.
    Breach {
        /// Where the gate stands, in meters.
        gate_m: (f32, f32, f32),
        /// How close a titan has to get for it to count as through.
        reach_m: f32,
        /// How many may get through before the sortie is lost. **Each titan counts once.**
        breaches_allowed: u32,
    },
    /// **Fly the rings** (`F-185`). Pass through every ring, in the order they are listed,
    /// before the deadline. No titans required — this is the mode that teaches the drive.
    Parcours {
        /// The gates, **in order**. A parcours you can fly backwards teaches nothing.
        rings: Vec<Ring>,
    },
    /// **Walk the cart home** (`F-073`). The cart rolls along its waypoints only while a player
    /// is inside `escort_radius_m` of it; it wins when it reaches the last one.
    Escort {
        /// The path, in order. The cart starts on the first and arrives on the last.
        waypoints: Vec<(f32, f32, f32)>,
        /// How fast it rolls while somebody is escorting it.
        speed_m_s: f32,
        /// How close a player has to be for it to move at all.
        escort_radius_m: f32,
        /// How big the cart is drawn, in meters.
        size_m: (f32, f32, f32),
    },
}

/// One ring of an [`Objective::Parcours`]. **A place and a size, nothing else.**
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ring {
    pub center_m: (f32, f32, f32),
    pub radius_m: f32,
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

// ---------------------------------------------------------------------------
// progress.ron — the progression spine: F-120 the curve, F-121 the rank, F-122 the budget
// ---------------------------------------------------------------------------

/// Everything `progress` and `save` need in order to say what a sortie was worth.
///
/// **Rule 2, in full force.** Not one of these numbers stands in Rust, and none of them has a
/// `serde(default)`: a `progress.ron` that is missing a field crashes at startup with the file
/// name, exactly like every other tuning file. The *save* file is the one place that is allowed
/// to fill a missing value, and the reason is written down in `src/save/file.rs`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub xp: XpTuning,
    pub levels: LevelTuning,
    pub gear: GearTuning,
    /// Ascending by `min_gear_points`, and `tests/progress.rs` is what keeps them that way —
    /// a rank list out of order would silently hand out the wrong letter.
    pub ranks: Vec<RankTier>,
    /// `"<template>"` or `"<template>/<difficulty>"` -> the lowest rank that may fly it.
    /// **Empty ships nothing locked** (`assets/data/progress.ron` says why).
    pub gates: BTreeMap<String, String>,
}

/// `F-120` — what one finished sortie earns. Facts in, experience out.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XpTuning {
    pub per_sortie_flown: f32,
    pub per_sortie_won: f32,
    pub per_titan_felled: f32,
    pub per_minute_in_the_field: f32,
    /// Keyed by `missions.ron: <template>.difficulties`.
    pub difficulty_multipliers: BTreeMap<String, f32>,
    /// The direct drop-in (`--mission tutorial`), which belongs to no tier.
    pub without_a_difficulty: f32,
}

impl XpTuning {
    /// The multiplier for a tier, or **the loud fallback**.
    ///
    /// A tier that is in `missions.ron` and not in `progress.ron` is a data error, and
    /// `tests/progress.rs::f120_every_difficulty_in_missions_ron_has_an_xp_multiplier` is what
    /// catches it before it ships. At runtime it must not crash — a career is not worth losing
    /// over a missing multiplier — so it earns the direct drop-in's rate and says so.
    pub fn multiplier_for(&self, difficulty: Option<&str>) -> f32 {
        let Some(tier) = difficulty else { return self.without_a_difficulty };
        match self.difficulty_multipliers.get(tier) {
            Some(m) => *m,
            None => {
                error!(
                    "progress.ron: no xp multiplier for difficulty {tier:?} — this sortie is \
                     paid at the no-tier rate {}",
                    self.without_a_difficulty
                );
                self.without_a_difficulty
            }
        }
    }
}

/// `F-120` — the curve, as three numbers instead of a hundred rows.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelTuning {
    pub max_level: u32,
    /// What the step from level 1 to level 2 costs.
    pub first_step_xp: f32,
    /// Every further step costs this much more than the one before it.
    pub step_growth: f32,
    pub skill_points_per_level: u32,
    pub gear_points_at_level_one: u32,
    pub gear_points_per_level: u32,
}

/// `F-122` — one budget over several axes, with the couplings that make it a choice.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearTuning {
    /// `< 1.0` or the strongest build is always a single-axis dump. **The whole design.**
    pub diminishing_exponent: f32,
    /// In the order a screen would show them.
    pub axes: OrderedMap<String, GearAxis>,
    pub couplings: Vec<GearCoupling>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearAxis {
    /// What one unit of this axis's effect is worth when a build is weighed against another.
    /// **A stand-in for a measured sortie**, and it is the one number in this file that is a
    /// property of the *test* rather than of the game (`docs/FINDINGS.md` FIND-155).
    pub strength_weight: f32,
}

/// "Speed costs control, damage costs durability" — one line per sentence.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearCoupling {
    pub spends: String,
    pub costs: String,
    /// How much of the spender's effect is taken back off the axis it costs.
    pub drag: f32,
}

/// `F-121` — one rung of the E..S ladder.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankTier {
    pub name: String,
    pub min_gear_points: u32,
}
