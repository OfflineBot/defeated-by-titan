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
    // 🔴 **No `.after(aim)`.** `aim` moved to `SimulationSystems::PostStep` (`FIND-217`,
    // `B-029`); the six stages are `.chain()`ed, so ordering a `World` system after a
    // `PostStep` one closes a dependency cycle in `FixedUpdate` — and bevy answers a cycle by
    // formatting **every simple cycle in the component** into one `String`, which on
    // 2026-09-01 was a 4.63 GB allocation and an OOM kill (`B-030`, `FIND-218`).
    // `World` is the stage before `Intent`, so it already is the last writer before the hooks
    // read. The edge bought nothing and cost the machine.
    app.add_systems(FixedUpdate, force_aim.in_set(SimulationSystems::World));
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
/// `dry` re-empties the tank every tick, and it is **not** a side note: `rope_drive` itself is
/// refused for a key-less player by `vector::gas::steer_has_effect` whatever the tank says, so
/// the two runs together are what tell the always-on pull (`FIND-172`, free, no grant) apart
/// from anything that reaches the player through the gas ledger. Measured 2026-08-23 the other
/// way round: with the early return in `rope_drive` deleted, the wet run stayed green and this
/// one went red.
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
fn f172_under_drive_a_hooked_player_who_presses_nothing_is_pulled_in_anyway() {
    // **The load-bearing sentence, and it is a NEW one.** The user, 2026-08-26:
    //
    // > *„ich will dass es immer ranzieht. nicht nur wenn ich w drücke!"*
    //
    // ⚠️ This test replaces `f149_under_drive_a_hooked_player_who_presses_nothing_is_not_held_
    // up_by_his_rope`, which asserted the exact opposite and was right to: `FIND-149` recorded
    // *„wenn ich nichts drucke dann wird auch nicht rangezogen!"* off the reference. **That
    // observation still stands and was deliberately not followed** — he has asked for the
    // opposite in this game, and his instruction beats his own earlier report (`CLAUDE.md`).
    // Whoever "restores" the old assert is undoing an instruction, not fixing a regression.
    //
    // The rope is 20 m **straight above**, so the pull and gravity are on the same axis and the
    // sign of one number is the whole verdict.
    let ticks_n = 30; // 0.5 s
    let (drive_fall, drive_drift) = hangs_and_holds_nothing(RopeForceModel::Drive, ticks_n, false);
    let (pendulum_fall, pendulum_drift) =
        hangs_and_holds_nothing(RopeForceModel::Pendulum, ticks_n, false);
    // The always-on pull is **free** — nothing bills it, because no key is down. So the empty
    // tank has to give the same answer, and that is also the control `FIND-152` needed: with a
    // full tank `vector::gas::steer_has_effect` refuses the grant for a key-less player, so a
    // number that only appeared with gas in the tank would be the ledger's and not the rope's.
    let (dry_fall, dry_drift) = hangs_and_holds_nothing(RopeForceModel::Drive, ticks_n, true);
    println!(
        "f172 half a second hanging on a 20 m vertical rope, nothing held: Drive {:.3} m · \
         Drive on an empty tank {:.3} m · Pendulum {:.3} m (negative = climbed)",
        drive_fall, dry_fall, pendulum_fall
    );

    // Free fall over half a second at the file's −20 m/s² is 2.5 m. He must not do that any
    // more, and the assert is on the SIGN: the rope wins against gravity.
    assert!(
        drive_fall < 0.0,
        "under `Drive` a hooked player with no key held moved {drive_fall:.3} m DOWN in 0.5 s — \
         „ich will dass es immer ranzieht\" is the instruction and free fall owes ~2.5 m"
    );
    // ⚠️ And he must not be YANKED at it either — *„es ist zu aggressiv"* is the other half of
    // the same message. `drive_idle_speed_m_s` is a ceiling on the closing speed, so half a
    // second can never move him further than that.
    let ceiling_m = shipped().game.vector.drive_idle_speed_m_s * 0.5;
    assert!(
        -drive_fall < ceiling_m,
        "he climbed {:.3} m in 0.5 s against a `drive_idle_speed_m_s` of {:.1} m/s, which cannot \
         carry him further than {ceiling_m:.3} m — something is hauling him at the anchor",
        -drive_fall,
        ceiling_m * 2.0
    );
    // The always-on pull costs nothing, so these two are the same run.
    assert!(
        (dry_fall - drive_fall).abs() < 0.05,
        "with a full tank he moved {drive_fall:.3} m and with an empty one {dry_fall:.3} m — the \
         idle pull is not billed by anything, so a difference means it is going through a gas \
         grant it does not need"
    );
    // And the pendulum still hangs, which is what makes the first number the DRIVE's.
    assert!(
        pendulum_fall.abs() < 0.05,
        "under `Pendulum` the same player moved {pendulum_fall:.3} m — the joint is supposed to \
         hold him, and every number measured before 2026-08-23 assumes it does"
    );
    // Neither of them drifts: the rope is vertical, and a pull that appears on X or Z is a bug
    // in the direction, not a pull.
    assert!(
        drive_drift < 0.05 && pendulum_drift < 0.05 && dry_drift < 0.05,
        "a player who pressed nothing drifted {drive_drift:.3} m (Drive) / {pendulum_drift:.3} m \
         (Pendulum) / {dry_drift:.3} m (Drive, empty tank) sideways off a vertical rope"
    );
}

