//! `F-004` pendulum and `F-005` reel-in — the guard over the rope.
//!
//! The rope is an avian `DistanceJoint` with `limits = (0, L)`. That is not a taste
//! decision, it is `docs/measurements/rope-decision.md`, and these tests are the criteria
//! that decision was made against, written down before the code was:
//!
//! - reeling in **gains** speed (58.23 m/s out of `v0 = 20` in the measurement, against
//!   exactly 20.000 for the hand-written clamp that was retired),
//! - the shortening happens **per substep**, not per tick (per tick injects
//!   `rate x SubstepCount` = 677 m/s and drives the player through walls),
//! - the swing loses little speed per second (4.26 %/s measured at 24 substeps),
//! - the rope pulls and never pushes,
//! - it never gets shorter than `vector.min_rope_m`,
//! - and letting go really removes the joint.
//!
//! ## Every number comes out of `GameData`
//!
//! Not one literal from `game.ron` stands in this file. A test that measures against a value
//! it hard-codes itself stays green on the day somebody hard-codes the same value in the Rust
//! — which is exactly the failure `tests/vector_hooks.rs` describes for the hook speed.
//!
//! ## Two things these tests do that need a reason
//!
//! 1. **They write `AimPoint` themselves**, through a system of their own, and put the
//!    carrier into `SpatialIndex` by hand. Same reason as in `tests/vector_hooks.rs`:
//!    `world::index::maintain_index` (`T-036a`) is a stub, so no body in the real world
//!    carries a `BodyId` yet and every real shot ends as `NoAnchor`.
//! 2. **The swing tests run with `Gravity(ZERO)`.** That is how the measurement was taken
//!    (`examples/probe_avian.rs::schwung_fahren`: gravity 0, anchor `L` above the player,
//!    `v0` sideways) and it is the only way the number means anything: with gravity on, a
//!    pendulum's speed swings by ±100 % from height alone, and what you would be measuring is
//!    `g`, not the solver. `Gravity` is a resource, so the test says so out loud instead of
//!    the code having a switch for it. The reel-in test keeps gravity **on** — there the
//!    number has to hold in the real world.

use avian3d::prelude::{DistanceJoint, Gravity, LinearVelocity, Position};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use defeated_by_titan::data::{GameData, RopeForceModel};
use defeated_by_titan::shared::{
    AimPoint, ArmAim, BodyId, BodyMask, Cli, Gas, HitStop, HitZone, Hook, HookReleased, HookState,
    IndexEntry,
    LocalPlayer, PlayerId, ReleaseReason, RopeLength, Side, SimulationSystems, SpatialIndex,
    TitanHit, TitanId, WarpPlayer,
};
use defeated_by_titan::vector::aim::aim;

// ---------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------

/// What `vector::aim` would write if `T-036a` were built. See the module header.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ForcedAim(AimPoint);

/// Writes the forced point into **both** carriers: the centre ray the crosshair reads
/// ([`AimPoint`]) and the per-arm ray `vector::hook` fires at ([`ArmAim`]).
///
/// Both, with the same value, because that is what the real `vector::aim` produces for a
/// target the whole spread covers — a side ray that finds nothing anchorable falls back to the
/// centre ray (`F-023`). This file is about the rope, not about the hemisphere split.
fn force_aim(mut players: Query<(&ForcedAim, &mut AimPoint, &mut ArmAim)>) {
    for (forced, mut point, mut arms) in &mut players {
        point.set_if_neq(forced.0);
        arms.set_if_neq(ArmAim { arms: [forced.0; 2] });
    }
}

/// Every `HookReleased` the run has produced, in order — the **noise** a release makes.
///
/// `B-003` was not a bug about a rope, it was a bug about silence: the joint outlived the
/// teleport and **nothing said so**. So the messages are recorded, not just the components.
#[derive(Resource, Default)]
struct Releases(Vec<(PlayerId, Side, ReleaseReason, u64)>);

fn collect_releases(mut log: ResMut<Releases>, mut messages: MessageReader<HookReleased>) {
    for m in messages.read() {
        log.0.push((m.player, m.side, m.reason, m.tick));
    }
}

/// Builds the **real** app, headless, one simulation step per `update()`.
fn app() -> App {
    let mut app = defeated_by_titan::app(Cli { headless: true, ..default() });
    // 🔴 **`Pendulum`, PINNED, and not because it is the default — because it is the SUBJECT.**
    // `FIND-152` wrote it down and 2026-08-23 collected the bill: *„all of tests/vector_rope.rs
    // is a statement about this line"*. Every `DistanceJoint`, every `B-005` ratchet, every
    // reel-in number in this file exists only under `Pendulum` — and yet the file read whichever
    // way `game.ron` happened to be set, so the day the shipped default moved to `Drive`
    // **thirteen of these went red at once** while measuring nothing that had changed. A test
    // whose subject is a tuning value must pin that value. The `FIND-149`/`FIND-153` tests below
    // override this with [`select`], which is what that helper is for.
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model =
        RopeForceModel::Pendulum;
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(1));
    app.init_resource::<Releases>();
    app.add_systems(FixedUpdate, force_aim.in_set(SimulationSystems::World).after(aim));
    // `PostStep`, so that a release written in `Intent` of the same tick is already in.
    app.add_systems(FixedUpdate, collect_releases.in_set(SimulationSystems::PostStep));
    app.update(); // Startup: the city and the local player come into being
    app
}

fn ticks(app: &mut App, n: u64) {
    for _ in 0..n {
        app.update();
    }
}

fn data(app: &App) -> GameData {
    app.world().resource::<GameData>().clone()
}

/// The one local player. Not `.single()` — every player is one of many (§6 rule 3).
fn me(app: &mut App) -> Entity {
    let mut q = app.world_mut().query_filtered::<Entity, With<LocalPlayer>>();
    q.iter(app.world()).next().expect("there must be a local player")
}

fn player_id(app: &App, e: Entity) -> PlayerId {
    *app.world().get::<PlayerId>(e).expect("every player has a stable id")
}

fn position(app: &App, e: Entity) -> Vec3 {
    app.world().get::<Position>(e).expect("the player is a physics body").0
}

fn velocity(app: &App, e: Entity) -> Vec3 {
    app.world().get::<LinearVelocity>(e).expect("the player is a physics body").0
}

fn set_velocity(app: &mut App, e: Entity, v: Vec3) {
    app.world_mut().get_mut::<LinearVelocity>(e).expect("the player is a physics body").0 = v;
}

fn rope_length(app: &App, e: Entity, side: Side) -> f32 {
    app.world()
        .get::<RopeLength>(e)
        .expect("every player carries a RopeLength")
        .length_m(side)
}

/// How many rope joints exist in the whole world.
fn joint_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&DistanceJoint>();
    q.iter(app.world()).count()
}

/// Warps the player exactly there and stops him dead. The sanctioned path (§12c) — and the
/// only one that does not fight avian over `Position`.
fn warp(app: &mut App, e: Entity, to: Vec3) {
    let id = player_id(app, e);
    app.world_mut().write_message(WarpPlayer {
        player: id,
        pos_x: to.x,
        pos_y: to.y,
        pos_z: to.z,
    });
}

/// Presses and holds the reel-in key. `src/net/local.rs` maps `ControlLeft` onto
/// `Buttons::REEL_IN` — the test presses the same key a human does.
fn hold_reel_in(app: &mut App) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ControlLeft);
}

/// Hangs the player on a rope of about `nominal_length_m`, at `player_pos`, with the anchor
/// straight **above** him. Returns the length the joint really got.
///
/// The player is put back onto his spot in **every** tick of the hook's flight, so that the
/// length the rope is born with is the one this function names and not one plus however far he
/// fell in the meantime. It stops the moment the hook bites.
///
/// ⚠️ **`place`, not `warp`, and this used to be a `warp`.** Since `B-003` a warp lets go of
/// every rope it leaves longer than it is (`player::rope::warp_keeps_the_rope`), and the warp
/// this harness used pinned the player against a hook that was still flying *outwards* — so the
/// rope grew under it every tick. `place` writes `Position` and does what this harness actually
/// means: move the body, touch nothing else.
fn hang(app: &mut App, e: Entity, player_pos: Vec3, nominal_length_m: f32) -> f32 {
    hang_on(app, e, player_pos, player_pos + Vec3::Y * nominal_length_m)
}

