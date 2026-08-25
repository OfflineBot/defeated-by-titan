//! **What decides a sortie** — the modes, and the one function that judges them.
//!
//! Until 2026-08-25 this game had exactly one mission: *kill `kill_target` titans before
//! `target_duration_s` runs out.* That was never a design decision. `mission::decide` read a
//! tally and a clock because a tally and a clock were the only two things anybody had written,
//! and every one of the four templates in `missions.ron` was the same mission with different
//! numbers in it. `F-072`, `F-073` and `F-185` are three modes the design asked for and none of
//! them fits in that shape.
//!
//! ```text
//!   Cull      kill n titans           ─ the clock LOSES it
//!   Breach    hold the gate           ─ the clock WINS it   ⚠️ inverted
//!   Parcours  fly the rings, in order ─ the clock LOSES it
//!   Escort    walk the cart home      ─ the clock LOSES it
//! ```
//!
//! ## The one thing in this file that is worth a red test on its own
//!
//! **The clock does not mean the same thing in every mode.** A breach that lost on the deadline
//! instead of winning on it would be a mission nobody can ever finish, and it would look
//! completely right in review — one character, `Won` against `Lost`, inside a match arm nobody
//! reads twice. So the judgement is [`verdict`]: a **pure function** of the mode and six facts,
//! with its own unit tests below, and `mission::decide` does nothing but gather the six facts
//! and hand them over. `tests/mission.rs` holds the same claim end to end in a real app.
//!
//! ## Where the numbers are, and where they are not
//!
//! Every number of every mode is in `missions.ron: templates.<m>.objective` — the gate's place
//! and reach, the rings, the cart's speed. **Nothing here is a literal** except
//! [`MARKER_HEIGHT_M`], which is how thick a ring marker is drawn and is not a game value.
//!
//! ## What a mode may not do, and why the progress lives on the mission entity
//!
//! [`GateWatch`], [`Course`] and [`Haul`] all sit on the **mission** entity, next to
//! [`MissionClock`](super::run::MissionClock) and [`KillTally`](super::run::KillTally), and the
//! props in the world ([`Cart`], [`RingMarker`]) carry no state of their own. There is one
//! sortie, so there is one answer to "how far along is it"; a cart that knew where it was and a
//! mission that also knew would be two answers, and the first co-op session would find them
//! disagreeing. The props are `DespawnOnExit(MissionPhase::Active)` and the progress is
//! despawned with the mission in `hub::open_hub` — the same two lifetimes the domain already
//! had.
//!
//! ## What titans do NOT do, said out loud because a reader will look for it
//!
//! ⚠️ **A titan walks at the nearest player, never at a place** (`titan::brain::nearest_player`).
//! There is no goal pathing in this game and this module does not add any — `titan` is another
//! domain and its brain is not `mission`'s to write. So a breach is defended by **standing
//! between them and the gate**, which is what a defence mission is, and it also means a player
//! who runs 200 m away takes the whole wave with him and the gate is never touched.
//! **That hole is real and it is not tuned shut this round** — it is written here rather than
//! papered over, and the report carries it.

use std::collections::BTreeSet;

use bevy::prelude::*;

use crate::data::{Objective, Ring};
use crate::shared::{Block, PlayerId, TitanId, TitanState};

use super::phase::MissionPhase;

/// How thick a ring marker and the cart's marker are drawn. **Not a game value** — nothing
/// reads it but the mesh, and the trigger is the radius out of the file.
const MARKER_HEIGHT_M: f32 = 0.3;

// ---------------------------------------------------------------------------
// The mode, and the progress of each mode — all of it on the mission entity
// ---------------------------------------------------------------------------

/// **The mode this sortie is flying**, copied onto the mission at deploy out of
/// `missions.ron: templates.<m>.objective`.
///
/// A component and not a resource for the same reason [`Mission`](super::run::Mission) is one:
/// it is a property of *this* sortie, and it has to stop existing with it.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MissionGoal(pub Objective);