/// The tuning as the game ships it — the asserts above are about `game.ron`'s own numbers and
/// must not carry a second copy of them.
fn shipped() -> GameData {
    GameData::load(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"))
}

/// Hangs a player on a 60 m rope straight along −Z at hand height, holds `keys` for half a
/// second and returns the **angle between his velocity and his rope**, in degrees.
///
/// That angle *is* the user's *„rangezogen gegen zur Seite"*: 0° is straight up the rope, 90° is
/// a flight that is not converging on the anchor at all. Gravity is **on** — the question is
/// what a player feels in the world he plays in, and half of "the pull is too strong" is how it
/// compares to the fall.
fn angle_with_keys(keys: &[KeyCode], idle_m_s: f32) -> f32 {
    let mut app = app();
    select(&mut app, RopeForceModel::Drive);
    app.world_mut().resource_mut::<GameData>().game.vector.drive_idle_speed_m_s = idle_m_s;
    let e = me(&mut app);
    // 300 m up for `holds_w_toward_an_anchor`'s reason: the tallest thing in the world is a
    // 123 m tower, so nothing here can fly into a building and report that as a curve.
    let start = Vec3::new(0.0, 300.0, 0.0);
    let eye = data(&app).game.player.eye_height_m;
    let anchor = start + Vec3::Y * eye + Vec3::NEG_Z * 60.0;
    hang_on(&mut app, e, start, anchor);
    for k in keys {
        hold_key(&mut app, *k);
    }
    ticks(&mut app, 30);
    let to_anchor = anchor - (position(&app, e) + Vec3::Y * eye);
    velocity(&app, e).angle_between(to_anchor).to_degrees()
}

#[test]
fn f172_the_three_angles_between_the_rope_and_the_flight() {
    // **The three numbers the user is answered with.** Half a second on the same rope, from
    // rest, with gravity on:
    //
    //   nothing held — *„ich will dass es immer ranzieht"*
    //   `W`          — *„ziemlich gerade"* (`FIND-153`, unchanged)
    //   `D`          — *„stärker zur seite als rangezogen"*
    let d = shipped();
    let idle = d.game.vector.drive_idle_speed_m_s;
    let nothing = angle_with_keys(&[], idle);
    let forward = angle_with_keys(&[KeyCode::KeyW], idle);
    let sideways = angle_with_keys(&[KeyCode::KeyD], idle);
    println!(
        "f172 angle between the rope and the velocity after 0.5 s: nothing {nothing:.1}° · \
         W {forward:.1}° · D {sideways:.1}°"
    );

    // **The control, and it is the habit `CLAUDE.md` rule 5 asks for:** delete the thing being
    // measured — the idle pull — and the same run has to answer differently. Without it the
    // player only falls, and a fall on a horizontal rope is 90° off it.
    let unpulled = angle_with_keys(&[], 0.0);
    assert!(
        unpulled > 80.0,
        "with `drive_idle_speed_m_s` at 0 the key-less player still flew {unpulled:.1}° off his \
         rope instead of the ~90° a pure fall owes — this test is measuring gravity, not the pull"
    );
    assert!(
        nothing < unpulled - 20.0,
        "holding nothing came out {nothing:.1}° off the rope against {unpulled:.1}° with the \
         idle pull switched off — „es zieht immer ran\" has to be visible in this number"
    );
    // `W` is the straight line, and it is the one thing `FIND-153` bought that must survive.
    assert!(
        forward < 15.0,
        "`W` left the player flying {forward:.1}° off his own rope — „ziemlich gerade\" \
         (`FIND-153`)"
    );
    // And `D` beats the pull: more than 45° off the rope is *by definition* more sideways than
    // inward.
    assert!(
        sideways > 45.0,
        "`D` left the player {sideways:.1}° off the rope — under 45° the rope is still winning, \
         and „staerker zur seite als rangezogen\" is exactly the 45° line"
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
fn q058_the_drive_builds_a_real_joint_born_at_the_bite_distance() {
    // 🔴 **This test says the opposite of what it said until 2026-08-27, and that is the
    // feature.** It was `f149_the_drive_builds_no_joint_and_still_publishes_a_length`, and it
    // asserted `joint_count == 0` under `Drive` — *"the rope applies no force of its own"*
    // (`FIND-149`). `docs/NEXT.md` §3D was then attempted twice on top of that model and
    // refuted 4/4 (`FIND-186`), because *"the anchor distance must not increase"* cannot be
    // proved by hand in a velocity target for two arms.
    //
    // The user settled it (`docs/NEXT.md` §3F, `Q-058`): *„aber NICHT das seil verlängern!!"*
    // **is** a hard maximum length. Whoever "restores" the old assert is undoing an
    // instruction, not fixing a regression.
    //
    // 🔴 **`limits.max` is born at the BITE DISTANCE, and that is the half that keeps it a rope
    // instead of a leash.** `(0, L)` corrects only when the distance *exceeds* `L`, so a rope
    // born at the distance that already exists cannot yank and cannot hold the player up — see
    // `f172_*` below, which measures exactly that under both models.
    let nominal_m = 20.0;
    let mut driven = app();
    select(&mut driven, RopeForceModel::Drive);
    let e = me(&mut driven);
    let published = hang(&mut driven, e, Vec3::new(0.0, 60.0, 0.0), nominal_m);

    assert_eq!(joint_count(&mut driven), 1, "a `Drive` rope built no distance joint (`Q-058`)");
    assert!(
        hook_state(&driven, e, Side::Left).is_anchored(),
        "the arm is not anchored — then this test measured nothing"
    );
    // The published length is `limits.max` now, not the reach — `sync_rope_length`'s `Some`
    // arm. The harness holds the player on his spot until the hook bites, so the bite distance
    // IS the nominal one.
    let here = position(&driven, e);
    let anchor = anchor_point(&mut driven).expect("the joint has an anchor marker");
    assert!(
        (published - (anchor - here).length()).abs() < 0.05,
        "the rope was born at {published:.3} m and the player really stands {:.3} m from his \
         anchor — a length that is not the bite distance is a yank on the first tick",
        (anchor - here).length()
    );
    assert!(
        (published - nominal_m).abs() < 1.0,
        "a `Drive` rope published {published:.3} m for a {nominal_m} m rope — `vector::hook` \
         reads this"
    );

    // The control: the other model builds exactly the same thing. Since `Q-058` the two differ
    // **only** in what `player::locomotion` does with the body, and if this ever disagrees then
    // `attach_ropes` has grown a fork it is not supposed to have.
    let mut other = app();
    select(&mut other, RopeForceModel::Pendulum);
    let e = me(&mut other);
    let pendulum = hang(&mut other, e, Vec3::new(0.0, 60.0, 0.0), nominal_m);
    assert_eq!(joint_count(&mut other), 1, "a `Pendulum` rope did NOT build a distance joint");
    assert!(
        (published - pendulum).abs() < 0.05,
        "the same hook was born {published:.3} m long under `Drive` and {pendulum:.3} m under \
         `Pendulum` — the rope is one thing now and only the locomotion forks"
    );
}

#[test]
fn q058_under_drive_ctrl_shortens_the_joint_exactly_as_under_pendulum() {
    // **Where `Ctrl` went.** `player::locomotion::rope_winch` lost its `grant.reel_in` branch
    // on 2026-08-27 because `player::rope::shorten_ropes` can finally see a `Drive` rope, and
    // the claim that replaces it is this one: the reel is a **length**, and it is the same
    // length under both models.
    //
    // Red by putting `commands.spawn(rope);` back into the `Drive` arm of `attach_ropes` — with
    // no joint there is nothing to shorten and the `Drive` row stops moving at all.
    let reeled = |model| {
        let mut app = app();
        select(&mut app, model);
        let e = me(&mut app);
        let born = hang(&mut app, e, Vec3::new(0.0, 60.0, 0.0), 20.0);
        hold_reel_in(&mut app);
        ticks(&mut app, 30); // half a second
        (born, rope_length(&app, e, Side::Left), position(&app, e).y)
    };
    let (born_d, after_d, y_d) = reeled(RopeForceModel::Drive);
    let (born_p, after_p, y_p) = reeled(RopeForceModel::Pendulum);
    println!(
        "q058 half a second of Ctrl on a 20 m vertical rope: Drive {born_d:.3} → {after_d:.3} m \
         (y {y_d:.3}) · Pendulum {born_p:.3} → {after_p:.3} m (y {y_p:.3})"
    );

    // It really shortens, and by the file's own rate — `reel_speed_m_s` over half a second,
    // with the take-up free to take more. Anything less than a fifth of that is a dead key.
    let d = data(&app()); // the shipped tuning; the asserts must not carry a second copy
    let due_m = d.game.vector.reel_speed_m_s * 0.5;
    assert!(
        born_d - after_d > due_m * 0.2,
        "`Ctrl` took {:.3} m off a `Drive` rope in half a second, and `reel_speed_m_s` alone is \
         worth {due_m:.1} m — the key is dead again",
        born_d - after_d
    );
    // 🔴 **The one that makes it the JOINT's reel and not something else**: the same key, the
    // same half second, the same length under the model that has always had a joint.
    assert!(
        (after_d - after_p).abs() < 0.05,
        "half a second of `Ctrl` left a `Drive` rope at {after_d:.3} m and a `Pendulum` rope at \
         {after_p:.3} m — `shorten_ropes` is one system and it must not see two models"
    );
    // And the rope really carried him: the reel is a length, the lift is the constraint's.
    assert!(
        y_d > 60.0 + 1.0,
        "he was reeled up a vertical rope for half a second and ended at y {y_d:.3} from 60.0 — \
         the length moved and the body did not, which is a rope that is not connected to him"
    );
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

    // ⚠️ **The band was 8° until `FIND-172` and it is 12° now — measured 2.2° at
    // `(70, 0.08, ∞)` and 8.0° at `(52, 0.08, 250)`, and that is the WEIGHT, not a slipped
    // number.** Straightening a flight that starts 40 m/s across its own rope is a 65 m/s change
    // of velocity, and `drive_accel_max_m_s2` is precisely the rule that a big change of
    // velocity costs time (*„es fühlt sich zu leicht an"*, 2026-08-26). A quarter second is the
    // hardest moment there is for this number; by half a second the same flight is back inside
    // 2°. 🔴 **If he ever says the drive bends instead of pulling, `drive_accel_max_m_s2` is
    // the key — and raising it gives the weight back.**
    assert!(
        drive < 12.0,
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

// ---------------------------------------------------------------------------------------
// `docs/NEXT.md` §3F · `Q-058` — THE HARD MAXIMUM LENGTH, FOR **TWO** ARMS AT ONCE
//
// The user, 2026-08-27:
//
//   *„wenn man a oder d drückt (relativ zum anker (DAS IST WICHTIG), immer alles relativ zum
//    anker gesehen) dann soll man zur seite gehen können. **aber NICHT das seil verlängern!!**"*
//
// Two attempts proved that invariant **by hand, in a velocity target**, and both were refuted
// 4/4 (`docs/FINDINGS.md` FIND-186): attempt 1 bounded the **sum** of the two distances and
// flew 40.00 → 69.33 m away from an arm; attempt 2 scaled the whole command to `Vec3::ZERO` on
// 25.8 % of ticks. **Neither failure is expressible against a joint**, which is why the answer
// to `Q-058` is a real `DistanceJoint` with `limits = (0, L)` on a `Drive` rope too.
// ---------------------------------------------------------------------------------------

/// One arm as the acceptance criterion sees it: where its anchor is, and what its rope allows.
#[derive(Clone, Copy, Debug)]
struct Arm {
    side: Side,
    anchor_m: Vec3,
    /// `limits.max` — the maximum the joint enforces. `None` for a rope with no joint at all,
    /// which is what a `Drive` rope was until `Q-058`.
    enforced_m: Option<f32>,
}

/// Every rope in the world, read out of the components rather than remembered by the test.
fn arms(app: &mut App) -> Vec<Arm> {
    let mut q = app
        .world_mut()
        .query::<(&defeated_by_titan::player::rope::Rope, Option<&DistanceJoint>)>();
    let found: Vec<(Side, Entity, Option<f32>)> = q
        .iter(app.world())
        .map(|(rope, joint)| (rope.side, rope.anchor, joint.map(|j| j.limits.max)))
        .collect();
    found
        .into_iter()
        .filter_map(|(side, anchor, enforced_m)| {
            app.world()
                .get::<Position>(anchor)
                .map(|p| Arm { side, anchor_m: p.0, enforced_m })
        })
        .collect()
}

/// Fires one arm at `anchor_m` and holds the player on `hold_m` until it bites. Returns the
/// length the rope was born with.
///
/// The anchor gets a [`BodyId`] of its own, so the two arms really hang on two different
/// bodies — [`hang_right`] deliberately re-uses the first one's, which is the single-anchor
/// case and not the one FIND-186 got wrong.
fn bite_on(
    app: &mut App,
    e: Entity,
    side: Side,
    anchor_m: Vec3,
    body: BodyId,
    hold_m: Vec3,
) -> f32 {
    app.world_mut().resource_mut::<SpatialIndex>().insert(IndexEntry {
        id: body,
        center_m: anchor_m,
        half_size_m: Vec3::splat(1.0),
        mask: BodyMask::SOLID.with(BodyMask::ANCHORABLE),
    });
    app.world_mut().entity_mut(e).insert(ForcedAim(AimPoint {
        point_m: Some(anchor_m),
        body: Some(body),
        anchorable: true,
    }));
    let key = match side {
        Side::Left => KeyCode::KeyQ,
        Side::Right => KeyCode::KeyE,
    };
    place(app, e, hold_m);
    set_velocity(app, e, Vec3::ZERO);
    app.update();
    app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
    for _ in 0..600 {
        place(app, e, hold_m);
        set_velocity(app, e, Vec3::ZERO);
        app.update();
        if hook_state(app, e, side).is_anchored() {
            place(app, e, hold_m);
            set_velocity(app, e, Vec3::ZERO);
            app.update();
            let l = rope_length(app, e, side);
            assert!(l > 0.0, "the {side:?} hook bit and no rope came into being");
            return l;
        }
    }
    panic!("the {side:?} hook did not bite in 600 ticks — it is {:?}", hook_state(app, e, side));
}

/// Hangs **both** arms on **two different** anchors. Returns the birth length per side.
fn hang_two(app: &mut App, e: Entity, hold_m: Vec3, left_m: Vec3, right_m: Vec3) -> (f32, f32) {
    let l = bite_on(app, e, Side::Left, left_m, BodyId(80_101), hold_m);
    let r = bite_on(app, e, Side::Right, right_m, BodyId(80_102), hold_m);
    (l, r)
}

/// The nine key combinations §3D's acceptance sentence names, by their `KeyCode`s.
const COMBOS: [(&str, &[KeyCode]); 9] = [
    ("none", &[]),
    ("W", &[KeyCode::KeyW]),
    ("A", &[KeyCode::KeyA]),
    ("D", &[KeyCode::KeyD]),
    ("S", &[KeyCode::KeyS]),
    ("A+W", &[KeyCode::KeyA, KeyCode::KeyW]),
    ("D+W", &[KeyCode::KeyD, KeyCode::KeyW]),
    ("A+S", &[KeyCode::KeyA, KeyCode::KeyS]),
    ("D+S", &[KeyCode::KeyD, KeyCode::KeyS]),
];

const SEPARATIONS_DEG: [f32; 4] = [60.0, 90.0, 120.0, 170.0];
/// Open air, far above anything: `MovementState::Tethered`, `in_flight` true, no ground under
/// the feet to write the horizontal velocity.
const AIR_SPOT_M: Vec3 = Vec3::new(0.0, 300.0, 0.0);
/// `scripts/f176-pull.txt`'s spot, and for its measured reason: the market hall covers
/// x −47..−15, z −13..13 and this does not, so a walk from here is a walk.
const GROUND_SPOT_M: Vec3 = Vec3::new(45.0, 0.6, -43.0);
const YAWS_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];
const ELEVATIONS_DEG: [f32; 2] = [20.0, 70.0];

/// One cell of the matrix: how far past its own `limits.max` an arm got, in metres.
#[derive(Clone, Copy, Debug)]
struct Cell {
    separation_deg: f32,
    elevation_deg: f32,
    yaw_deg: f32,
    #[allow(dead_code)] stance: &'static str,
    combo: &'static str,
    /// The worst `distance − enforced` over every tick and both arms. Negative = slack.
    worst_excess_m: f32,
    /// 🔴 **The same number over the ticks where the two maxima are SATISFIABLE AT ALL.**
    ///
    /// Two joints with maxima `Lₗ` and `Lᵣ` on anchors `dₐ` apart have a solution iff
    /// `Lₗ + Lᵣ ≥ dₐ` — the triangle inequality, and it is the whole of it. Below that no
    /// position in space honours both and the solver has to abandon one arm; that is
    /// `FIND-191`, it is `player::rope::shorten_ropes`' defect and it predates the joint on
    /// `Drive` by the whole life of `Pendulum`.
    ///
    /// **This field is the acceptance criterion `Q-058` can actually be held to**, and it is
    /// not a loosened assert: it is `worst_excess_m` restricted to the ticks where the promise
    /// is a promise. [`Cell::ticks_infeasible`] is what stops that restriction from being a
    /// silent exclusion — it is printed in the matrix and asserted on separately.
    worst_feasible_excess_m: f32,
    /// How many of the cell's ticks were excluded from [`Cell::worst_feasible_excess_m`] because
    /// `Lₗ + Lᵣ < dₐ`. **A `continue` in a sweep is an exclusion invisible in the denominator**
    /// (`CLAUDE.md` §6 rule 5), so it is counted and reported instead of skipped.
    ticks_infeasible: u32,
    /// How many ticks had fewer than two ropes left. A rope that is *gone* cannot be measured,
    /// and a cell that lost one is not a two-anchor cell any more.
    ticks_with_one_rope: u32,
}

/// The whole sweep, one stance, as a `Vec<Cell>` — so the caller can print a matrix and assert
/// on it separately.
///
/// # 🔴 What this reads, and what it varies. The difference is the bug.
///
/// **Read by the code under test** (`player::locomotion::air_control` → `rope_drive` +
/// `rope_winch` + `air_thrust`, `player::rope::attach_ropes` + `shorten_ropes`, and the avian
/// solver): `Intent::move_x`, `Intent::move_y`, `Intent::yaw`, `Intent::look_dir()` (yaw AND
/// pitch), `Buttons::REEL_IN`, `LinearVelocity`, `MovementState`, `Gas` / `GasGrant::steer` /
/// `GasGrant::reel_in`, `Hook::arm(side).tip_m` for both arms, `Transform::translation`,
/// `ReelSpeed`, `DistanceJoint::limits.max` per rope, both anchor `Position`s, and out of
/// `game.ron`: `drive_speed_m_s`, `drive_lateral_m_s`, `drive_ramp_s`, `drive_accel_max_m_s2`,
/// `drive_steer_pull_fraction`, `drive_idle_speed_m_s`, `drive_idle_ramp_s`, `reel_speed_m_s`,
/// `min_rope_m`, `air_accel_m_s2`, `air_accel_empty_fraction`, `gravity_m_s2`, `max_speed_m_s`,
/// `eye_height_m`, `run_speed_m_s`.
///
/// **Varied by this sweep:** the nine key combos, `yaw` (4 values, a full turn — this is the
/// coordinate `FIND-183`'s elevator lived in, because `ê_right` is built out of it and out of
/// nothing else), the two anchors' **angular separation** (4), their **elevation** above the
/// hand (2), and — by the caller, one matrix each — the **stance** (open air at 300 m,
/// `MovementState::Tethered`, `in_flight` true; against standing on clear ground, `Grounded`,
/// the winch's other branch and `ground_locomotion`'s), the **force model**, and whether
/// **`Ctrl`** is held.
///
/// 🔴 **`ctrl` and `model` are the caller's, and there are FOUR matrices, not one.** The two
/// `Ctrl`-free ones assert an absolute bound (`f3f_two_anchors_…`, `f3f_the_same_two_anchors_…`);
/// the two with `Ctrl` held assert a **difference against `Pendulum`**, because with the reel in
/// the composition the absolute bound is not reachable by any solver — `FIND-191`. That is not a
/// loosened assert: it is a different claim, and it is the claim `Q-058` actually made
/// (*"`Drive` INHERITS the joint `Pendulum` has always had"*).
///
/// **NOT varied, and each one is a hole somebody may fall into later:** `pitch` (fixed at 0,
/// so a player looking straight down at his own anchor is never sampled); the gas (always
/// full, so `air_accel_empty_fraction`'s half-strength drive is never sampled); the rope
/// **length** (30 m, one value — `min_rope_m` = 3 is never approached, so `FIND-035`'s cliff
/// is outside this matrix); the two ropes' lengths relative to each other (always equal); the
/// player's velocity at the start (always zero); `game.ron` itself (shipped values only); and
/// the number of arms (always exactly two — the one-arm case is `f172`/`f153`'s).
fn two_anchor_matrix(
    model: RopeForceModel,
    stance: &'static str,
    stand_m: Vec3,
    ticks_n: u64,
    ctrl: bool,
) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut skipped = 0u32;
    for &separation_deg in &SEPARATIONS_DEG {
        for &elevation_deg in &ELEVATIONS_DEG {
            for &yaw_deg in &YAWS_DEG {
                for (combo, keys) in COMBOS {
                    let mut app = app();
                    select(&mut app, model);
                    let e = me(&mut app);
                    let eye = data(&app).game.player.eye_height_m;
                    let hand = stand_m + Vec3::Y * eye;
                    // The two anchors, 30 m out, `separation_deg` apart **around the vertical**
                    // and `elevation_deg` above the horizon. Two roofs, which is the shape the
                    // player really hooks — and the shape both refuted attempts got wrong.
                    let l_m = 30.0f32;
                    let half = (separation_deg * 0.5).to_radians();
                    let el = elevation_deg.to_radians();
                    let dir = |a: f32| {
                        Vec3::new(a.sin() * el.cos(), el.sin(), -a.cos() * el.cos()).normalize()
                    };
                    let left_m = hand + dir(-half) * l_m;
                    let right_m = hand + dir(half) * l_m;

                    let (birth_l, birth_r) = hang_two(&mut app, e, stand_m, left_m, right_m);
                    let birth = |side: Side| match side {
                        Side::Left => birth_l,
                        Side::Right => birth_r,
                    };
                    for k in keys {
                        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(*k);
                    }
                    if ctrl {
                        app.world_mut()
                            .resource_mut::<ButtonInput<KeyCode>>()
                            .press(KeyCode::ControlLeft);
                    }

                    let mut worst_excess_m = f32::NEG_INFINITY;
                    let mut worst_feasible_excess_m = f32::NEG_INFINITY;
                    let mut ticks_infeasible = 0;
                    let mut ticks_with_one_rope = 0;
                    // The anchors do not move, so their separation is a constant of the cell —
                    // it is the `dₐ` of [`Cell::worst_feasible_excess_m`]'s triangle inequality.
                    let anchors_apart_m = (right_m - left_m).length();
                    for _ in 0..ticks_n {
                        // The look is an absolute angle and it is consumed every tick
                        // (`net::local::read_input` `take()`s it), so it is written every tick.
                        app.world_mut()
                            .resource_mut::<defeated_by_titan::shared::LookOverride>()
                            .0 = Some((yaw_deg.to_radians(), 0.0));
                        app.update();
                        let here = position(&app, e);
                        let found = arms(&mut app);
                        if found.len() < 2 {
                            ticks_with_one_rope += 1;
                        }
                        // `Lₗ + Lᵣ ≥ dₐ` — whether a position satisfying BOTH maxima exists at
                        // all this tick. A rope with no joint has no maximum, so it cannot make
                        // the pair impossible: `f32::INFINITY` is the honest reading of "no
                        // constraint" and it is also what the pre-`Q-058` build measured.
                        let budget_m: f32 = found
                            .iter()
                            .map(|a| a.enforced_m.unwrap_or(f32::INFINITY))
                            .sum();
                        let feasible = found.len() == 2 && budget_m >= anchors_apart_m;
                        if !feasible {
                            ticks_infeasible += 1;
                        }
                        for arm in found {
                            // **The joint constrains the body ORIGIN**, not the hand: both
                            // local anchors are `Vec3::ZERO` (`player::rope::attach_ropes`), so
                            // this is the very distance `DistanceLimit` measures.
                            let distance_m = (arm.anchor_m - here).length();
                            // No joint = no enforced maximum, so the rope is held to the length
                            // it was BORN with. That is the LENIENT reading: the ratchet only
                            // ever takes length away, so a build that passes this would also
                            // have to pass against `limits.max`.
                            let max_m = arm.enforced_m.unwrap_or_else(|| birth(arm.side));
                            worst_excess_m = worst_excess_m.max(distance_m - max_m);
                            if feasible {
                                worst_feasible_excess_m =
                                    worst_feasible_excess_m.max(distance_m - max_m);
                            }
                        }
                    }
                    if worst_excess_m == f32::NEG_INFINITY {
                        // Counted, never silent: a cell that measured nothing is not a pass.
                        skipped += 1;
                        continue;
                    }
                    cells.push(Cell {
                        separation_deg,
                        elevation_deg,
                        yaw_deg,
                        stance,
                        combo,
                        worst_excess_m,
                        // `NEG_INFINITY` here means *every* tick of the cell was infeasible.
                        // It is not a pass and it is not a `continue`: it is carried into the
                        // matrix as `-inf` and `ticks_infeasible == ticks_n` says why.
                        worst_feasible_excess_m,
                        ticks_infeasible,
                        ticks_with_one_rope,
                    });
                }
            }
        }
    }
    let expected = SEPARATIONS_DEG.len() * ELEVATIONS_DEG.len() * YAWS_DEG.len() * COMBOS.len();
    assert_eq!(
        cells.len() + skipped as usize,
        expected,
        "the sweep lost cells it never counted"
    );
    assert_eq!(
        skipped, 0,
        "{skipped} of {expected} {stance} cells measured NOTHING and would have passed in \
         silence — a `continue` in a sweep is an exclusion invisible in the denominator"
    );
    cells
}

/// Prints the matrix worst-first and returns the worst cell.
fn report(name: &str, cells: &[Cell]) -> Cell {
    let mut sorted: Vec<Cell> = cells.to_vec();
    sorted.sort_by(|a, b| b.worst_excess_m.total_cmp(&a.worst_excess_m));
    let infeasible: u32 = cells.iter().map(|c| c.ticks_infeasible).sum();
    let lost: u32 = cells.iter().map(|c| c.ticks_with_one_rope).sum();
    println!(
        "--- {name}: {} cells, worst 12 by excess · {infeasible} tick(s) where no position \
         satisfies both maxima · {lost} tick(s) with <2 ropes ---",
        cells.len()
    );
    println!("  sep   el   yaw  keys   worst excess     worst FEASIBLE   infeasible ticks");
    for c in sorted.iter().take(12) {
        println!(
            "  {:5.0} {:4.0} {:5.0}  {:5}  {:+13.4}  {:+15.4}  {:8}",
            c.separation_deg,
            c.elevation_deg,
            c.yaw_deg,
            c.combo,
            c.worst_excess_m,
            c.worst_feasible_excess_m,
            c.ticks_infeasible
        );
    }
    sorted[0]
}

/// The solver's own error is not a rope getting longer.
///
/// `docs/measurements/rope-decision.md` records 2–5 mm of `DistanceJoint` error under load, and
/// the drive presses against the limit at `drive_accel_max_m_s2` = 250 m/s² for as long as a key
/// is held. **1 cm is the band, and it is not a tuning number:** the failures this test exists
/// to catch are 29 m (`FIND-186` attempt 1: 40.00 → 69.33 m) and 45 m (`A`+`W` on HEAD:
/// 51.55 → 96.75 m), i.e. three orders of magnitude away from it.
const SOLVER_SLOP_M: f32 = 0.01;

/// The band the two force models are allowed to differ by **with `Ctrl` held**, and it is a band
/// on a DIFFERENCE, not on an excess.
///
/// `FIND-191`'s single stance measured `Drive` and `Pendulum` agreeing to three decimals. Over
/// the 288-cell matrix the agreement is looser, and it has to be: with both `limits.max` driven
/// below what the geometry allows the solver is choosing which arm to abandon, and that choice
/// is decided by millimetres of position error which the drive's own acceleration then amplifies
/// over 90 ticks. **This number is measured and stated below, not tuned** — and the failures it
/// exists to catch are the ones `FIND-186` measured: 29 m and 45 m.
const CTRL_MODEL_GAP_M: f32 = 0.05;

#[test]
fn f3f_two_anchors_and_no_arm_ever_gets_further_from_its_own_anchor_than_its_rope_allows() {
    // 🔴 **RED FIRST, and this is what it read before `Q-058` was built: +51.1978 m** at 60°
    // separation, 20° elevation, yaw 90°, `A` held — a `Drive` rope had no joint at all, so
    // nothing in the game held either distance and `A`/`D` walked straight out of both ropes.
    //
    // 🔴 **AND RED AGAIN AFTERWARDS, 2026-08-27 [offlinebot], by putting `commands.spawn(rope);`
    // back into the `Drive` arm of `player::rope::attach_ropes`** — the rollback point `Q-058`
    // names, one branch, and the whole feature. All four matrices went red on the same run:
    //
    // | matrix | worst excess | where |
    // |---|---|---|
    // | air | **+51.1978 m** | 60° / 20° / yaw 90° / `A` |
    // | ground | **+50.0737 m** | 170° / 20° / yaw 270° / `W` |
    // | air + `Ctrl` (feasible ticks only) | **+51.1978 m** | 60° / 20° / yaw 90° / `A` |
    // | ground + `Ctrl` (feasible ticks only) | **+50.0737 m** | 170° / 20° / yaw 270° / `W` |
    //
    // Restored, all four are green at ≤ 0.0050 m / ≤ 0.2750 m. The band between the two states
    // is four orders of magnitude, which is what makes this a guard and not a tolerance.
    //
    // ⚠️ **`Ctrl` is deliberately NOT in THIS matrix, and that is not a loosened assert** — the
    // reel can drive two maxima below what the geometry allows, and then no solver can honour
    // both (`FIND-191`). The ratchet is not left out of the acceptance criterion, it is measured
    // where it can be measured: `f3f_with_the_ratchet_held_no_arm_escapes_a_maximum_a_position_could_satisfy`
    // and `f3f_the_same_with_the_ratchet_held_…` run this same 288-cell sweep with `Ctrl` down
    // under **both** force models. What they assert is stated in [`ctrl_matrix`]: the excess
    // over the ticks where both maxima are SATISFIABLE (bounded by one tick of reel), and the
    // per-cell difference to `Pendulum` in the saturated regime, which is the claim `Q-058`
    // made.
    let air = two_anchor_matrix(RopeForceModel::Drive, "air", AIR_SPOT_M, 90, false);
    let worst = report("two anchors, open air", &air);
    assert!(
        worst.worst_excess_m <= SOLVER_SLOP_M,
        "air: at {:.0}° separation, {:.0}° elevation, yaw {:.0}°, keys `{}`, an arm ended \
         {:.4} m FURTHER from its anchor than its own rope allows. „aber NICHT das seil \
         verlängern!!\" — the joint is what is supposed to make that impossible",
        worst.separation_deg,
        worst.elevation_deg,
        worst.yaw_deg,
        worst.combo,
        worst.worst_excess_m
    );
}

#[test]
fn f3f_the_same_two_anchors_hold_with_the_player_standing_on_the_ground() {
    // The other stance, and it is a different code path in three places: `in_flight` is false,
    // so the winch's always-on branch is off; `ground_locomotion` writes the horizontal
    // velocity; and the ground itself can push the player around. `FIND-182`'s elevator lived
    // exactly here — a walking hooked player at 50.6 m/s and 16.5 m up.
    //
    // The spot is `scripts/f176-pull.txt`'s, and for its measured reason: the market hall
    // covers x −47..−15, z −13..13 and this does not, so a walk from here is a walk.
    let ground = two_anchor_matrix(RopeForceModel::Drive, "ground", GROUND_SPOT_M, 90, false);
    let worst = report("two anchors, standing", &ground);
    assert!(
        worst.worst_excess_m <= SOLVER_SLOP_M,
        "ground: at {:.0}° separation, {:.0}° elevation, yaw {:.0}°, keys `{}`, an arm ended \
         {:.4} m further from its anchor than its own rope allows",
        worst.separation_deg,
        worst.elevation_deg,
        worst.yaw_deg,
        worst.combo,
        worst.worst_excess_m
    );
}

/// **The comparison the two `Ctrl` matrices make, factored out** — same sweep, two force models,
/// per cell. Returns `(worst cell under `Drive`, worst per-cell gap, the cell it was in)`.
///
/// The pairing is positional and it is checked, not trusted: [`two_anchor_matrix`] asserts
/// `skipped == 0`, so both matrices carry every combination in the same loop order and cell `i`
/// on the left is cell `i` on the right. If that ever stops being true the `assert_eq!` below
/// says so instead of comparing two different stances and calling them equal.
fn ctrl_matrix(stance: &'static str, spot_m: Vec3) {
    let drive = two_anchor_matrix(RopeForceModel::Drive, stance, spot_m, 90, true);
    let pendulum = two_anchor_matrix(RopeForceModel::Pendulum, stance, spot_m, 90, true);
    report(&format!("{stance} — Drive"), &drive);
    report(&format!("{stance} — Pendulum"), &pendulum);
    assert_eq!(drive.len(), pendulum.len(), "the two matrices are not the same sweep");

    // ---- 1. the acceptance criterion, over the ticks where it is a criterion ------------
    let mut worst_feasible = drive[0];
    let mut measured_cells = 0;
    let mut infeasible_ticks = 0u32;
    for c in &drive {
        infeasible_ticks += c.ticks_infeasible;
        if c.worst_feasible_excess_m > f32::NEG_INFINITY {
            measured_cells += 1;
            if c.worst_feasible_excess_m > worst_feasible.worst_feasible_excess_m {
                worst_feasible = *c;
            }
        }
    }
    // 🔴 **THE BAND WITH `Ctrl` HELD IS ONE TICK OF REEL, AND IT IS DERIVED, NOT TUNED.**
    // `player::rope::shorten_ropes` takes `reel_speed_m_s / simulation_hz` = 28/60 = **0.4667 m**
    // off `limits.max` every tick, and the sample below is read once a tick, *after* the update.
    // So the maximum can move that far underneath a body the solver has already placed, and
    // `distance − limits.max` is then a **shrinking** maximum and not a lengthening rope — the
    // opposite of what §3F forbids. `SOLVER_SLOP_M` (1 cm) is the band when nothing moves the
    // maximum; this is the band when the ratchet does, and it is the same 1 cm plus the one
    // thing that changed.
    // ⚠️ It is a bound, not a measurement: the run reads 0.2748 m (air) / 0.2750 m (ground),
    // i.e. 59 % of it. If it ever exceeds one tick of reel, something is moving the BODY out
    // and not the maximum in.
    let reel_lag_m =
        shipped().game.vector.reel_speed_m_s / shipped().game.simulation_hz as f32
            + SOLVER_SLOP_M;
    let ticks_total = drive.len() as u32 * 90;
    println!(
        "{stance}: {measured_cells} of {} cells had at least one satisfiable tick ·          {infeasible_ticks} of {ticks_total} ticks ({:.1} %) asked for two maxima no position          satisfies (`FIND-191`) · worst FEASIBLE excess {:+.4} m",
        drive.len(),
        100.0 * f64::from(infeasible_ticks) / f64::from(ticks_total),
        worst_feasible.worst_feasible_excess_m
    );
    assert!(
        measured_cells > 0,
        "{stance}: not one of {} cells had a single tick where both maxima were satisfiable, so          this matrix asserted NOTHING — the reel reaches `min_rope_m` before the sweep starts          measuring and the fixture needs a shorter hold, not a weaker assert",
        drive.len()
    );
    assert!(
        worst_feasible.worst_feasible_excess_m <= reel_lag_m,
        "{stance}: with the ratchet in the composition, an arm ended {:.4} m FURTHER from its \
         anchor than its own `limits.max` allows at {:.0}° separation, {:.0}° elevation, yaw \
         {:.0}°, keys `{}` — **and on a tick where both maxima WERE satisfiable**, so \
         `FIND-191` does not excuse it. One tick of reel is {reel_lag_m:.4} m and that is all \
         a shrinking maximum can explain",
        worst_feasible.worst_feasible_excess_m,
        worst_feasible.separation_deg,
        worst_feasible.elevation_deg,
        worst_feasible.yaw_deg,
        worst_feasible.combo
    );

    // ---- 2. and the 50 m is the REEL's defect, not the drive's -------------------------
    //
    // 🔴 **AND THE CONTROL IS THE SATURATED NUMBER, NOT THE FEASIBLE ONE — measured, after
    // writing the assert the other way round first.** The obvious version of this control
    // compares both numbers and it goes red at **0.1878 m** (120° separation, 20° elevation,
    // yaw 0°, `D` held). That is not a defect: over the ticks where the pair IS satisfiable the
    // two models are supposed to move the player DIFFERENTLY — `D` under `Drive` is
    // `rope_drive`'s 36 m/s lateral velocity target, `D` under `Pendulum` is `rope_steer`'s
    // force, and with no key at all `Drive` still has `rope_winch`'s always-on pull that
    // `Pendulum` has never had (`f172_…`, `f149_the_two_force_models_are_not_the_same_thing`).
    // **A model comparison over the feasible ticks is a test that the two models are the same
    // thing, and they are not.**
    //
    // What `FIND-191` claims is narrower and it is exactly what the saturated number says:
    // once the reel has driven `Lₗ + Lᵣ` below the anchors' separation, **the end state is
    // geometric** — the player sits at `min_rope_m` from one anchor and the other rope is over
    // by the rest, whatever the keys were doing. If that fixpoint were different under the two
    // models, `Q-058` would have changed the reel instead of inheriting it. It is 0.0000 m
    // across all 288 cells in both stances.
    let mut worst_gap = 0.0f32;
    let mut worst_cell = drive[0];
    let mut worst_feasible_gap = 0.0f32;
    for (a, b) in drive.iter().zip(pendulum.iter()) {
        assert_eq!(
            (a.separation_deg, a.elevation_deg, a.yaw_deg, a.combo),
            (b.separation_deg, b.elevation_deg, b.yaw_deg, b.combo),
            "the two matrices drifted out of step — cell {a:?} is being compared with {b:?}"
        );
        let gap = (a.worst_excess_m - b.worst_excess_m).abs();
        if gap > worst_gap {
            worst_gap = gap;
            worst_cell = *a;
        }
        // Printed and NOT asserted on, for the reason above — it is the size of the difference
        // between two force models, and a round that wants it smaller wants a different game.
        worst_feasible_gap = worst_feasible_gap
            .max((a.worst_feasible_excess_m - b.worst_feasible_excess_m).abs());
    }
    println!(
        "{stance}: worst per-cell gap `Drive` vs `Pendulum` — saturated {worst_gap:.4} m (at \
         {:.0}°/{:.0}°/yaw {:.0}° `{}`), over the FEASIBLE ticks {worst_feasible_gap:.4} m \
         (the two force models, doing their two different jobs)",
        worst_cell.separation_deg,
        worst_cell.elevation_deg,
        worst_cell.yaw_deg,
        worst_cell.combo
    );
    assert!(
        worst_gap <= CTRL_MODEL_GAP_M,
        "{stance}: with `Ctrl` held, `Drive` and `Pendulum` disagree by {worst_gap:.4} m at \
         {:.0}° separation, {:.0}° elevation, yaw {:.0}°, keys `{}` — `Q-058` gave `Drive` the \
         joint `Pendulum` already had, so a difference means it changed the reel instead of \
         inheriting it (`FIND-191`)",
        worst_cell.separation_deg,
        worst_cell.elevation_deg,
        worst_cell.yaw_deg,
        worst_cell.combo
    );
}

/// 🔴 **THE RATCHET IS INSIDE THE COMPOSITION, and this is the matrix that says what it does.**
///
/// The acceptance criterion of `Q-058` names *thrust + drive + winch + gravity + the joint + the
/// ratchet* — and `Ctrl` is the ratchet's key. The two `Ctrl`-free matrices above hold an
/// **absolute** bound (`worst excess ≤ 1 cm`). **With the reel held that bound is not reachable
/// by any solver**, and not because of anything `Q-058` built: `player::rope::shorten_ropes`
/// walks BOTH `limits.max` down toward `vector.min_rope_m` while the two anchors stay tens of
/// metres apart, so it asks for a pair of maxima that **no point in space satisfies** and avian
/// keeps one arm and abandons the other (`FIND-191`).
///
/// So this test asserts the claim `Q-058` actually made — **`Drive` INHERITS the rope `Pendulum`
/// has always had** — as a per-cell difference. It cannot be satisfied by weakening anything:
/// making `Drive` better than `Pendulum` fails it exactly as making it worse does.
///
/// # What this reads, and what it varies
///
/// Same lists as [`two_anchor_matrix`]'s header, plus the force model. **Not varied here and it
/// matters:** the reel SPEED (`reel_speed_m_s`, one shipped value) and the floor
/// (`min_rope_m` = 3 m, one value) — the pair that decides how fast the two maxima become
/// impossible.
#[test]
fn f3f_with_the_ratchet_held_no_arm_escapes_a_maximum_a_position_could_satisfy() {
    ctrl_matrix("air+Ctrl", AIR_SPOT_M);
}

/// The other stance of the same claim — `Ctrl` on the ground is `F-005`'s *„Spieler kann aus dem
/// Tiefpunkt Hoehe gewinnen"*, i.e. the key that takes the player off the floor, and it runs
/// through `ground_locomotion` and the winch's `in_flight == false` branch on the way.
#[test]
fn f3f_the_same_with_the_ratchet_held_and_the_player_standing_on_the_ground() {
    ctrl_matrix("ground+Ctrl", GROUND_SPOT_M);
}

/// `FIND-191` / `B-013`, **inverted on 2026-08-28 when the defect was fixed, and this is the
/// record of what it used to say** — because a test whose subject is repaired must not be
/// quietly deleted and must not stay written as if the repair had not happened.
///
/// **Until 2026-08-28 it asserted `drive > 10.0`**: *the reel can ask for two lengths no
/// position in space satisfies, and the left rope ends 50.167 m past its own maximum.* It said
/// so in its own failure message — *"`FIND-191` is either fixed or no longer reproduced by this
/// fixture"* — and that is exactly what happened: `player::rope::hold_the_pair` now stops the
/// reel before the two maxima stop having a common solution, and this fixture reads **0.0093 m**
/// where it read **50.167 m**.
///
/// 🔴 **What is left is the claim this fixture was always FOR, and it is not weaker: the reel is
/// ONE system, so the repair has to reach both force models identically.** `Pendulum` has
/// carried a `DistanceJoint` since `F-004`; `Drive` got one with `Q-058`. If the two ever part
/// company, `shorten_ropes` has stopped being one thing — and that cannot be satisfied by
/// making `Drive` better than `Pendulum` any more than by making it worse.
/// The metres live in `f004_two_far_apart_anchors_…` (84 cells); this is the differential.
#[test]
fn b013_the_two_rope_hold_reaches_both_force_models_because_the_reel_is_one_system() {
    let worst = |model| {
        let mut app = app();
        select(&mut app, model);
        let e = me(&mut app);
        let stand = Vec3::new(0.0, 300.0, 0.0);
        let eye = data(&app).game.player.eye_height_m;
        let hand = stand + Vec3::Y * eye;
        let half = 85.0f32.to_radians();
        let el = 20.0f32.to_radians();
        let dir =
            |a: f32| Vec3::new(a.sin() * el.cos(), el.sin(), -a.cos() * el.cos()).normalize();
        hang_two(&mut app, e, stand, hand + dir(-half) * 30.0, hand + dir(half) * 30.0);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ControlLeft);
        let mut worst_m = f32::NEG_INFINITY;
        for _ in 0..90 {
            app.update();
            let here = position(&app, e);
            for arm in arms(&mut app) {
                let max_m = arm.enforced_m.expect("every rope carries a joint since `Q-058`");
                worst_m = worst_m.max((arm.anchor_m - here).length() - max_m);
            }
        }
        worst_m
    };
    let drive = worst(RopeForceModel::Drive);
    let pendulum = worst(RopeForceModel::Pendulum);
    println!("find191 two anchors 170° apart, Ctrl held: Drive {drive:.4} m · Pendulum {pendulum:.4} m past the maximum");

    // 🔴 **The whole point, and it is unchanged since 2026-08-27.** `Pendulum` has carried a
    // `DistanceJoint` since `F-004` and `shorten_ropes` has been its reel the whole time. If the
    // two models disagree here, `shorten_ropes` has stopped being one system — before the fix
    // that would have meant `Q-058` caused `FIND-191`, and after it, that the repair reaches
    // only the shipped model.
    assert!(
        (drive - pendulum).abs() < 0.05,
        "Drive ended {drive:.4} m past a maximum and Pendulum {pendulum:.4} m — one reel, one \
         rope: a difference means `player::rope` is no longer one system for the two force models"
    );
    // The inverted half. `HOLD_EXCESS_TOL_M` is the same allowance the 84-cell sweep holds; this
    // fixture is the `170°`, `30 m / 30 m`, `y = 300` cell of it and must not be laxer.
    for (name, worst_m) in [("Drive", drive), ("Pendulum", pendulum)] {
        assert!(
            worst_m <= HOLD_EXCESS_TOL_M,
            "{name} left an arm {worst_m:.4} m past its own maximum — this fixture read 50.167 m \
             before `B-013` and 0.0093 m after, and a number back in between means the reel can \
             ask for a pair of maxima no position in space satisfies again"
        );
    }
}

/// **The probe behind `FIND-191`** — two anchors 170° apart, `Ctrl` held, both models.
///
/// Prints the per-tick truth: `limits.max` and the real distance for each arm. Run it with
/// `cargo test --test vector_rope probe_two_opposite -- --nocapture`.
#[test]
#[ignore = "a measurement, not a guard — see FIND-191"]
fn probe_two_opposite_anchors_with_the_reel_held() {
    for model in [RopeForceModel::Drive, RopeForceModel::Pendulum] {
        let mut app = app();
        select(&mut app, model);
        let e = me(&mut app);
        let stand = Vec3::new(0.0, 300.0, 0.0);
        let eye = data(&app).game.player.eye_height_m;
        let hand = stand + Vec3::Y * eye;
        let half = 85.0f32.to_radians();
        let el = 20.0f32.to_radians();
        let dir = |a: f32| Vec3::new(a.sin() * el.cos(), el.sin(), -a.cos() * el.cos()).normalize();
        let (bl, br) = hang_two(&mut app, e, stand, hand + dir(-half) * 30.0, hand + dir(half) * 30.0);
        println!("--- {model:?}: birth {bl:.3} / {br:.3} m, anchors 170° apart ---");
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ControlLeft);
        for t in 0..90 {
            app.update();
            if t % 6 != 0 {
                continue;
            }
            let here = position(&app, e);
            let v = velocity(&app, e).length();
            let row: Vec<String> = arms(&mut app)
                .iter()
                .map(|a| {
                    format!(
                        "{:?} max {:6.3} dist {:7.3}",
                        a.side,
                        a.enforced_m.unwrap_or(f32::NAN),
                        (a.anchor_m - here).length()
                    )
                })
                .collect();
            println!("  t{t:3} |v| {v:7.3}  {}", row.join("  |  "));
        }
    }
}

/// One second of `keys` for a hooked player standing at `scripts/f176-pull.txt`'s stance, as
/// **the distance to his anchor** — which is what `docs/NEXT.md` §3D's acceptance is about and
/// what the script harness cannot see (`src/debug/script.rs`: `rope` is the anchored ARM COUNT,
/// not a length).
fn distance_after_a_second_of(model: RopeForceModel, keys: &[KeyCode], hooked: bool) -> (f32, f32) {
    let mut app = app();
    select(&mut app, model);
    let e = me(&mut app);
    // ⚠️ **`warp`, not `place`.** `place` writes `Position` and the next tick's `Transform`
    // sync puts the body back where its `Transform` still says it is — a first version of this
    // fixture measured a player at the ORIGIN on a 63.5 m rope that was slack for every tick,
    // and every number it produced was about nothing at all.
    warp(&mut app, e, Vec3::new(45.0, 0.6, -43.0));
    ticks(&mut app, 40); // he lands and settles
    let start = position(&app, e);
    assert!(
        (start.xz() - Vec2::new(45.0, -43.0)).length() < 1.0,
        "the fixture is supposed to stand at (45, −43) and it stands at {start:?}"
    );
    // `scripts/f176-pull.txt`'s own anchor: body 149 at (45.00, 34.25, −29.00), up and at +Z,
    // so `S` walks him straight away from it. It is 36.72 m off and the rope is born there, so
    // the rope is TAUT from the first tick — which is the state §3D R1 is about.
    let anchor = Vec3::new(45.0, 34.25, -29.0);
    if hooked {
        hang_on(&mut app, e, start, anchor);
    }
    set_velocity(&mut app, e, Vec3::ZERO);
    let before = (anchor - position(&app, e)).length();
    for k in keys {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(*k);
    }
    for _ in 0..60 {
        // ⚠️ **`look 180 66.6`, exactly `scripts/f176-pull.txt`'s, and it is load-bearing.**
        // Measured 2026-08-26 and written into that script: the **+Z** side of this spot is
        // blocked, and a walk into it reads 0.915 m/s instead of 6.000. At the default yaw `S`
        // walks toward +Z, so a first version of this fixture measured a player pressed against
        // a wall — its control moved 0.028 m and called that "a man walking away". At yaw 180
        // `S` points at −Z, which is the clear direction AND the one away from the anchor.
        // The override is consumed every tick (`net::local::read_input` `take()`s it).
        app.world_mut().resource_mut::<defeated_by_titan::shared::LookOverride>().0 =
            Some((180.0f32.to_radians(), 66.6f32.to_radians()));
        app.update();
    }
    (before, (anchor - position(&app, e)).length())
}

/// `docs/NEXT.md` §3D **R1**, and it is the one the script can only see as a speed.
///
/// > *„wenn ich von seil weg gehe. also seil ist vorne und ich laufe zurück werde cih nicht ran
/// > gezogen. sonst werde ich ranzeogen!"* — read in §3D as **"the pull is unconditional; `S`
/// > is the one input that is allowed to fight it, and even then it may only slow the approach,
/// > not reverse it into a retreat."**
///
/// Measured 2026-08-27 over one second of `S` from a taut 36.72 m rope:
///
/// | | before | after |
/// |---|---|---|
/// | `Drive` | 36.723 | **35.637** — slowed to a crawl and still closing |
/// | `Pendulum` | 36.723 | 36.715 — he stops dead; **nothing pulls** |
/// | `Drive`, no rope | 36.723 | 39.619 — a man walking away at `run_speed_m_s` |
///
/// 🔴 **The two models are NOT alike here and that is the point.** `Pendulum`'s rope is a
/// constraint and nothing else, so `S` into a taut rope is a stand — which satisfies
/// `scripts/f176-pull.txt`'s `assert Speed < 2` and fails §3D R1. `Drive` keeps `FIND-172`'s
/// always-on pull on top of the same constraint, so `S` is *slowed approach* and not *retreat*,
/// which is R1 verbatim and reads 7.63 m/s on that same assert. **The assert cannot tell an
/// escape at 6 m/s from a haul at 12 m/s** — `CLAUDE.md` §6 rule 5, second half — and this test
/// is what tells them apart. → `docs/FINDINGS.md` FIND-192.
#[test]
fn f176_under_drive_walking_backwards_on_a_taut_rope_still_closes_on_the_anchor() {
    let (b_d, a_d) = distance_after_a_second_of(RopeForceModel::Drive, &[KeyCode::KeyS], true);
    let (b_p, a_p) = distance_after_a_second_of(RopeForceModel::Pendulum, &[KeyCode::KeyS], true);
    // 🔴 **The control §6 rule 5 demands: delete the rope and check the number moves.**
    let (b_f, a_f) = distance_after_a_second_of(RopeForceModel::Drive, &[KeyCode::KeyS], false);
    println!(
        "f176 one second of `S` from a taut rope, distance to the anchor: Drive {b_d:.3} → \
         {a_d:.3} m · Pendulum {b_p:.3} → {a_p:.3} m · no rope {b_f:.3} → {a_f:.3} m"
    );

    // One second of `run_speed_m_s` = 6 m at an angle to the rope, so the DISTANCE grows by
    // less than the walk: 2.896 m measured. Two metres is the band, and it is what tells a walk
    // from the 0.028 m a first version of this fixture read while pressed against the +Z wall.
    assert!(
        a_f > b_f + 2.0,
        "the control gained {:.3} m and it is supposed to be a man leaving — then this test is \
         measuring a wall, not a rope",
        a_f - b_f
    );
    // R1, and it is a SIGN: the distance may not grow. „aber NICHT das seil verlängern"
    // (`§3F`) is the same sentence from the other side.
    assert!(
        a_d < b_d,
        "one second of `S` on a taut rope took the player from {b_d:.3} m to {a_d:.3} m — §3D R1 \
         is that `S` may only SLOW the approach, never reverse it into a retreat"
    );
    // And it is really the always-on pull doing it, not the constraint: the model that has the
    // constraint and no pull does not close at all.
    assert!(
        (a_p - b_p).abs() < 1.0,
        "under `Pendulum` the same second moved him {:.3} m — if the constraint alone closes the \
         distance then the number above is not `FIND-172`'s pull and this test is mislabelled",
        b_p - a_p
    );
}

const PROBE_KEY: KeyCode = KeyCode::KeyS;

#[test]
#[ignore = "a probe — the per-tick truth behind FIND-192 and `scripts/f176-pull.txt` ACT 2/2b"]
fn probe_ground_walk_on_a_rope() {
    for model in [RopeForceModel::Drive, RopeForceModel::Pendulum] {
        let mut app = app();
        select(&mut app, model);
        let e = me(&mut app);
        warp(&mut app, e, Vec3::new(45.0, 0.6, -43.0));
        ticks(&mut app, 40);
        let start = position(&app, e);
        hang_on(&mut app, e, start, Vec3::new(45.0, 34.25, -29.0));
        set_velocity(&mut app, e, Vec3::ZERO);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(PROBE_KEY);
        println!("--- {model:?} {PROBE_KEY:?} start {start:?} ---");
        for t in 0..90 {
            app.update();
            if t % 9 != 0 { continue; }
            let st = *app.world().get::<defeated_by_titan::shared::MovementState>(e).unwrap();
            let ra = app.world().get::<defeated_by_titan::shared::RunAccel>(e).map(|r| r.0).unwrap_or(Vec3::ZERO);
            let here = position(&app, e);
            let max = arms(&mut app).first().map(|a| a.enforced_m.unwrap_or(f32::NAN)).unwrap_or(f32::NAN);
            let dist = arms(&mut app).first().map(|a| (a.anchor_m - here).length()).unwrap_or(f32::NAN);
            println!("  t{t:3} {st:?} y {:7.3} |v| {:7.3} runaccel {:8.2} max {max:7.3} dist {dist:7.3}", here.y, velocity(&app, e).length(), ra.length());
        }
    }
}

// ---------------------------------------------------------------------------------------
// `FIND-191` — **TWO ROPES MUST HOLD YOU, NOT BREAK.** The user was asked what should happen
// when both hooks sit on far-apart anchors and he holds `Ctrl`, and he chose:
//
//   „Beide Seile bleiben, du haengst fest — zwei Seile die sich widersprechen halten dich."
//
// 🔴 **He accepted being STUCK. He did not accept a BROKEN ROPE.** What the build did before
// 2026-08-28 was neither: `shorten_ropes` walked BOTH `limits.max` toward `vector.min_rope_m`
// while the anchors stayed where they were, and two maxima are satisfiable at all only if
// `Lₗ + Lᵣ ≥ dₐ`. Below that no position in space honours both, avian keeps one arm and
// abandons the other, and the player sits at 0.000 m/s with the other rope **50.167 m past its
// own maximum** — a constraint VIOLATION, not a stand-off. → `docs/BUGS.md` B-013.
// ---------------------------------------------------------------------------------------

/// The seven separations this sweep runs, and **0° is in it on purpose**: with the two ropes
/// the same length it puts both anchors on the same POINT, which is the `n = 1` case in
/// disguise and the one place a separation term can divide by nothing (`CLAUDE.md` §6 rule 5,
/// third shape).
const HOLD_SEPARATIONS_DEG: [f32; 7] = [0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0];
/// The two arms' lengths **against each other**. Three of the four disagree — `two_anchor_matrix`
/// holds both at 30 m, and an aggregate over two equal elements is exactly where a per-element
/// promise goes to die.
const HOLD_LENGTH_PAIRS_M: [(f32, f32); 4] =
    [(30.0, 30.0), (12.0, 48.0), (48.0, 12.0), (10.0, 10.0)];
/// The player's **height**, one column of the map so nothing but `y` changes: standing on clear
/// ground (`Grounded`, `ground_locomotion`), 40 m up (he reaches the ground inside the run) and
/// 300 m up (`Tethered` for every tick). This is the axis `f177_no_stance_…` was blind in.
const HOLD_HEIGHTS_M: [f32; 3] = [0.6, 40.0, 300.0];
/// A tick under this counts as **pinned**. `vector.max_speed_m_s` is 75; this is 1/1500 of it.
const HOLD_PINNED_M_S: f32 = 0.05;
/// The joint's own error under load is 2–5 mm (`docs/measurements/rope-decision.md`) and the
/// `Ctrl`-free 288-cell matrix reads a worst excess of 0.0050 m. This is ten times that, and it
/// is the whole allowance: `FIND-191` reads **50.167 m**.
///
/// ⚠️ **It was set before the fix existed and it has not been moved since** — the sweep read
/// **51.7104 m** against it on the unfixed build and **0.0092 m** on the shipped one. The
/// intermediate build, with the exact rule and no `vector.min_rope_m` of play in hand, reads
/// **0.2810 m** and fails this line: that is the tangent-boundary sag, and it is `FIND-198`
/// and `Q-079`, not a tolerance question.
const HOLD_EXCESS_TOL_M: f32 = 0.05;

/// One cell of the two-rope hold sweep.
#[derive(Clone, Copy, Debug)]
struct HoldCell {
    separation_deg: f32,
    left_m: f32,
    right_m: f32,
    height_m: f32,
    /// The distance between the two anchor markers **as the game's own components report it**,
    /// not as this test computed it (`CLAUDE.md` §6 rule 5, fourth shape: the provenance of the
    /// input is an axis too).
    anchors_apart_m: f32,
    /// `distance − limits.max`, worst over every tick and both arms. Negative = slack.
    worst_excess_m: f32,
    /// The shortest `limits.max` any arm reached — the reel's own achievement, and what says
    /// whether the feasibility rule blocked a reel that should have been free to run.
    shortest_max_m: f32,
    /// Ticks where `Lₗ + Lᵣ < dₐ`, i.e. where the pair asked for a position that does not exist.
    ticks_infeasible: u32,
    /// Ticks with fewer than two ropes. **Counted, not skipped** — a cell that lost a rope is
    /// not a two-anchor cell and must not vanish into the denominator.
    ticks_with_one_rope: u32,
    /// Ticks at under [`HOLD_PINNED_M_S`]. Being held still is what the user CHOSE; it is
    /// reported, never asserted on.
    ticks_pinned: u32,
    /// Ticks pinned **while an arm was past its maximum or the pair was infeasible**. That is
    /// the defect, and it is the number that has to be zero.
    ticks_pinned_by_a_violation: u32,
}

/// 🔴 **What the code reads, and what this sweep varies. The difference is the bug.**
///
/// **Read by `player::rope::shorten_ropes`, the system under test:** `DistanceJoint::limits.max`
/// of **both** ropes, the `Position` of **both** anchor markers, the player's `Position`,
/// `ReelSpeed::m_s` per side, `Has<HitStop>`, `Time<Substeps>::delta_secs()`, and out of
/// `game.ron` `vector.min_rope_m` and (through `vector::gas`) `vector.reel_speed_m_s`.
/// **Read by the rest of the tick it lives in:** `Intent` (`move_x`, `move_y`, `yaw`, pitch),
/// `Buttons::REEL_IN`, `LinearVelocity`, `MovementState`, `Gas`/`GasGrant`, `Hook` for both
/// arms, `Transform`, and out of `game.ron` the drive numbers, `gravity_m_s2`, `max_speed_m_s`,
/// `eye_height_m`, `run_speed_m_s`.
///
/// **Varied here:** the anchors' angular separation (7, **including 0°**), the two ropes'
/// lengths against each other (4 pairs, 3 asymmetric), the player's height (3), and — as a
/// consequence of all three — the real anchor separation `dₐ`, which runs from **0.000 m** to
/// **95.7 m** and is the quantity the rule under test is written in.
/// **`Ctrl` is held in every cell**: it is the one input that can ask for the impossible.
///
/// **NOT varied, and each is a hole somebody may fall into later:** `yaw` (0°, one value —
/// `two_anchor_matrix` sweeps four and finds the same defect); the key combo (none besides
/// `Ctrl`); the force model (`Drive`, the shipped one — `find191_…` above is what proves the
/// two models are the same defect); the anchors' elevation above the hand (20°); pitch (0);
/// the gas (full); `game.ron` (shipped values); and the number of arms — **always exactly two,
/// which is the premise of this game and not an edge case**.
///
/// **Nothing is skipped.** There is no `continue` in the per-tick loop: a cell that loses a rope
/// is counted in [`HoldCell::ticks_with_one_rope`] and asserted on separately.
fn two_rope_hold_sweep() -> (Vec<HoldCell>, f32) {
    let mut cells = Vec::new();
    let mut min_rope_m = f32::NAN;
    for &height_m in &HOLD_HEIGHTS_M {
        for &(left_m, right_m) in &HOLD_LENGTH_PAIRS_M {
            for &separation_deg in &HOLD_SEPARATIONS_DEG {
                let mut app = app();
                select(&mut app, RopeForceModel::Drive);
                let e = me(&mut app);
                let d = data(&app);
                min_rope_m = d.game.vector.min_rope_m;
                // One column of the map — `scripts/f176-pull.txt`'s clear ground — so that
                // **only `y` changes** between the three heights.
                let stand_m = Vec3::new(45.0, height_m, -43.0);
                let hand = stand_m + Vec3::Y * d.game.player.eye_height_m;
                let half = (separation_deg * 0.5).to_radians();
                let el = 20.0f32.to_radians();
                let dir = |a: f32| {
                    Vec3::new(a.sin() * el.cos(), el.sin(), -a.cos() * el.cos()).normalize()
                };
                hang_two(
                    &mut app,
                    e,
                    stand_m,
                    hand + dir(-half) * left_m,
                    hand + dir(half) * right_m,
                );
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::ControlLeft);

                let mut cell = HoldCell {
                    separation_deg,
                    left_m,
                    right_m,
                    height_m,
                    anchors_apart_m: f32::NAN,
                    worst_excess_m: f32::NEG_INFINITY,
                    shortest_max_m: f32::INFINITY,
                    ticks_infeasible: 0,
                    ticks_with_one_rope: 0,
                    ticks_pinned: 0,
                    ticks_pinned_by_a_violation: 0,
                };
                for _ in 0..120 {
                    app.update();
                    let here = position(&app, e);
                    let speed_m_s = velocity(&app, e).length();
                    let found = arms(&mut app);
                    if found.len() < 2 {
                        cell.ticks_with_one_rope += 1;
                    }
                    let mut tick_excess_m = f32::NEG_INFINITY;
                    let mut budget_m = 0.0f32;
                    for arm in &found {
                        let max_m =
                            arm.enforced_m.expect("every rope carries a joint since `Q-058`");
                        // The joint constrains the body ORIGIN — both local anchors are
                        // `Vec3::ZERO` (`player::rope::attach_ropes`), so this is the very
                        // distance `DistanceLimit` measures.
                        tick_excess_m = tick_excess_m.max((arm.anchor_m - here).length() - max_m);
                        budget_m += max_m;
                        cell.shortest_max_m = cell.shortest_max_m.min(max_m);
                    }
                    // 🔴 The separation the GAME's own anchor markers have. Reading the two
                    // points this test invented would make the oracle and the code agree about
                    // a number neither of them got from the world.
                    let apart_m = if found.len() == 2 {
                        (found[0].anchor_m - found[1].anchor_m).length()
                    } else {
                        f32::NAN
                    };
                    cell.anchors_apart_m = apart_m;
                    // `Lₗ + Lᵣ ≥ dₐ` — the triangle inequality, and the whole of the rule.
                    let feasible = found.len() == 2 && budget_m >= apart_m - 0.01;
                    if !feasible {
                        cell.ticks_infeasible += 1;
                    }
                    cell.worst_excess_m = cell.worst_excess_m.max(tick_excess_m);
                    if speed_m_s < HOLD_PINNED_M_S {
                        cell.ticks_pinned += 1;
                        if !feasible || tick_excess_m > HOLD_EXCESS_TOL_M {
                            cell.ticks_pinned_by_a_violation += 1;
                        }
                    }
                }
                cells.push(cell);
            }
        }
    }
    (cells, min_rope_m)
}