/// [`hang`] with the anchor spelled out instead of derived — the same harness for a rope that
/// is not vertical. `FIND-149`'s drive is a statement about the direction *the player is
/// looking along*, and an anchor straight overhead is the one geometry where the look gate
/// `max(0, l̂·r̂)` is zero for a player with his default pitch.
fn hang_on(app: &mut App, e: Entity, player_pos: Vec3, anchor: Vec3) -> f32 {
    let body = BodyId(80_001);
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m: anchor + Vec3::Y * 2.0,
        half_size_m: Vec3::splat(2.0),
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(anchor),
        body: Some(body),
        anchorable: true,
    }));

    place(app, e, player_pos);
    set_velocity(app, e, Vec3::ZERO);
    app.update();
    // `Q`, not the left mouse button. `src/net/local.rs` moved the hooks onto the keyboard on
    // 2026-08-10 ("the ropes have to be steerable") and put the blades on the mouse; the test
    // presses the key a human presses, and pressing the old one anchored nothing at all.
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);

    for _ in 0..600 {
        place(app, e, player_pos);
        set_velocity(app, e, Vec3::ZERO);
        app.update();
        let anchored = app
            .world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .expect("every player carries both arms")
            .arm(Side::Left)
            .state
            .is_anchored();
        if anchored {
            // One more tick without a warp, so `sync_rope_length` has published a length that
            // belongs to a player who really stands where he stands.
            app.update();
            let l = rope_length(app, e, Side::Left);
            assert!(l > 0.0, "the hook bit and no rope came into being");
            return l;
        }
    }
    panic!(
        "the hook did not bite within 600 ticks — it is {:?}",
        app.world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .map(|h| h.arm(Side::Left).state)
            .unwrap_or(HookState::Idle)
    );
}

/// Hangs the **second** arm on the same anchor the first one already holds.
///
/// Deliberately **without** a warp: since `B-003` a warp lets go of every rope, so warping
/// the player during the right hook's flight — the way [`hang`] does — would quietly kill the
/// left rope this function is supposed to add to. The player hangs still (no gravity, no
/// velocity), so he does not need holding in place.
fn hang_right(app: &mut App, e: Entity) -> f32 {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyE);
    for _ in 0..600 {
        app.update();
        if hook_state(app, e, Side::Right).is_anchored() {
            app.update(); // one tick, so `sync_rope_length` has published the length
            let l = rope_length(app, e, Side::Right);
            assert!(l > 0.0, "the right hook bit and no rope came into being");
            return l;
        }
    }
    panic!("the right hook did not bite within 600 ticks — it is {:?}", hook_state(app, e, Side::Right));
}

fn hook_state(app: &App, e: Entity, side: Side) -> HookState {
    app.world().get::<Hook>(e).expect("every player carries both arms").arm(side).state
}

/// Moves the body **without** a `WarpPlayer` — the rope survives this one.
///
/// Writing `Position` directly is what a warp deliberately does not do (`src/player/mod.rs`
/// writes the `Transform`), and it is the only way left to put a hanging player somewhere
/// else since a warp releases his ropes on purpose (`B-003`).
fn place(app: &mut App, e: Entity, to: Vec3) {
    app.world_mut().get_mut::<Position>(e).expect("the player is a physics body").0 = to;
}

fn kill_gravity(app: &mut App) {
    app.insert_resource(Gravity(Vec3::ZERO));
}

/// The anchor of the one rope that exists. Read out of the world, not remembered by the test
/// — that is what makes the "the joint is really gone" assertion mean something.
fn anchor_point(app: &mut App) -> Option<Vec3> {
    let mut q = app.world_mut().query::<&DistanceJoint>();
    let anchor = q.iter(app.world()).next().map(|j| j.body1)?;
    app.world().get::<Position>(anchor).map(|p| p.0)
}

// ---------------------------------------------------------------------------------------
// The overshoot measurement — `B-00X`
// ---------------------------------------------------------------------------------------

/// One tick of an approach run: everything the overshoot claim is made out of.
#[derive(Clone, Copy, Debug)]
struct Row {
    tick: u64,
    /// `RopeLength` — the length the joint really enforces (`limits.max`).
    enforced_m: f32,
    /// The distance the player really has to his anchor.
    distance_m: f32,
    /// How far past the anchor he is, along the direction he came from. Negative before.
    past_m: f32,
    speed_m_s: f32,
}

/// Flies the player straight at his anchor from `l0` metres out at `v0` m/s and records
/// every tick. The anchor sits **above** him ([`hang`]), so "at the anchor" is `+Y`.
///
/// Gravity off: the only thing that may bend this trajectory is the rope.
fn approach(l0_nominal_m: f32, v0_m_s: f32, reel: bool, ticks_n: u64) -> (f32, Vec<Row>) {
    let mut app = app();
    let e = me(&mut app);
    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), l0_nominal_m);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");
    app.world_mut()
        .get_mut::<defeated_by_titan::shared::Gas>(e)
        .expect("every player carries a tank")
        .unlimited = true;
    if reel {
        hold_reel_in(&mut app);
    }
    set_velocity(&mut app, e, Vec3::Y * v0_m_s);

    let mut rows = Vec::new();
    for t in 0..ticks_n {
        app.update();
        if reel {
            hold_reel_in(&mut app);
        }
        let p = position(&app, e);
        rows.push(Row {
            tick: t,
            enforced_m: rope_length(&app, e, Side::Left),
            distance_m: (p - anchor).length(),
            past_m: p.y - anchor.y,
            speed_m_s: velocity(&app, e).length(),
        });
    }
    (l0, rows)
}

#[test]
#[ignore = "measurement, not a criterion — run with --ignored --nocapture"]
fn measure_the_overshoot_past_the_anchor() {
    // `v_max` is the other half of the trade and it belongs next to `past_max`: a rope that
    // takes up slack cannot whip, and the whip is where the speed came from. The two numbers
    // are only an argument when they stand in the same row.
    println!(
        "{:>5} {:>6} {:>5} | {:>8} {:>8} {:>9} {:>8} {:>8}",
        "l0", "v0", "reel", "min_dist", "past_max", "enf_at_0", "enf_end", "v_max"
    );
    for reel in [false, true] {
        for v0 in [10.0f32, 20.0, 25.0, 28.0, 30.0, 40.0, 55.0, 75.0] {
            let (l0, rows) = approach(50.0, v0, reel, 240);
            let min_dist = rows.iter().fold(f32::MAX, |a, r| a.min(r.distance_m));
            let past_max = rows.iter().fold(f32::MIN, |a, r| a.max(r.past_m));
            // The enforced length on the tick he is closest to the anchor: that is the
            // slack he still has to fly out before anything stops him.
            let at_closest = rows
                .iter()
                .min_by(|a, b| a.distance_m.total_cmp(&b.distance_m))
                .copied()
                .expect("240 ticks");
            let v_max = rows.iter().fold(f32::MIN, |a, r| a.max(r.speed_m_s));
            println!(
                "{l0:>5.1} {v0:>6.1} {:>5} | {min_dist:>8.3} {past_max:>8.3} \
                 {:>9.3} {:>8.3} {v_max:>8.3}",
                if reel { "yes" } else { "no" },
                at_closest.enforced_m,
                rows.last().expect("240 ticks").enforced_m
            );
        }
    }
}

/// Swings the player on a rope of `l0_nominal_m` with **gravity on**, pushed sideways at
/// `v0_m_s`, and reports how far his distance to the anchor dips **below** the enforced
/// length. That dip is the whole risk of a slack take-up: whatever it eats, it eats once per
/// arc and never gives back.
#[test]
#[ignore = "measurement, not a criterion — run with --ignored --nocapture"]
fn measure_the_dip_of_a_swing_below_its_own_length() {
    println!(
        "{:>6} {:>6} {:>8} | {:>9} {:>9} {:>9} {:>9}",
        "grav", "v0", "l0", "min_dist", "max_dist", "worst_dip", "enf_end"
    );
    for gravity in [true, false] {
        for v0 in [8.0f32, 12.0, 16.0, 20.0, 30.0] {
            let mut app = app();
            let e = me(&mut app);
            if !gravity {
                kill_gravity(&mut app);
            }
            let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 8.0);
            let anchor = anchor_point(&mut app).expect("a rope has an anchor");
            set_velocity(&mut app, e, Vec3::new(v0, 0.0, 0.0));

            let (mut min_d, mut max_d, mut worst_dip) = (f32::MAX, f32::MIN, 0.0f32);
            for _ in 0..240 {
                app.update();
                let d = (position(&app, e) - anchor).length();
                let enforced = rope_length(&app, e, Side::Left);
                min_d = min_d.min(d);
                max_d = max_d.max(d);
                worst_dip = worst_dip.max(enforced - d);
            }
            println!(
                "{:>6} {v0:>6.1} {l0:>8.3} | {min_d:>9.4} {max_d:>9.4} {worst_dip:>9.4} {:>9.4}",
                if gravity { "on" } else { "off" },
                rope_length(&app, e, Side::Left)
            );
        }
    }
}