/// `F-072` — **how many titans have been through the gate.**
///
/// `through` is a set and not a counter for exactly the reason
/// [`KillTally`](super::run::KillTally)`::credited` is one: a titan standing at the gate is at
/// the gate on every tick, and a counter that took each tick would lose the sortie in the first
/// second off a single body. **One titan is one breach, forever.**
#[derive(Component, Clone, Debug, PartialEq)]
pub struct GateWatch {
    /// Where the gate stands.
    pub gate: Vec3,
    /// How close a titan has to get to be through it.
    pub reach_m: f32,
    /// How many may be through before the sortie is lost.
    pub allowed: u32,
    through: BTreeSet<TitanId>,
}

impl GateWatch {
    pub fn new(gate: Vec3, reach_m: f32, allowed: u32) -> Self {
        GateWatch { gate, reach_m, allowed, through: BTreeSet::new() }
    }

    /// Books one titan as through. `false` when he was already counted.
    pub fn note(&mut self, titan: TitanId) -> bool {
        self.through.insert(titan)
    }

    /// How many distinct titans have reached it.
    pub fn count(&self) -> u32 {
        self.through.len() as u32
    }

    /// Whether the gate has fallen. `allowed` is how many may get through, so the loss is at
    /// **more** than that — `breaches_allowed: 0` is "not one of them".
    pub fn fallen(&self) -> bool {
        self.count() > self.allowed
    }

    /// Whether this position is inside the gate.
    pub fn reaches(&self, at: Vec3) -> bool {
        at.distance(self.gate) <= self.reach_m
    }
}

/// One ring of a parcours, in world units. The file's [`Ring`] converted **once**, at the
/// boundary, exactly like `missions.ron`'s seconds become ticks in
/// [`to_ticks`](super::run::to_ticks).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CourseGate {
    pub at: Vec3,
    pub radius_m: f32,
}

/// `F-185` — **how far through the parcours the squad is.**
///
/// `passed` is an index and not a set, and that is the whole feature: the rings have to be
/// flown **in the order the file lists them**. A parcours you can clear by standing in ring
/// four teaches nothing, and the point of `F-185` is that it is the only thing in this game
/// that teaches the drive at all.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Course {
    pub gates: Vec<CourseGate>,
    passed: usize,
}

impl Course {
    pub fn new(rings: &[Ring]) -> Self {
        Course {
            gates: rings
                .iter()
                .map(|r| CourseGate { at: Vec3::from(r.center_m), radius_m: r.radius_m })
                .collect(),
            passed: 0,
        }
    }

    /// The ring that has to be flown next, or `None` when the course is done.
    pub fn next_gate(&self) -> Option<(usize, CourseGate)> {
        self.gates.get(self.passed).map(|g| (self.passed, *g))
    }

    /// Books the next ring as flown. Returns the index that was just cleared.
    pub fn pass(&mut self) -> Option<usize> {
        let index = self.passed;
        if index >= self.gates.len() {
            return None;
        }
        self.passed += 1;
        Some(index)
    }

    pub fn passed(&self) -> usize {
        self.passed
    }

    /// Whether every ring has been flown. **An empty course is not done** — a parcours with no
    /// rings in the file would otherwise be won at tick 0, which is the same shape as
    /// `KillTally::reached` refusing a target of zero.
    pub fn done(&self) -> bool {
        !self.gates.is_empty() && self.passed >= self.gates.len()
    }
}

/// `F-073` — **where the cart is, and how far it still has to go.**
///
/// The position is `(leg, along_m)` and not a `Vec3`: a cart that stored its own point would
/// drift off its path by an f32 epsilon per tick and arrive somewhere near the last waypoint
/// instead of on it. [`Haul::at`] derives the point, and [`Haul::roll`] is a pure function with
/// its own test — **a step longer than the leg it is on has to carry over into the next one**,
/// or a fast cart walks one waypoint per tick and stops early.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Haul {
    pub waypoints: Vec<Vec3>,
    pub speed_m_s: f32,
    pub escort_radius_m: f32,
    /// The waypoint the cart is walking **away from**.
    leg: usize,
    /// How far along that leg it is, in meters.
    along_m: f32,
}

