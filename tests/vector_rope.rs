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
use defeated_by_titan::data::GameData;
use defeated_by_titan::shared::{
    AimPoint, BodyId, BodyMask, Cli, Hook, HookReleased, HookState, IndexEntry, LocalPlayer,
    PlayerId, ReleaseReason, RopeLength, Side, SimulationSystems, SpatialIndex, WarpPlayer,
};
use defeated_by_titan::vector::aim::aim;

// ---------------------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------------------

/// What `vector::aim` would write if `T-036a` were built. See the module header.
#[derive(Component, Clone, Copy, Debug, Default)]
struct ForcedAim(AimPoint);

fn force_aim(mut players: Query<(&ForcedAim, &mut AimPoint)>) {
    for (forced, mut point) in &mut players {
        if *point != forced.0 {
            *point = forced.0;
        }
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
/// every rope of the player it moves — including one anchored in the very tick of the warp
/// (`player::rope::attach_ropes`). Pinning a player with a warp while his hook is in the air is
/// therefore a way to build a rope that is destroyed the moment it exists. `place` writes
/// `Position` and does what this harness actually means: move the body, touch nothing else.
fn hang(app: &mut App, e: Entity, player_pos: Vec3, nominal_length_m: f32) -> f32 {
    let anchor = player_pos + Vec3::Y * nominal_length_m;
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