/// `FIND-026` asked whether a rope contributes anything in the **real city**. It measured
/// **0.000**: from the tallest roof, hooking the church, the run with the rope was
/// bit-identical to the run without one, because 51.29 m of rope against 18.5 m of anchor
/// height puts the bottom of the arc 24.3 m underground.
///
/// This re-runs that in the graybox, with a **real** anchorable body out of `maps.ron`, and
/// adds the case the take-up is for: the player closes on his anchor first.
#[test]
#[ignore = "measurement, not a criterion — run with --ignored --nocapture"]
fn measure_whether_a_pendulum_exists_in_the_graybox() {
    for closing_m_s in [0.0f32, 20.0, 40.0, 75.0] {
        let mut app = app();
        let e = me(&mut app);
        // `world::index::maintain_index` hands out the `BodyId`s in `FixedUpdate`, and its
        // `Commands` land at the next sync point — one startup tick is not enough.
        ticks(&mut app, 5);

        // The tallest anchorable body in the real map — the church, found in the world and
        // not remembered from a document.
        let mut q = app
            .world_mut()
            .query::<(&BodyId, &defeated_by_titan::shared::Body, &GlobalTransform)>();
        let (body, top_m) = q
            .iter(app.world())
            .filter(|(_, b, _)| b.mask.contains(BodyMask::ANCHORABLE))
            .map(|(id, b, t)| (*id, t.translation() + Vec3::Y * b.half_size_m.y))
            .max_by(|a, b| a.1.y.total_cmp(&b.1.y))
            .expect("the graybox has anchorable bodies");
        let anchor_m = top_m - Vec3::Y * 0.05;

        // On the roof of the tallest house a player can stand on (`FIND-026`: 11.5 m), at the
        // horizontal distance that gives the 51.29 m rope of that finding.
        let start = Vec3::new(anchor_m.x + 47.83, 11.5, anchor_m.z);
        app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
            point_m: Some(anchor_m),
            body: Some(body),
            anchorable: true,
        }));
        place(&mut app, e, start);
        set_velocity(&mut app, e, Vec3::ZERO);
        app.update();
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyQ);
        for _ in 0..600 {
            place(&mut app, e, start);
            set_velocity(&mut app, e, Vec3::ZERO);
            app.update();
            if hook_state(&app, e, Side::Left).is_anchored() {
                break;
            }
        }
        app.update();
        let l0 = rope_length(&app, e, Side::Left);

        // Closing on the anchor first — what a player does with gas, injected as velocity so
        // that the aim is not what is being measured.
        if closing_m_s > 0.0 {
            let toward = (anchor_m - position(&app, e)).normalize();
            set_velocity(&mut app, e, toward * closing_m_s);
            for _ in 0..60 {
                app.update();
            }
        }
        let l_after_close = rope_length(&app, e, Side::Left);
        let anchor_height_m = anchor_m.y - position(&app, e).y;

        // And then: let go of everything and fall. Real gravity, real ground.
        set_velocity(&mut app, e, Vec3::ZERO);
        let (mut peak_speed, mut lowest, mut taut_ticks) = (0.0f32, f32::MAX, 0u32);
        for _ in 0..300 {
            app.update();
            let p = position(&app, e);
            peak_speed = peak_speed.max(velocity(&app, e).length());
            lowest = lowest.min(p.y);
            if (p - anchor_m).length() >= rope_length(&app, e, Side::Left) - 0.02 {
                taut_ticks += 1;
            }
        }
        println!(
            "closing {closing_m_s:>5.1} m/s | rope {l0:>7.3} -> {l_after_close:>7.3} m · \
             anchor {anchor_height_m:>6.2} m above him · arc bottom \
             {:>7.2} m · peak {peak_speed:>6.3} m/s · lowest {lowest:>6.2} m · taut \
             {taut_ticks:>3} of 300 ticks",
            anchor_m.y - l_after_close
        );
    }
}

#[test]
#[ignore = "measurement, not a criterion — run with --ignored --nocapture"]
fn measure_one_approach_tick_by_tick() {
    let (l0, rows) = approach(50.0, 75.0, true, 90);
    println!("l0 = {l0:.3} m, v0 = 75 m/s, reel held");
    println!("{:>5} {:>10} {:>10} {:>9} {:>9}", "tick", "enforced", "distance", "past", "speed");
    for r in &rows {
        println!(
            "{:>5} {:>10.3} {:>10.3} {:>9.3} {:>9.3}",
            r.tick, r.enforced_m, r.distance_m, r.past_m, r.speed_m_s
        );
    }
}

// ---------------------------------------------------------------------------------------
// `B-004` — the rope is a ratchet, or the player flies past his own anchor
// ---------------------------------------------------------------------------------------

#[test]
fn b004_a_player_flying_at_his_anchor_does_not_pass_it() {
    // **The bug the user played into.** „das seil hat eine maximal einhol dauer … wenn ich
    // mich festhake und ganz schnell ran fliege kann ich overshooten" — and he is right twice
    // over. Measured before the fix, on a 50.000 m rope:
    //
    //   reel NOT held : the enforced length stayed 50.000 m and he flew **50.000 m past the
    //                   anchor** at 20, 25, 28, 30, 40, 55 and 75 m/s — every speed, the whole
    //                   rope length. Without the button the length never shrinks at all.
    //   reel held     : the overshoot begins exactly at `vector.reel_speed_m_s` and grows —
    //                   8.667 m at 40 m/s, 16.000 m at 55, 22.500 m at 75.
    //
    // The criterion is the geometry and not a tolerance: `min_rope_m` is the shortest the rope
    // may be, so the furthest side of the anchor he may ever reach is `min_rope_m`. Anything
    // beyond that is slack the rope was given and did not take in.
    //
    // Gravity off — the only thing that may bend this trajectory is the rope.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let min_rope_m = d.game.vector.min_rope_m;
    let reel_speed_m_s = d.game.vector.reel_speed_m_s;
    let v0 = d.game.vector.max_speed_m_s;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 50.0);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");
    // The reel is deliberately NOT held: the take-up has to work without the button, or the
    // half of the bug the user did not name stays in.
    set_velocity(&mut app, e, Vec3::Y * v0);

    let mut past_max = f32::MIN;
    let mut enforced_at_worst = l0;
    for _ in 0..240 {
        app.update();
        let past = position(&app, e).y - anchor.y;
        if past > past_max {
            past_max = past;
            enforced_at_worst = rope_length(&app, e, Side::Left);
        }
    }

    assert!(
        past_max <= min_rope_m + 0.05,
        "flying at {v0} m/s (game.ron: vector.max_speed_m_s) into a {l0:.3} m rope the player \
         ended up {past_max:.3} m on the far side of his anchor, with {enforced_at_worst:.3} m \
         of rope still enforced. The floor is {min_rope_m} m (vector.min_rope_m) — everything \
         past that is slack the rope was given and never took in. Reeling cannot fix it: the \
         reel does {reel_speed_m_s} m/s and he does {v0}."
    );
}

#[test]
fn b004_the_enforced_length_never_grows() {
    // The user's other sentence: „wenn mit seilen verbunden und wurde kürzer soll erstmal
    // nicht länger werden". A ratchet is a claim about **every** tick, not about the ends, so
    // this run varies what the player does — fly in, fly out, swing, dive, climb — and checks
    // the length against the shortest it has ever been, tick by tick.
    //
    // **The reel is never held.** `F-005` has three tests of its own; if this one reeled, the
    // closing assertion ("it really did get shorter") would be satisfied by the reel and the
    // take-up could be missing entirely without a single line going red.
    let mut app = app();
    let e = me(&mut app);

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 40.0);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    let mut longest = l0;
    for phase in 0..6 {
        // Six different things to do to a rope, 60 ticks each. The velocities are directions,
        // not tuning: toward the anchor, away from it, sideways, and again.
        let v = match phase {
            0 => Vec3::Y * 30.0,
            1 => Vec3::X * 25.0,
            2 => Vec3::new(15.0, -20.0, 10.0),
            3 => Vec3::NEG_Y * 18.0,
            4 => Vec3::new(-22.0, 12.0, -6.0),
            _ => Vec3::Y * 45.0,
        };
        set_velocity(&mut app, e, v);
        for _ in 0..60 {
            app.update();
            let now = rope_length(&app, e, Side::Left);
            assert!(
                now <= longest + 1e-4,
                "the rope had already been down to {longest:.4} m and is {now:.4} m again \
                 (phase {phase}, anchor {anchor:?}) — a rope that has been taken in does not \
                 pay the slack back out"
            );
            longest = longest.min(now);
        }
    }
    assert!(
        longest < l0 - 1.0,
        "over six phases of flying at and around the anchor, without the reel ever being held, \
         the {l0:.3} m rope never got shorter than {longest:.3} m — the slack the player flew \
         in was never taken up"
    );
}