impl Haul {
    pub fn new(waypoints: &[(f32, f32, f32)], speed_m_s: f32, escort_radius_m: f32) -> Self {
        Haul {
            waypoints: waypoints.iter().copied().map(Vec3::from).collect(),
            speed_m_s,
            escort_radius_m,
            leg: 0,
            along_m: 0.0,
        }
    }

    /// Where the cart stands right now.
    pub fn at(&self) -> Vec3 {
        let Some(from) = self.waypoints.get(self.leg) else {
            return self.waypoints.last().copied().unwrap_or(Vec3::ZERO);
        };
        let Some(to) = self.waypoints.get(self.leg + 1) else {
            return *from;
        };
        let leg = *to - *from;
        let length = leg.length();
        if length <= f32::EPSILON {
            return *from;
        }
        *from + leg * (self.along_m / length).clamp(0.0, 1.0)
    }

    /// Whether it is home. **A haul with fewer than two waypoints is never home** — a path
    /// nobody wrote must not be a mission that is won before it starts.
    pub fn arrived(&self) -> bool {
        self.waypoints.len() >= 2 && self.leg + 1 >= self.waypoints.len()
    }

    /// Rolls the cart `step_m` further along its path, carrying the remainder over into every
    /// leg it crosses. Stops on the last waypoint.
    pub fn roll(&mut self, step_m: f32) {
        if step_m <= 0.0 {
            return;
        }
        let mut left = step_m;
        while left > 0.0 && !self.arrived() {
            let (Some(from), Some(to)) =
                (self.waypoints.get(self.leg), self.waypoints.get(self.leg + 1))
            else {
                return;
            };
            let length = from.distance(*to);
            let rest = length - self.along_m;
            if left < rest {
                self.along_m += left;
                return;
            }
            // The leg is finished; what is left of the step continues on the next one.
            left -= rest.max(0.0);
            self.leg += 1;
            self.along_m = 0.0;
        }
    }
}

/// The picture of the cart. **Carries no state** — [`roll_the_cart`] writes its `Transform` off
/// the mission's [`Haul`] every tick.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cart;

/// The picture of one ring, by its index in the [`Course`]. Despawned when it is flown, which
/// is the only feedback a parcours has until `hud` learns about modes.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RingMarker(pub usize);

// ---------------------------------------------------------------------------
// The judgement — one pure function, and it is the whole feature
// ---------------------------------------------------------------------------

/// What the sortie has achieved, in the one shape [`verdict`] needs.
///
/// Six booleans and not six queries: the gathering is `mission::decide`'s job and the
/// **judging** is this file's, and that split is what lets the inversion below be a unit test
/// instead of a 19 800-tick app run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Achieved {
    /// `KillTally::reached` — `kill_target` cortex cuts are in.
    pub kills_reached: bool,
    /// `GateWatch::fallen` — more titans got through than the file allows.
    pub gate_fallen: bool,
    /// `Course::done` — every ring is flown.
    pub course_done: bool,
    /// `Haul::arrived` — the cart is on its last waypoint.
    pub cart_home: bool,
    /// `MissionClock::expired` — the deadline tick has come.
    pub expired: bool,
    /// Every player is out of the fight.
    pub everybody_down: bool,
}