/// 🔴 **`F-004`'s own note has said „Two ropes at once is unmeasured" since 2026-08-09. This is
/// that measurement, and it is the acceptance for `FIND-191`/`B-013`.**
///
/// 84 cells (3 heights × 4 length pairs × 7 separations), `Ctrl` held in every one of them,
/// 120 ticks each. Four claims, and the last one is what keeps the fix from being a brake:
///
/// 1. **No arm is ever further from its own anchor than its own `limits.max`** allows.
///    `FIND-191` reads **50.167 m** past; the allowance here is [`HOLD_EXCESS_TOL_M`], and the
///    shipped build reads **0.0092 m** — 5 600x, and this fixture is where the number comes
///    from. The rule under test is `player::rope::hold_the_pair`.
/// 2. **The two maxima are simultaneously satisfiable on every tick** — `Lₗ + Lᵣ ≥ dₐ`.
/// 3. **Nobody is ever pinned at 0.000 m/s BY a violation.** Being held between two ropes that
///    contradict each other is what the user chose; being dragged onto one anchor while the
///    other rope is 50 m past its maximum is not.
/// 4. **The same-anchor case is untouched.** Where the two anchors are one point (`0°` with two
///    equal ropes) the reel still reaches `vector.min_rope_m` — the feasibility rule must not
///    cost the one-anchor player his reel, and that is the `n = 1` case a separation term is
///    most likely to break.
#[test]
fn f004_two_far_apart_anchors_hold_the_player_instead_of_dragging_him_past_a_maximum() {
    let (cells, min_rope_m) = two_rope_hold_sweep();
    let mut worst = cells.clone();
    worst.sort_by(|a, b| b.worst_excess_m.total_cmp(&a.worst_excess_m));
    println!(
        "two-rope hold sweep: {} cells, Ctrl held, 120 ticks each, min_rope_m {min_rope_m:.3} m",
        cells.len()
    );
    println!("  sep°  L_l   L_r   y      d_a      worst excess  shortest max  infeasible  pinned  pinned-by-violation  one-rope");
    for c in worst.iter().take(10) {
        println!(
            "  {:5.0} {:5.1} {:5.1} {:6.1} {:8.3} {:13.4} {:13.3} {:11} {:7} {:20} {:9}",
            c.separation_deg,
            c.left_m,
            c.right_m,
            c.height_m,
            c.anchors_apart_m,
            c.worst_excess_m,
            c.shortest_max_m,
            c.ticks_infeasible,
            c.ticks_pinned,
            c.ticks_pinned_by_a_violation,
            c.ticks_with_one_rope
        );
    }
    let ticks_total: u32 = cells.len() as u32 * 120;
    let infeasible: u32 = cells.iter().map(|c| c.ticks_infeasible).sum();
    let one_rope: u32 = cells.iter().map(|c| c.ticks_with_one_rope).sum();
    let pinned: u32 = cells.iter().map(|c| c.ticks_pinned).sum();
    let by_violation: u32 = cells.iter().map(|c| c.ticks_pinned_by_a_violation).sum();
    println!(
        "  totals over {ticks_total} ticks: infeasible {infeasible} · one rope {one_rope} · \
         pinned {pinned} · pinned BY A VIOLATION {by_violation}"
    );

    let top = worst[0];
    assert!(
        top.worst_excess_m <= HOLD_EXCESS_TOL_M,
        "an arm ended {:.4} m past its own maximum ({:.0}° apart, ropes {:.1}/{:.1} m, y {:.1}, \
         anchors {:.3} m apart) — `FIND-191` read 50.167 m and the rule that two maxima must \
         stay simultaneously satisfiable is what has to stop it",
        top.worst_excess_m,
        top.separation_deg,
        top.left_m,
        top.right_m,
        top.height_m,
        top.anchors_apart_m
    );
    assert_eq!(
        infeasible, 0,
        "the reel asked for a pair of maxima no position in space satisfies on {infeasible} of \
         {ticks_total} ticks — `Lₗ + Lᵣ ≥ dₐ` is the whole rule and it may not be broken once"
    );
    assert_eq!(
        by_violation, 0,
        "the player was pinned under {HOLD_PINNED_M_S} m/s by a VIOLATION on {by_violation} of \
         {ticks_total} ticks — being held by two ropes that contradict each other is what the \
         user chose, being dragged onto one anchor with the other rope past its maximum is not"
    );
    assert_eq!(
        one_rope, 0,
        "{one_rope} of {ticks_total} ticks had fewer than two ropes — the cells are not \
         two-anchor cells and every number above is about something else"
    );
    // 4. The `n = 1` case in disguise, and it must be untouched.
    let same_point: Vec<&HoldCell> = cells.iter().filter(|c| c.anchors_apart_m < 0.01).collect();
    assert!(
        !same_point.is_empty(),
        "no cell put both anchors on the same point — the one-anchor case is not being covered \
         at all and `HOLD_SEPARATIONS_DEG`/`HOLD_LENGTH_PAIRS_M` no longer produce it"
    );
    for c in same_point {
        assert!(
            (c.shortest_max_m - min_rope_m).abs() < 0.05,
            "both arms hang on ONE point ({:.3} m apart) and the reel only got to {:.3} m \
             instead of vector.min_rope_m = {min_rope_m:.3} — the feasibility rule has taken \
             the reel away from a player it was never about",
            c.anchors_apart_m,
            c.shortest_max_m
        );
    }
}