#[test]
fn b004_a_swing_keeps_its_length_across_the_arc() {
    // **The regression guard for the one thing the take-up could have eaten.** A pendulum
    // needs a constant length; a take-up that bit on the solver's own error would shorten the
    // rope every arc and ratchet the player into his anchor.
    //
    // Measured before the take-up existed: over 4 s on an 8.000 m rope the distance to the
    // anchor dipped below the enforced length by **0.0000 m** at v0 8, 12, 16 and 30 m/s,
    // gravity on and off. So there is nothing legitimate for the ratchet to bite on, and if
    // this test goes red the take-up has started eating the swing itself.
    //
    // v0 = 12 m/s on an 8 m rope at g = -20: the apex is 12²/(2·20) = 3.6 m of 8, so he stays
    // in the lower half of the arc and the rope is taut the whole way — the normal swing.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as u64;

    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 8.0);
    set_velocity(&mut app, e, Vec3::X * 12.0);

    let mut shortest = l0;
    for _ in 0..4 * hz {
        app.update();
        shortest = shortest.min(rope_length(&app, e, Side::Left));
    }

    assert!(
        (l0 - shortest).abs() < 0.01,
        "over four seconds of a normal swing the {l0:.4} m rope was ratcheted down to \
         {shortest:.4} m — {:.4} m gone that no reel-in paid for. The arc measured 0.0000 m of \
         dip below its own length before the take-up existed, so this is the take-up eating \
         the swing.",
        l0 - shortest
    );
}

#[test]
fn b004_anchoring_still_does_not_yank() {
    // `attach_ropes` is born at the distance that really exists "so anchoring never yanks the
    // player" (`src/player/rope.rs`). The take-up runs in the same tick — so this test is here
    // to say that it changes nothing at the moment of the bite. It cannot: the length starts
    // equal to the distance, and `min(length, distance)` of two equal numbers is that number.
    let mut app = app();
    let e = me(&mut app);

    kill_gravity(&mut app);
    let nominal_m = 20.0;
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), nominal_m);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");
    let real_m = (position(&app, e) - anchor).length();

    assert!(
        (l0 - real_m).abs() < 0.01,
        "the rope was born {l0:.4} m long at a real distance of {real_m:.4} m — a rope that is \
         born shorter than the distance yanks the player the moment it exists"
    );
    // And he stays put: no velocity, no gravity, so any movement at all is the rope pulling.
    set_velocity(&mut app, e, Vec3::ZERO);
    let before = position(&app, e);
    ticks(&mut app, 30);
    let moved_m = (position(&app, e) - before).length();
    assert!(
        moved_m < 0.01,
        "half a second after the hook bit at {l0:.3} m the player has moved {moved_m:.4} m \
         without a single input — that is the yank"
    );
}

// ---------------------------------------------------------------------------------------
// `F-005` — the reel-in, and the two numbers the whole round hangs on
// ---------------------------------------------------------------------------------------

#[test]
fn f005_reeling_in_gains_speed_beyond_the_start() {
    // **The criterion the round hangs on.** Reeling in through the joint preserves angular
    // momentum: `v * L` stays put, so a third of the length is three times the speed. The
    // measurement got 58.23 m/s out of `v0 = 20`; the hand-written solver that was retired
    // got exactly 20.000 — it ate the reel-in, and the reel-in is the feel of the gear.
    //
    // Gravity stays ON here. This number has to hold in the real world, not in a vacuum.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let min_rope_m = d.game.vector.min_rope_m;
    let max_speed_m_s = d.game.vector.max_speed_m_s;

    // Three times the floor, so that "about a third of the start length" is exactly the floor
    // and the test does not have to guess where to stop.
    let start_length_m = min_rope_m * 3.0;
    let v0 = 20.0;

    // High above the city: this test measures the rope, not a roof.
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), start_length_m);
    set_velocity(&mut app, e, Vec3::new(v0, 0.0, 0.0));

    // Measured **at the end of the reel-in**, not some fixed number of ticks later. Once the
    // rope is at its floor the pendulum starts trading the speed back for height against
    // gravity — that is physics, not the rope, and a test that waits measures `g`. (For the
    // record: 120 ticks later the same run reads 41.55 m/s.)
    hold_reel_in(&mut app);
    let mut peak = 0.0f32;
    let mut speed = 0.0f32;
    let mut end_length_m = l0;
    for _ in 0..120 {
        app.update();
        peak = peak.max(velocity(&app, e).length());
        speed = velocity(&app, e).length();
        end_length_m = rope_length(&app, e, Side::Left);
        if end_length_m <= min_rope_m + 1e-4 {
            break;
        }
    }

    assert!(
        (end_length_m - min_rope_m).abs() < 0.01,
        "the rope ran from {l0:.3} m to {end_length_m:.3} m, expected the floor at \
         {min_rope_m} m (game.ron: vector.min_rope_m) — the reel-in did not run"
    );
    assert!(
        speed >= 45.0,
        "from v0 = {v0} m/s at {l0:.2} m down to {end_length_m:.2} m the player reached only \
         {speed:.2} m/s (peak {peak:.2}). The joint has to preserve angular momentum — the \
         measurement got 58.23 m/s. A clamp that eats the reel-in gives back exactly v0."
    );
    assert!(
        peak <= max_speed_m_s + 0.1,
        "the player reached {peak:.2} m/s, the file allows {max_speed_m_s} \
         (game.ron: vector.max_speed_m_s) — MaxLinearSpeed is not on the body"
    );
}

#[test]
fn f005_shortening_happens_per_substep_not_per_tick() {
    // **The test that goes red when somebody "simplifies" the substep system into
    // `FixedUpdate`.** One tick of reeling shortens the rope by `reel_speed / simulation_hz`
    // — once, not once per substep. Shortening per tick instead injects
    // `rate x SubstepCount` and the measurement watched the player reach 677.66 m/s and go
    // 2.53 m through a wall.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as f32;
    let substeps = d.game.substeps as f32;
    let per_tick_m = d.game.vector.reel_speed_m_s / hz;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 4.0);

    hold_reel_in(&mut app);
    app.update();
    let after_one_tick = rope_length(&app, e, Side::Left);
    let shortened = l0 - after_one_tick;

    assert!(
        (shortened - per_tick_m).abs() <= per_tick_m * 0.01,
        "one tick of reeling took {shortened:.5} m off the rope; the file says \
         {per_tick_m:.5} m (vector.reel_speed_m_s / simulation_hz) ± 1 %. \
         {:.5} m would be once per substep applied {substeps} times over — that is the \
         per-tick bug the measurement clocked at 677.66 m/s.",
        per_tick_m * substeps
    );
}

#[test]
fn f005_the_rope_never_gets_shorter_than_the_file_says() {
    // `vector.min_rope_m`: any closer would drag the camera into the wall. Read from
    // `GameData`, never from a literal — and reeled at for far longer than it takes to get
    // there, so a missing clamp runs the length negative and the joint starts pushing.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let min_rope_m = d.game.vector.min_rope_m;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), min_rope_m * 4.0);
    // `unlimited` gas, so that an empty tank is not what stops the reel-in.
    app.world_mut()
        .get_mut::<defeated_by_titan::shared::Gas>(e)
        .expect("every player carries a tank")
        .unlimited = true;

    hold_reel_in(&mut app);
    let mut shortest = l0;
    for _ in 0..300 {
        app.update();
        shortest = shortest.min(rope_length(&app, e, Side::Left));
    }

    assert!(
        shortest >= min_rope_m - 1e-4,
        "the rope got down to {shortest:.5} m, the file allows {min_rope_m} m \
         (game.ron: vector.min_rope_m)"
    );
    assert!(
        (shortest - min_rope_m).abs() < 0.01,
        "the rope stopped at {shortest:.5} m instead of at the floor {min_rope_m} m — it is \
         not the file that stopped it"
    );
}

// ---------------------------------------------------------------------------------------
// `F-004` — the pendulum
// ---------------------------------------------------------------------------------------