/// **The verdict, and the one place that knows what the clock means.**
///
/// `None` is "nothing has been decided yet" and it is the normal answer on almost every tick.
///
/// The win is checked before the loss in every mode **except** [`Objective::Breach`], where
/// letting them through is the loss and it beats the deadline: a gate that fell on the last
/// tick did fall, and "you held it" would be a lie the player watched happen.
pub fn verdict(goal: &Objective, a: Achieved) -> Option<MissionPhase> {
    match goal {
        // The mission this game had before 2026-08-25, unchanged to the tick.
        Objective::Cull => {
            if a.kills_reached {
                Some(MissionPhase::Won)
            } else if a.expired || a.everybody_down {
                Some(MissionPhase::Lost)
            } else {
                None
            }
        }
        // ⚠️ **The inversion.** Surviving to the deadline is the win.
        Objective::Breach { .. } => {
            if a.gate_fallen || a.everybody_down {
                Some(MissionPhase::Lost)
            } else if a.expired {
                Some(MissionPhase::Won)
            } else {
                None
            }
        }
        Objective::Parcours { .. } => {
            if a.course_done {
                Some(MissionPhase::Won)
            } else if a.expired || a.everybody_down {
                Some(MissionPhase::Lost)
            } else {
                None
            }
        }
        Objective::Escort { .. } => {
            if a.cart_home {
                Some(MissionPhase::Won)
            } else if a.expired || a.everybody_down {
                Some(MissionPhase::Lost)
            } else {
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Building the mode into the world
// ---------------------------------------------------------------------------

/// The progress component a mode needs, ready to be inserted on the mission entity.
///
/// One function so that `mission::deploy` has no `match` of its own — a second place that maps
/// a mode onto its state is a second place that can forget a variant.
pub fn progress_of(goal: &Objective) -> ObjectiveProgress {
    match goal {
        Objective::Cull => ObjectiveProgress::Cull,
        Objective::Breach { gate_m, reach_m, breaches_allowed } => ObjectiveProgress::Breach(
            GateWatch::new(Vec3::from(*gate_m), *reach_m, *breaches_allowed),
        ),
        Objective::Parcours { rings } => ObjectiveProgress::Parcours(Course::new(rings)),
        Objective::Escort { waypoints, speed_m_s, escort_radius_m, .. } => {
            ObjectiveProgress::Escort(Haul::new(waypoints, *speed_m_s, *escort_radius_m))
        }
    }
}

/// What [`progress_of`] hands back. An enum and not four `Option`s, because a sortie flies
/// exactly one mode.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectiveProgress {
    Cull,
    Breach(GateWatch),
    Parcours(Course),
    Escort(Haul),
}

/// Puts the mode's props into the world: the rings of a parcours, the cart of an escort, the
/// gate of a breach. **All of them `DespawnOnExit(MissionPhase::Active)`** — a ring that
/// outlived its sortie would stand in the hub.
///
/// Amber throughout, and that is the rule and not a taste: `docs/conventions.md` §3 reserves
/// amber for "cortex, weak points, **objectives**", and every one of these is the objective.
pub fn furnish(commands: &mut Commands, goal: &Objective, amber: [f32; 3]) {
    match goal {
        Objective::Cull => {}
        Objective::Breach { gate_m, reach_m, .. } => {
            commands.spawn((
                Name::new("breach_gate"),
                Block {
                    size: Vec3::new(reach_m * 2.0, MARKER_HEIGHT_M, reach_m * 2.0),
                    color: amber,
                },
                Transform::from_translation(Vec3::from(*gate_m)),
                DespawnOnExit(MissionPhase::Active),
            ));
        }
        Objective::Parcours { rings } => {
            for (index, ring) in rings.iter().enumerate() {
                commands.spawn((
                    Name::new(format!("parcours_ring_{index}")),
                    RingMarker(index),
                    Block {
                        size: Vec3::new(ring.radius_m * 2.0, MARKER_HEIGHT_M, ring.radius_m * 2.0),
                        color: amber,
                    },
                    Transform::from_translation(Vec3::from(ring.center_m)),
                    DespawnOnExit(MissionPhase::Active),
                ));
            }
        }
        Objective::Escort { waypoints, size_m, .. } => {
            let start = waypoints.first().copied().unwrap_or((0.0, 0.0, 0.0));
            commands.spawn((
                Name::new("escort_cart"),
                Cart,
                Block { size: Vec3::from(*size_m), color: amber },
                Transform::from_translation(Vec3::from(start)),
                DespawnOnExit(MissionPhase::Active),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// The three systems — all of them `PostStep`, all of them `Active` only
// ---------------------------------------------------------------------------

/// `F-072`: books every titan that has reached the gate.
///
/// **A dying titan does not breach.** A body keeps its [`TitanId`] for `death_s` after the
/// cortex cut (`titan::brain::dissolve`), so a husk cut down *on* the gate would otherwise be
/// counted as through it a second after he stopped being a threat — the same trap
/// `KillTally::credit` pays for from the other side.
pub fn watch_the_gate(
    titans: Query<(&TitanId, &TitanState, &Transform)>,
    mut watches: Query<&mut GateWatch>,
) {
    for mut watch in &mut watches {
        for (id, state, at) in &titans {
            if *state == TitanState::Death {
                continue;
            }
            if watch.reaches(at.translation) && watch.note(*id) {
                info!(
                    "breach: titan {} reached the gate — {}/{} allowed",
                    id.0,
                    watch.count(),
                    watch.allowed
                );
            }
        }
    }
}

/// `F-185`: books the next ring when a player flies through it, and takes its marker away.
///
/// **Only the next one.** The rings are an order, and a player who cuts the corner past ring
/// three has not flown the parcours — that is the difference between a course and a scavenger
/// hunt, and `tests/mission.rs::f185_a_ring_flown_out_of_order_counts_for_nothing` is the half
/// of this that a "he touched all of them" test cannot see.
pub fn fly_the_course(
    mut commands: Commands,
    players: Query<&Transform, With<PlayerId>>,
    mut courses: Query<&mut Course>,
    markers: Query<(Entity, &RingMarker)>,
) {
    for mut course in &mut courses {
        let Some((index, gate)) = course.next_gate() else {
            continue;
        };
        let flown = players.iter().any(|p| p.translation.distance(gate.at) <= gate.radius_m);
        if !flown {
            continue;
        }
        course.pass();
        info!("parcours: ring {} flown — {}/{}", index, course.passed(), course.gates.len());
        for (entity, marker) in &markers {
            if marker.0 == index {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// `F-073`: rolls the cart while somebody is escorting it, and moves its picture with it.
///
/// **It only moves while a player is inside `escort_radius_m`.** That is the whole mode: the
/// cart is not a timer you wait out, it is a thing you have to stay next to while the titans
/// that spawned around it want you somewhere else.
pub fn roll_the_cart(
    time: Res<Time<Fixed>>,
    players: Query<&Transform, With<PlayerId>>,
    mut hauls: Query<&mut Haul>,
    mut carts: Query<&mut Transform, (With<Cart>, Without<PlayerId>)>,
) {
    let dt = time.delta_secs();
    for mut haul in &mut hauls {
        let here = haul.at();
        let escorted =
            players.iter().any(|p| p.translation.distance(here) <= haul.escort_radius_m);
        if escorted && !haul.arrived() {
            let step = haul.speed_m_s * dt;
            haul.roll(step);
        }
        let now = haul.at();
        for mut cart in &mut carts {
            cart.translation = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breach() -> Objective {
        Objective::Breach { gate_m: (0.0, 0.0, 0.0), reach_m: 5.0, breaches_allowed: 2 }
    }

    fn parcours() -> Objective {
        Objective::Parcours {
            rings: vec![
                Ring { center_m: (0.0, 10.0, 0.0), radius_m: 5.0 },
                Ring { center_m: (40.0, 10.0, 0.0), radius_m: 5.0 },
            ],
        }
    }

    fn escort() -> Objective {
        Objective::Escort {
            waypoints: vec![(0.0, 0.0, 0.0), (10.0, 0.0, 0.0)],
            speed_m_s: 2.0,
            escort_radius_m: 8.0,
            size_m: (2.0, 2.0, 3.0),
        }
    }

    #[test]
    fn the_clock_loses_a_cull_and_wins_a_breach() {
        // ⭐ **The one that matters.** One character apart in two match arms, and getting it
        // wrong is a defence mission nobody can ever finish — while every test that only asks
        // "does it end" stays green.
        let out_of_time = Achieved { expired: true, ..default() };
        assert_eq!(verdict(&Objective::Cull, out_of_time), Some(MissionPhase::Lost));
        assert_eq!(verdict(&breach(), out_of_time), Some(MissionPhase::Won));
        assert_eq!(verdict(&parcours(), out_of_time), Some(MissionPhase::Lost));
        assert_eq!(verdict(&escort(), out_of_time), Some(MissionPhase::Lost));
    }

    #[test]
    fn a_gate_that_fell_on_the_deadline_tick_is_still_a_loss() {
        // The one place the win is *not* checked first. Both facts are true on the same tick;
        // "you held it" would be a lie the player just watched not happen.
        let both = Achieved { expired: true, gate_fallen: true, ..default() };
        assert_eq!(verdict(&breach(), both), Some(MissionPhase::Lost));
    }

    #[test]
    fn nothing_is_decided_while_nothing_has_happened() {
        // The normal answer on almost every tick of every mode. A mode that decided on an empty
        // `Achieved` would end its sortie at tick 0.
        let nothing = Achieved::default();
        for goal in [Objective::Cull, breach(), parcours(), escort()] {
            assert_eq!(verdict(&goal, nothing), None, "{goal:?} decided itself out of nothing");
        }
    }

    #[test]
    fn every_mode_is_lost_when_every_player_is_down() {
        // `docs/PLAN-GAME.md` §1: one way to win, two ways to lose — and the second one is not
        // a property of the cull, it is a property of the game.
        let down = Achieved { everybody_down: true, ..default() };
        for goal in [Objective::Cull, breach(), parcours(), escort()] {
            assert_eq!(verdict(&goal, down), Some(MissionPhase::Lost), "{goal:?}");
        }
    }

    #[test]
    fn each_mode_is_won_by_its_own_achievement_and_by_nobody_elses() {
        // ⭐ The control. Without it, a `verdict` that answered `Won` to any true flag at all
        // would pass every test above — the parcours would be won by a kill and the escort by a
        // ring.
        let kills = Achieved { kills_reached: true, ..default() };
        let course = Achieved { course_done: true, ..default() };
        let cart = Achieved { cart_home: true, ..default() };

        assert_eq!(verdict(&Objective::Cull, kills), Some(MissionPhase::Won));
        assert_eq!(verdict(&Objective::Cull, course), None, "a ring does not win a cull");
        assert_eq!(verdict(&Objective::Cull, cart), None);

        assert_eq!(verdict(&parcours(), course), Some(MissionPhase::Won));
        assert_eq!(verdict(&parcours(), kills), None, "a kill does not win a parcours");

        assert_eq!(verdict(&escort(), cart), Some(MissionPhase::Won));
        assert_eq!(verdict(&escort(), kills), None, "a kill does not win an escort");

        // And a breach is won by nothing at all except the clock.
        assert_eq!(verdict(&breach(), kills), None);
        assert_eq!(verdict(&breach(), course), None);
    }

    #[test]
    fn one_titan_at_the_gate_is_one_breach_however_long_he_stands_there() {
        // The `KillTally::credited` trap from the other side: a titan is at the gate on every
        // tick he is at the gate, and a counter would lose the sortie inside a second.
        let mut watch = GateWatch::new(Vec3::ZERO, 5.0, 2);
        assert!(watch.note(TitanId(1)));
        assert!(!watch.note(TitanId(1)), "the same titan a second time is not a second breach");
        assert_eq!(watch.count(), 1);
        assert!(!watch.fallen(), "one of two allowed is not a fall");
        watch.note(TitanId(2));
        assert!(!watch.fallen(), "two of two allowed is still not a fall");
        watch.note(TitanId(3));
        assert!(watch.fallen(), "the third one is one more than allowed");
    }

    #[test]
    fn the_gate_reaches_exactly_as_far_as_the_file_says() {
        let watch = GateWatch::new(Vec3::new(0.0, 0.0, -14.0), 7.0, 3);
        assert!(watch.reaches(Vec3::new(0.0, 0.0, -14.0)));
        assert!(watch.reaches(Vec3::new(0.0, 0.0, -7.0)), "7.0 m out is the edge, and it counts");
        assert!(!watch.reaches(Vec3::new(0.0, 0.0, -6.9)), "7.1 m out is outside");
        // And it is 3D: a titan is 15 m tall and his root is on the ground, but a hook-borne
        // player 40 m over the gate is not standing in it.
        assert!(!watch.reaches(Vec3::new(0.0, 40.0, -14.0)));
    }

    #[test]
    fn a_course_is_flown_in_order_and_an_empty_one_is_never_done() {
        let mut course = Course::new(&[
            Ring { center_m: (0.0, 0.0, 0.0), radius_m: 4.0 },
            Ring { center_m: (30.0, 0.0, 0.0), radius_m: 4.0 },
        ]);
        assert!(!course.done());
        assert_eq!(course.next_gate().map(|(i, _)| i), Some(0));
        assert_eq!(course.pass(), Some(0));
        assert_eq!(course.next_gate().map(|(i, _)| i), Some(1), "the second ring comes second");
        assert!(!course.done(), "one of two rings is not a parcours");
        assert_eq!(course.pass(), Some(1));
        assert!(course.done());
        assert_eq!(course.pass(), None, "there is no third ring to fly");

        // A course nobody wrote must not be won before it starts — the same refusal
        // `KillTally::reached` makes for a target of zero.
        assert!(!Course::new(&[]).done());
    }

    #[test]
    fn a_step_longer_than_a_leg_carries_over_instead_of_stopping_on_the_waypoint() {
        // ⭐ The bug this shape exists to make impossible: a cart that clamps at each waypoint
        // walks one waypoint per tick at most, so a 3 m/s cart on 1 m legs is a 60 m/s cart —
        // and a cart that *stops* on each one never arrives at all.
        let mut haul = Haul::new(
            &[(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (2.0, 0.0, 0.0), (3.0, 0.0, 0.0)],
            10.0,
            8.0,
        );
        haul.roll(2.5);
        assert!((haul.at() - Vec3::new(2.5, 0.0, 0.0)).length() < 1e-4, "at {:?}", haul.at());
        assert!(!haul.arrived());
        haul.roll(0.5);
        assert!(haul.arrived(), "3.0 m of a 3.0 m path is home");
        assert!((haul.at() - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-4);
        // And it stays home: an escorted cart on the last waypoint does not walk off the end.
        haul.roll(100.0);
        assert!((haul.at() - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn a_haul_without_a_path_is_never_home() {
        // Otherwise an `Escort` with an empty `waypoints` list in the file would be won on the
        // first tick and look like the mode works.
        assert!(!Haul::new(&[], 2.0, 8.0).arrived());
        assert!(!Haul::new(&[(0.0, 0.0, 0.0)], 2.0, 8.0).arrived());
    }

    #[test]
    fn the_progress_of_a_mode_is_that_modes_progress_and_no_other() {
        // A `match` that fell through to `Cull` would give a breach no gate to watch, and the
        // sortie would then be decided by a kill target of zero — i.e. never.
        assert_eq!(progress_of(&Objective::Cull), ObjectiveProgress::Cull);
        assert!(matches!(progress_of(&breach()), ObjectiveProgress::Breach(_)));
        assert!(matches!(progress_of(&parcours()), ObjectiveProgress::Parcours(_)));
        assert!(matches!(progress_of(&escort()), ObjectiveProgress::Escort(_)));
    }
}