#[test]
fn f004_a_swing_loses_little_speed_per_second() {
    // Without gravity a pendulum is a pure circle and **every** loss of speed is solver
    // damping. That is how `examples/probe_avian.rs::schwung_fahren` measured it: 4.26 %/s at
    // 24 substeps. The criterion is 6 %/s. For scale: the pure radial projection that was
    // considered and rejected loses 99.2 %/s at `L = 3 m, v = 75 m/s`.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as u64;

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), 8.0);

    let v0 = 20.0;
    set_velocity(&mut app, e, Vec3::new(v0, 0.0, 0.0));
    ticks(&mut app, hz); // exactly one second

    let end = velocity(&app, e).length();
    let loss_per_s = (1.0 - end / v0) * 100.0;
    assert!(
        loss_per_s <= 6.0,
        "over one second on a {l0:.2} m rope the swing fell from {v0:.2} to {end:.2} m/s — \
         {loss_per_s:.2} %/s, the criterion is 6 %/s (measured 4.26 %/s at \
         {} substeps)",
        d.game.substeps
    );
    // And the other direction: a rope that gains speed out of nothing is not a rope.
    assert!(
        end <= v0 * 1.01,
        "the swing ended at {end:.2} m/s having started at {v0:.2} — a constraint that adds \
         energy is not a constraint"
    );
}

#[test]
fn f004_the_rope_pulls_but_does_not_push() {
    // `limits.min = 0.0` is what says so: `DistanceLimit::compute_correction` corrects only
    // above the maximum. A player who is closer to his anchor than `L` has to be able to stay
    // there — a rope is not a rod, and pushing is what a spring would do.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 4.0);
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    // Straight at the anchor, at half his rope's length, and then let go of everything.
    //
    // `place` and **not** `warp`: since `B-003` a warp releases every rope of the player it
    // moves, and this test needs the rope alive to have anything to measure. The old version
    // of this test warped here and would have gone on passing with no joint in the world at
    // all — a green test measuring nothing.
    let half = l0 * 0.5;
    place(&mut app, e, anchor - Vec3::Y * half);
    app.update();
    set_velocity(&mut app, e, Vec3::ZERO);
    assert_eq!(joint_count(&mut app), 1, "the rope has to still be there to be measured");

    let start = (position(&app, e) - anchor).length();
    ticks(&mut app, 60);
    let end = (position(&app, e) - anchor).length();

    assert!(
        end <= start + 0.05,
        "the player sat {start:.3} m under an anchor whose rope is {l0:.3} m long and was \
         pushed out to {end:.3} m. A rope pulls; it does not push."
    );
    assert!(
        end < l0 - 0.5,
        "he ended up at {end:.3} m of {l0:.3} m — something drove him to the full length"
    );
}

// ---------------------------------------------------------------------------------------
// `B-003` — a teleport that drags a rope behind it, and says nothing
// ---------------------------------------------------------------------------------------

#[test]
fn b003_a_warp_lets_go_of_every_rope() {
    // **The bug this test was written for.** `scripts/game-full.txt` warped the player 55 m
    // away in ACTS 2 and 3 while ACT 1's rope was still attached. He was not falling onto the
    // nape, he was being yanked back toward an anchor 55 m behind him — two of three kills
    // silently did not happen, and not one line of the log said so.
    //
    // Both arms, because "every rope" is the claim and one side proves half of it.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let hz = d.game.simulation_hz as u64;

    // Gravity off: then the only thing in the world that can move the player away from the
    // spot the warp put him on is the rope, and the drag distance is the whole measurement.
    kill_gravity(&mut app);
    let home = Vec3::new(0.0, 120.0, 0.0);
    let l_left = hang(&mut app, e, home, d.game.vector.min_rope_m * 3.0);
    let l_right = hang_right(&mut app, e);
    assert_eq!(joint_count(&mut app), 2, "two hooks, two joints");
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    // 55 m — the distance `scripts/game-full.txt` warps over, and far beyond a 9 m rope.
    let far = home + Vec3::X * 55.0;
    let asked_for_m = (far - anchor).length();
    warp(&mut app, e, far);
    app.update();

    let left_over = joint_count(&mut app);
    let drag_m = (position(&app, e) - far).length();
    assert_eq!(
        left_over, 0,
        "one tick after a warp of {asked_for_m:.2} m, {left_over} joint(s) are still holding \
         the player: ropes of {l_left:.2} m and {l_right:.2} m. He was dragged {drag_m:.2} m \
         away from the coordinate he was warped to, back toward his old anchor."
    );
    assert_eq!(
        [rope_length(&app, e, Side::Left), rope_length(&app, e, Side::Right)],
        [0.0, 0.0],
        "`RopeLength` still claims a constraint on a player who was teleported away — \
         0.0 means 'no constraint' (src/shared/gear.rs)"
    );

    // The tips have to come home before the arms are free again: the hand is 55 m from the
    // old anchor and `vector.hook_retract_speed_m_s` decides how long that takes. Four
    // seconds is many times the 0.46 s the file's 120 m/s need for it.
    ticks(&mut app, 4 * hz);
    assert_eq!(
        [hook_state(&app, e, Side::Left), hook_state(&app, e, Side::Right)],
        [HookState::Idle, HookState::Idle],
        "the ropes are gone but the arms still believe they are holding on — an arm that never \
         comes back to `Idle` can never fire again (`vector::hook`, decision 1)"
    );

    // And he really stands where the warp put him (§12c: "the player stands exactly there").
    let drift_m = (position(&app, e) - far).length();
    assert!(
        drift_m < 0.05,
        "{:.2} s after the warp the player is {drift_m:.2} m off the coordinate he was warped \
         to, {:.2} m from his old anchor — something is still pulling him",
        4.0,
        (position(&app, e) - anchor).length()
    );
}

#[test]
fn b003_a_warp_that_lets_go_says_so_out_loud() {
    // **The other half of `B-003`, and the expensive half.** A teleport that drops a rope
    // without a word is a teleport nobody can debug: `hud` and `sound` read
    // `HookReleased.reason` (`src/shared/message.rs`), and a run's log is the only thing a
    // `--headless` script leaves behind. A fix that releases the rope in silence has fixed
    // the yank and left the day-costing part in place.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let id = player_id(&app, e);

    kill_gravity(&mut app);
    let home = Vec3::new(0.0, 120.0, 0.0);
    hang(&mut app, e, home, d.game.vector.min_rope_m * 3.0);
    assert!(hook_state(&app, e, Side::Left).is_anchored(), "the left arm has to be holding on");

    // Everything said before the warp is somebody else's business.
    let before = app.world().resource::<Releases>().0.len();
    warp(&mut app, e, home + Vec3::X * 55.0);
    ticks(&mut app, 3);

    let said: Vec<_> = app.world().resource::<Releases>().0[before..].to_vec();
    assert!(
        said.iter().any(|(p, s, _, _)| *p == id && *s == Side::Left),
        "the warp let go of a rope the player was hanging on and wrote no `HookReleased` for \
         it — {} message(s) in three ticks: {said:?}. That silence is `B-003`.",
        said.len()
    );
}

#[test]
fn f004_releasing_the_hook_removes_the_joint() {
    // Every release reason goes through `HookReleased`. A joint that outlives its hook is an
    // invisible rope: the player hangs on nothing anybody can see, and no message will ever
    // come to free him.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);

    kill_gravity(&mut app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 120.0, 0.0), d.game.vector.min_rope_m * 3.0);
    assert_eq!(joint_count(&mut app), 1, "one hook, one joint");
    let anchor = anchor_point(&mut app).expect("a rope has an anchor");

    // Let go of the hook key — the same input a human gives (`Q` since 2026-08-10).
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::KeyQ);
    set_velocity(&mut app, e, Vec3::new(20.0, 0.0, 0.0));
    app.update();

    assert!(
        !app.world()
            .get::<defeated_by_titan::shared::Hook>(e)
            .expect("every player carries both arms")
            .arm(Side::Left)
            .state
            .is_anchored(),
        "the hook did not let go"
    );
    assert_eq!(
        joint_count(&mut app),
        0,
        "the hook let go and {} joint(s) are still holding the player",
        joint_count(&mut app)
    );
    assert_eq!(
        rope_length(&app, e, Side::Left),
        0.0,
        "`RopeLength` still claims a constraint that no longer exists — 0.0 means \
         'no constraint' (src/shared/gear.rs)"
    );

    // And he really flies free: 60 ticks at 20 m/s put him far beyond his old rope's length.
    ticks(&mut app, 60);
    let distance = (position(&app, e) - anchor).length();
    assert!(
        distance > l0 * 2.0,
        "one second after letting go he is {distance:.2} m from the anchor, his rope was \
         {l0:.2} m — something is still holding him"
    );
}

/// ★ **`B-004`, second face — a frozen player does not spool rope.**
///
/// `combat::hitstop` takes the body out of the solver for the impact frame, and before this
/// test [`shorten_ropes`](defeated_by_titan::player::rope::shorten_ropes) went on shortening
/// `limits.max` right through it: rope the player cannot follow, stored and paid back in the
/// first tick he moves again. In `scripts/f-flight-cut.txt` that was **0.93 m over the two
/// frozen ticks of a torso hit** — `vector.reel_speed_m_s` 28 / 60 Hz x 2 — cashed in as
/// 74.700 m/s, which is `vector.max_speed_m_s` and therefore the clamp and not a speed anybody
/// chose. A cortex freeze is 7 ticks and would store 3.27 m.
///
/// The criterion is the one number that can be read from outside: the enforced length does not
/// move by a millimetre while `HitStop` is on the player, with the reel held down the whole
/// time.
#[test]
fn b004_a_frozen_player_does_not_spool_rope() {
    let mut app = app();
    kill_gravity(&mut app);
    let e = me(&mut app);
    let d = data(&app);
    let l0 = hang(&mut app, e, Vec3::new(0.0, 60.0, 0.0), 30.0);

    // The reel, held from here to the end — the length has to fall before the freeze, or the
    // test would be green on a rope that never moved at all.
    hold_reel_in(&mut app);
    ticks(&mut app, 6);
    let before = rope_length(&app, e, Side::Left);
    assert!(
        before < l0 - 0.5,
        "the reel took in {:.3} m in six ticks — nothing is being measured here",
        l0 - before
    );

    // The cut. `TitanHit` is the message `combat::hitstop::begin` reacts to.
    let id = *app.world().get::<PlayerId>(e).expect("the player carries his id");
    app.world_mut().write_message(TitanHit {
        titan: TitanId(1),
        by: id,
        zone: HitZone::Cortex,
        speed_m_s: 30.0,
    });
    ticks(&mut app, 1);
    assert!(
        app.world().get::<HitStop>(e).is_some(),
        "the hit did not freeze the player, so this test never met the bug"
    );
    let at_freeze = rope_length(&app, e, Side::Left);

    // Five ticks inside the seven-tick cortex freeze.
    ticks(&mut app, 5);
    let inside = rope_length(&app, e, Side::Left);
    assert!(app.world().get::<HitStop>(e).is_some(), "the freeze ended too early to measure");
    let stored_m = at_freeze - inside;
    assert_eq!(
        inside, at_freeze,
        "the rope was taken in by {stored_m:.4} m over five frozen ticks while the body could \
         not follow — at {} m/s that is what the first unfrozen tick pays back as a clamped \
         {} m/s (B-004)",
        d.game.vector.reel_speed_m_s, d.game.vector.max_speed_m_s
    );

    // And it starts again when the freeze lifts — a fix that simply stops the reel is wrong.
    ticks(&mut app, 6);
    assert!(app.world().get::<HitStop>(e).is_none(), "the freeze never ended");
    let after = rope_length(&app, e, Side::Left);
    assert!(
        after < inside,
        "the reel never started again after the freeze: {inside:.3} m -> {after:.3} m"
    );
    println!(
        "B-004 second face: {before:.3} m -> {at_freeze:.3} m at the freeze, {inside:.3} m five \
         frozen ticks later, {after:.3} m six ticks after it lifted"
    );
}

// ---------------------------------------------------------------------------------------
// `B-003`, the second half — WHICH warps let go, and which do not
// ---------------------------------------------------------------------------------------
//
// The fix of 2026-08-10 released **every** rope on **every** `warp`, whatever the distance.
// That is right for the case it was written for — a 55 m teleport off a 9 m rope dragged the
// player 47.93 m back in one tick — and wrong for the 35 scripts that use `warp` as a way to
// put a player somewhere: it cost the `F-029` round two runs, and it is written down nowhere a
// script author looks (`docs/FINDINGS.md` FIND-116).
//
// The rule is not a taste, it is what the joint does. `limits = (0, L)` corrects only when the
// distance **exceeds** `L` (`avian3d-0.7.0/src/dynamics/joints/mod.rs:329-343`), so a teleport
// that leaves the player inside his own rope length cannot yank him at all — the rope is slack,
// and `shorten_ropes` spools the slack in. Only the excess is a drag, and it is a drag of
// roughly its own size: 55.73 m off a 9.00 m rope is 46.73 m of excess and measured 47.93 m of
// drag. `vector.warp_rope_slack_m` is how much excess is allowed to survive.

/// The distance from the warp destination to the one anchor in the world.
fn distance_to_anchor(app: &mut App, to: Vec3) -> f32 {
    (to - anchor_point(app).expect("a rope has an anchor")).length()
}

#[test]
fn b003_a_warp_inside_the_rope_length_keeps_the_rope() {
    // ★ The bug half that is being fixed today. Five centimetres sideways off a 9 m rope
    // leaves the player 9.0001 m from his anchor — 0.0001 m of excess on a rope that is
    // allowed `vector.warp_rope_slack_m` of it — so there is nothing for the joint to correct
    // and nothing that could pull him anywhere. Cutting it is a rope lost for free.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    let id = player_id(&app, e);
    kill_gravity(&mut app);

    let home = Vec3::new(0.0, 120.0, 0.0);
    let l = hang(&mut app, e, home, d.game.vector.min_rope_m * 3.0);
    assert_eq!(joint_count(&mut app), 1, "one hook, one joint");
    let before = app.world().resource::<Releases>().0.len();

    let nudge = home + Vec3::X * 0.05;
    let reach_m = distance_to_anchor(&mut app, nudge);
    let excess_m = reach_m - l;
    println!(
        "b003 nudge: rope {l:.4} m, the warp leaves him {reach_m:.4} m from the anchor — \
         {excess_m:.4} m of excess against a slack of {:.4} m",
        d.game.vector.warp_rope_slack_m
    );
    assert!(
        excess_m < d.game.vector.warp_rope_slack_m,
        "the fixture does not test what it says it does"
    );
    warp(&mut app, e, nudge);
    ticks(&mut app, 3);

    assert_eq!(
        joint_count(&mut app),
        1,
        "a warp of 0.05 m cut a rope of {l:.2} m. The joint had {excess_m:.4} m to correct, \
         which the solver returns as 0.14 m/s — a fortieth of the speed the player runs at. \
         That is B-003's second half."
    );
    assert!(
        hook_state(&app, e, Side::Left).is_anchored(),
        "the arm let go although its rope is still there — {:?}",
        hook_state(&app, e, Side::Left)
    );
    assert!(
        rope_length(&app, e, Side::Left) > 0.0,
        "`RopeLength` reports no constraint on an arm that is still holding on"
    );
    let said: Vec<_> = app.world().resource::<Releases>().0[before..].to_vec();
    assert!(
        !said.iter().any(|(p, s, _, _)| *p == id && *s == Side::Left),
        "the arm was told to let go of a rope that survived: {said:?}"
    );
    // §12c still holds *within what the solver can do with 0.0001 m*: the rope has 0.0001 m
    // of excess, the solver corrects it in one substep, and that comes back out as 0.143 m/s.
    // What must not happen is the thing `B-003` was written for — being pulled back toward the
    // old anchor. The drift stays under the nudge itself, and the speed under the bound the
    // key is chosen by.
    let drift_m = (position(&app, e) - nudge).length();
    let speed_m_s = velocity(&app, e).length();
    println!("b003 nudge: {drift_m:.4} m of drift, {speed_m_s:.3} m/s three ticks later");
    assert!(
        drift_m < 0.05,
        "he is {drift_m:.4} m off the coordinate he was warped to — further than the 0.05 m \
         the warp moved him, so something is pulling him somewhere"
    );
    assert!(
        speed_m_s < d.game.player.run_speed_m_s,
        "a kept rope kicked him to {speed_m_s:.3} m/s, faster than he runs — \
         `vector.warp_rope_slack_m` is too big"
    );
}

#[test]
fn b003_a_warp_past_the_rope_length_still_lets_go() {
    // The other side of the same line, and the reason the 2026-08-10 fix exists at all. Ten
    // metres sideways off a 9 m rope is 4.45 m of excess — 17 times the slack — and every
    // centimetre of it is a yank the player never asked for.
    let mut app = app();
    let e = me(&mut app);
    let d = data(&app);
    kill_gravity(&mut app);

    let home = Vec3::new(0.0, 120.0, 0.0);
    let l = hang(&mut app, e, home, d.game.vector.min_rope_m * 3.0);
    let far = home + Vec3::X * 10.0;
    let reach_m = distance_to_anchor(&mut app, far);
    println!(
        "b003 past: rope {l:.4} m, the warp leaves him {reach_m:.4} m from the anchor — \
         {:.4} m of excess",
        reach_m - l
    );
    assert!(reach_m - l > d.game.vector.warp_rope_slack_m, "the fixture has no excess to cut on");
    warp(&mut app, e, far);
    ticks(&mut app, 3);

    assert_eq!(
        joint_count(&mut app),
        0,
        "a warp that leaves the player {:.2} m outside a {l:.2} m rope kept the joint — and a \
         joint with {:.2} m to correct drags him back in one tick",
        reach_m - l,
        reach_m - l
    );
    let drift_m = (position(&app, e) - far).length();
    assert!(drift_m < 0.05, "he was dragged {drift_m:.2} m off the coordinate he was warped to");
}

#[test]
fn b003_the_warp_slack_is_bounded_by_the_files_own_numbers() {
    // Not a taste and not `serde(default)` — a bound, at both ends, out of the same file.
    let d = GameData::load(std::path::Path::new("assets/data"));
    let slack_m = d.game.vector.warp_rope_slack_m;
    assert!(
        slack_m > 0.0,
        "at 0.00 m every warp is 'outside the rope' by one float ULP and the rule is the old \
         cut-everything again"
    );
    // ★ The bound, and it is a **speed** bound, not a distance one. The solver corrects the
    // whole excess inside ONE substep, so it comes back out as a velocity of
    // `excess * simulation_hz * substeps` — measured 2026-08-19 on a 9.00 m rope: 0.0001 m of
    // excess leaves at 0.143 m/s, 0.0100 m at 14.403 m/s, 0.0500 m at 72.004 m/s. There is no
    // metre budget that is "small enough to be harmless"; one centimetre is already twice
    // running speed. The first version of this key was 0.25, derived from
    // `player.max_substep_m` — which bounds a position step and says nothing about a solver
    // impulse — and the measurement is what took it apart.
    let per_substep_hz = d.game.simulation_hz as f32 * d.game.substeps as f32;
    let kick_m_s = slack_m * per_substep_hz;
    println!("b003 slack {slack_m} m -> a kick of {kick_m_s:.2} m/s at {per_substep_hz} /s");
    assert!(
        kick_m_s <= d.game.player.run_speed_m_s,
        "a warp that keeps a rope {slack_m} m too long kicks the player at {kick_m_s:.2} m/s, \
         faster than he runs ({} m/s). This key is a float tolerance, not a distance a player \
         may be moved",
        d.game.player.run_speed_m_s
    );
}


// ---------------------------------------------------------------------------------------
// `FIND-149` — the DRIVE, in the whole app. `game.ron: vector.rope_force_model`.
//
// The user, playing *Attack on Titan Revolution* beside this game on 2026-08-23: *„wenn ich
// mich hooke: dann werde ich direkt rangezogen wenn ich ran gehe. mit a und d kann man zur
// seite gehen. aber sonst wird man direkt hingezogen! **wenn ich nichts drucke dann wird auch
// nicht rangezogen!**"*
//
// The pure-function half is `tests/player.rs::f149_*`; the in-game evidence is
// `scripts/f006-drive.txt`. What is measured **here** is the thing neither of those can see:
// that the joint really is not built, and that gravity is the only thing left acting on a
// hooked player who holds nothing.
// ---------------------------------------------------------------------------------------

fn select(app: &mut App, model: RopeForceModel) {
    app.world_mut().resource_mut::<GameData>().game.vector.rope_force_model = model;
}

fn hold_key(app: &mut App, key: KeyCode) {
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
}

/// Hangs a player on a 20 m rope straight above him, presses **nothing** for `ticks_n`, and
/// returns how far he fell and how far he drifted sideways.
///
/// `dry` re-empties the tank every tick, and it is **not** a side note: with gas in the tank the
/// drive never even runs for a key-less player, because `vector::gas::steer_has_effect` refuses
/// the grant first. An empty tank is the one state in which `air_control` enters the drive
/// branch with nothing held (`docs/NEXT.md` §1e, *„aber hälfte ca"*) — so it is the only case
/// that can tell "the drive returns zero" apart from "the gas ledger happened to say no".
/// Measured 2026-08-23: with the early return in `rope_drive` deleted, the wet run stayed green
/// and this one went red.
fn hangs_and_holds_nothing(model: RopeForceModel, ticks_n: u64, dry: bool) -> (f32, f32) {
    let mut app = app();
    select(&mut app, model);
    let e = me(&mut app);
    hang(&mut app, e, Vec3::new(0.0, 60.0, 0.0), 20.0);
    let before = position(&app, e);
    for _ in 0..ticks_n {
        if dry {
            app.world_mut().get_mut::<Gas>(e).expect("a player carries a tank").current = 0.0;
        }
        ticks(&mut app, 1);
    }
    let after = position(&app, e);
    (before.y - after.y, (after.xz() - before.xz()).length())
}

#[test]
fn f149_under_drive_a_hooked_player_who_presses_nothing_is_not_held_up_by_his_rope() {
    // **The load-bearing sentence, measured.** Same app, same hook, same 20 m rope, same zero
    // keys — the only difference is one line in `game.ron`.
    //
    // ⚠️ Gravity is ON here, deliberately and against this file's usual doctrine: the question
    // is precisely *what happens when nothing but gravity is left*. The control that makes the
    // number mean something is the other model in the same test — if the two ever answer the
    // same, this is measuring the harness and not the rope.
    let ticks_n = 30; // 0.5 s
    let (drive_fall, drive_drift) = hangs_and_holds_nothing(RopeForceModel::Drive, ticks_n, false);
    let (pendulum_fall, pendulum_drift) =
        hangs_and_holds_nothing(RopeForceModel::Pendulum, ticks_n, false);
    // The control that makes the first number worth anything — see [`hangs_and_holds_nothing`].
    let (dry_fall, dry_drift) = hangs_and_holds_nothing(RopeForceModel::Drive, ticks_n, true);

    // Free fall over half a second at the file's −20 m/s² is 2.5 m. The drive's rope holds
    // nothing at all, so that is what has to come out.
    assert!(
        drive_fall > 2.0,
        "under `Drive` a hooked player with no key held fell {drive_fall:.3} m in 0.5 s — free \
         fall owes ~2.5 m, so something is still carrying him and the rope is not a direction"
    );
    // And the pendulum hangs, which is the whole of what the user says the reference does NOT do.
    assert!(
        pendulum_fall < 0.05,
        "under `Pendulum` the same player fell {pendulum_fall:.3} m — the joint is supposed to \
         hold him, and every number measured before 2026-08-23 assumes it does"
    );
    assert!(
        drive_fall - pendulum_fall > 1.0,
        "the two force models let the same player fall {drive_fall:.3} m and {pendulum_fall:.3} \
         m — `vector.rope_force_model` is then a switch between one model and itself"
    );
    // Neither of them is *pulled* sideways. Under `Drive` because nothing acts at all; under
    // `Pendulum` because the rope is vertical and taut. The assert is here so that a future
    // "drive" that quietly hauls a key-less player at his anchor cannot pass this test.
    assert!(
        drive_drift < 0.05 && pendulum_drift < 0.05 && dry_drift < 0.05,
        "a player who pressed nothing drifted {drive_drift:.3} m (Drive) / {pendulum_drift:.3} m \
         (Pendulum) / {dry_drift:.3} m (Drive, empty tank) toward his anchor"
    );
    // **The one measurement the gas ledger cannot be responsible for.** With an empty tank the
    // drive branch really is entered with no key held, so this number is `rope_drive`'s own
    // answer and nothing else's.
    assert!(
        dry_fall > 2.0,
        "under `Drive` with an EMPTY tank and no key held the player fell {dry_fall:.3} m in \
         0.5 s instead of free-falling ~2.5 m — the drive is chasing a target of 0 m/s and \
         braking him in mid-air, which is the exact opposite of „wenn ich nichts drucke dann \
         wird auch nicht rangezogen\""
    );
}

/// Holds `W` for 0.75 s — three of `drive_ramp_s` — and returns the speed reached along the
/// rope, which points along −Z.
///
/// `hook` is the **control**: with it false nothing is anchored and the very same run measures
/// the free air control alone. A drive test that passes with the rope deleted is measuring
/// `air_accel_m_s2` (`CLAUDE.md` rule 5, the control habit).
fn holds_w_toward_an_anchor(model: RopeForceModel, hook: bool) -> f32 {
    let mut app = app();
    select(&mut app, model);
    let e = me(&mut app);
    kill_gravity(&mut app); // the only thing allowed to bend this is the rope
    // ⚠️ **300 m up, and the number is derived.** The first version of this ran at y = 60 in
    // `maps.ron: current` and the velocity fell from 43.1 m/s to 17.3 m/s at tick 27 — the
    // player had flown into a building. The tallest thing in the world is a tower at 123 m
    // (`scripts/f029-grapple.txt` derives the same figure), so at 300 m the 60 m of rope ahead
    // of him is empty air and the only thing bending the flight is the drive.
    let start = Vec3::new(0.0, 300.0, 0.0);
    if hook {
        let eye = data(&app).game.player.eye_height_m;
        // At hand height and straight along −Z, i.e. exactly where a player with yaw 0 and
        // pitch 0 is looking: the look gate is then 1.0 and the geometry is out of the picture.
        hang_on(&mut app, e, start, start + Vec3::Y * eye + Vec3::NEG_Z * 60.0);
    } else {
        // ⚠️ `warp` and not `place`: `place` writes `Position` alone, and avian syncs that back
        // from the `Transform` the player still carries — the first version of this control
        // left him standing on the ground at 0.00 m/s, which would have made "the control is
        // slower than the drive" true for the wrong reason. A warp writes the `Transform`
        // (`player::apply_warps`) and is the sanctioned path (§12c).
        warp(&mut app, e, start);
        app.update();
    }
    hold_key(&mut app, KeyCode::KeyW);
    ticks(&mut app, 45);
    velocity(&app, e).dot(Vec3::NEG_Z)
}

#[test]
fn f149_under_drive_w_hauls_the_player_along_the_rope_and_without_a_rope_it_does_not() {
    let d = GameData::load(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"));
    let want = d.game.vector.drive_speed_m_s;

    let drive = holds_w_toward_an_anchor(RopeForceModel::Drive, true);
    let control = holds_w_toward_an_anchor(RopeForceModel::Drive, false);
    let pendulum = holds_w_toward_an_anchor(RopeForceModel::Pendulum, true);

    // 0.75 s is 3 x `drive_ramp_s`, i.e. 95 % of the target — plus whatever the free air
    // thrust (`air_accel_m_s2`, aimed the same way) puts on top of it, which the drive treats
    // as a disturbance and only partly cancels. The band is wide on purpose: all three keys
    // are ⚠️ UNTUNED and are meant to move.
    assert!(
        drive > want * 0.8 && drive < want * 1.4,
        "0.75 s of `W` on a drive rope reached {drive:.2} m/s against a `drive_speed_m_s` of \
         {want} — 3 time constants owe ~95 % of it"
    );
    // **The control.** Same keys, same app, no rope.
    assert!(
        control < drive * 0.5,
        "with NO rope at all the same 0.75 s of `W` reached {control:.2} m/s against the \
         {drive:.2} m/s with one — this test would be measuring `air_accel_m_s2`"
    );
    // And the pendulum is a different number in the same run: it BUILDS speed at
    // `air_pull_m_s2` instead of chasing one, so at this instant it is well short of the drive.
    assert!(
        (drive - pendulum).abs() > 5.0,
        "the drive reached {drive:.2} m/s and the pendulum {pendulum:.2} m/s — the two models \
         are supposed to be different things"
    );
}

#[test]
fn f149_the_drive_builds_no_joint_and_still_publishes_a_length() {
    // The mechanism, not the feel. Under `Drive` **there is no `DistanceJoint` in the world at
    // all** — that is what "the rope applies no force of its own" means here, and it is why
    // `combat::hitstop` (which takes `JointDisabled` off every joint of a body when a freeze
    // lifts) cannot bring the constraint back to life in the middle of a flight.
    let mut driven = app();
    select(&mut driven, RopeForceModel::Drive);
    let e = me(&mut driven);
    let published = hang(&mut driven, e, Vec3::new(0.0, 60.0, 0.0), 20.0);

    assert_eq!(joint_count(&mut driven), 0, "a `Drive` rope built a distance joint");
    assert!(
        hook_state(&driven, e, Side::Left).is_anchored(),
        "the arm is not anchored — then this test measured nothing"
    );
    // `RopeLength` is read by `vector::hook` (`overextended`) and drawn by the HUD. `0.0` means
    // "no constraint" in that type, so a jointless rope publishes the length it really has.
    assert!(
        (published - 20.0).abs() < 1.0,
        "a `Drive` rope published {published:.3} m for a 20 m rope — `vector::hook` reads this"
    );

    // The control: the same hook under the other model does build one.
    let mut other = app();
    select(&mut other, RopeForceModel::Pendulum);
    let e = me(&mut other);
    hang(&mut other, e, Vec3::new(0.0, 60.0, 0.0), 20.0);
    assert_eq!(joint_count(&mut other), 1, "a `Pendulum` rope did NOT build a distance joint");
}

/// Holds `W` toward an anchor 60 m along −Z while the player is **already flying sideways** at
/// `cross_m_s`, and returns the angle between his velocity and his own rope, in degrees.
///
/// ⚠️ **Gravity is ON**, against this file's usual doctrine and for the same reason
/// [`hangs_and_holds_nothing`] leaves it on: gravity is one of the two things that bend the line
/// the user asked to be straight, and a measurement of *„ziemlich gerade"* with gravity switched
/// off would be measuring a world he never plays in. The other one is the carried momentum, and
/// that is what `cross_m_s` puts there.
fn angle_to_the_rope_deg(model: RopeForceModel, cross_m_s: f32, ticks_n: u64) -> f32 {
    let mut app = app();
    select(&mut app, model);
    let e = me(&mut app);
    // 300 m up for `holds_w_toward_an_anchor`'s reason: the tallest thing in the world is a
    // 123 m tower, so nothing here can fly into a building and report that as a curve.
    let start = Vec3::new(0.0, 300.0, 0.0);
    let eye = data(&app).game.player.eye_height_m;
    let anchor = start + Vec3::Y * eye + Vec3::NEG_Z * 60.0;
    hang_on(&mut app, e, start, anchor);
    set_velocity(&mut app, e, Vec3::X * cross_m_s);
    hold_key(&mut app, KeyCode::KeyW);
    ticks(&mut app, ticks_n);
    let to_anchor = anchor - (position(&app, e) + Vec3::Y * eye);
    velocity(&app, e).angle_between(to_anchor).to_degrees()
}

#[test]
fn f153_under_drive_w_pulls_the_flight_onto_the_rope_line() {
    // *„wenn ich mich hooke und w drücke … dann soll ich erstmal ziemlich direkt daran gezogen
    // werden. also ziemlich gerade"* (2026-08-23). **The angle between the rope and the velocity
    // IS „gerade"**, and it is the one thing neither the pure function nor the script can see:
    // it is made of the carried momentum, of gravity, and of the ramp, all three at once.
    let ticks_n = 15; // a quarter second — „man macht was und man merkt es auch direkt"
    let drive = angle_to_the_rope_deg(RopeForceModel::Drive, 40.0, ticks_n);
    let pendulum = angle_to_the_rope_deg(RopeForceModel::Pendulum, 40.0, ticks_n);
    // Printed, not only asserted: this is one of the two numbers `FIND-153` answers him with,
    // and a number that only exists inside a failure message cannot be quoted at him.
    // `cargo test --test vector_rope f153 -- --nocapture`.
    println!("f153 angle to the rope after {ticks_n} ticks: Drive {drive:.1}° · Pendulum {pendulum:.1}°");

    assert!(
        drive < 8.0,
        "a quarter second of `W` left the player flying {drive:.1}° off his own rope. He started \
         90° off it (40 m/s of crossing momentum) and gravity pulls another `drive_ramp_s · g` \
         out of the line — „ziemlich gerade\" is the instruction"
    );
    // **The control that makes the number mean something.** A pendulum KEEPS the crossing
    // momentum — that is what a swing is — so if these two ever answer the same, this test is
    // measuring the harness and not the force model.
    assert!(
        pendulum > 2.0 * drive,
        "the drive came out {drive:.1}° off the rope and the pendulum {pendulum:.1}° — those are \
         supposed to be different models"
    );
}
