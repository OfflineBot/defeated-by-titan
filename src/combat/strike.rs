//! `P5`, second half — **a titan's `Strike` takes health off the player it reaches.**
//!
//! Until this file existed a titan could wind up, strike, land — and nothing happened. That is
//! the second way to lose missing, and with it the whole reason a telegraphed wind-up is worth
//! reading: if tanking a blow costs nothing, `recover_s` is not a punish window, it is
//! decoration.
//!
//! ## The two numbers, and where they come from
//!
//! `titan.ron: <kind>.damage` and `<kind>.attack_range_m`, out of [`GameData`], never a
//! literal. They are calibrated against `game.ron: player.health`: **100 against a husk's 34 is
//! three strikes and you are down** — `tests/combat.rs::p5_a_husk_needs_exactly_three_strikes`
//! computes that quotient out of the two files rather than hard-coding a 3, and
//! `p5_the_damage_comes_out_of_the_file_and_not_out_of_rust` changes the number and watches the
//! count change with it.
//!
//! ## Once per strike, never once per tick
//!
//! A husk's `strike_s` is 0.2 s = **12 ticks**. A system that subtracts while
//! `TitanState::Strike` holds subtracts twelve times, the player dies inside the first blow,
//! and the wind-up he was supposed to read never mattered. So the strike is **booked**: the
//! first tick combat sees a titan in `Strike` it applies the damage and marks the titan with
//! [`StrikeSpent`]; the mark comes off again when the titan leaves `Strike`, and the FSM cannot
//! go `Strike → Strike` (`titan::brain::decide`: the only edge out of `Strike` is `Recover`).
//! `tests/combat.rs::p5_one_strike_subtracts_once_and_not_once_per_tick` counts over the whole
//! 12 ticks.
//!
//! ## Why the reach is measured from the titan's feet and not from his hand
//!
//! Because the hand is in `titan/` and `combat` may not know how a titan is built
//! (`docs/architecture.md`) — but that is not the real reason, it is only the one that decides
//! it. The real reason is that **`titan::brain` commits to the attack on the ground distance**:
//! `Pursue → Windup` fires at `distance_m <= attack_range_m`, measured on the ground plane from
//! the body's origin. Any *other* measure here would disagree with the one that started the
//! attack, and a titan that commits to a blow he then cannot land is a titan that whiffs
//! forever, for a reason nobody can see on screen.
//!
//! The one thing the ground plane does not answer is *up*: a player 60 m over the titan's head
//! is 5 m away on the ground and must not be hit. So the reach carries a ceiling as well —
//! `scale.ron: titan.shoulder_height_fraction × height`, the height the hand hangs from. It is
//! read out of `data/`, which every domain may read, so it costs no edge.
//!
//! ⚠️ **Finding, not a decision of this file:** the ceiling is the shoulder because the hand's
//! real position is `titan::rig`'s. A `TitanPart::Hand` with a `GlobalTransform`, or a strike
//! volume written by `titan/` into a message, is the better answer and belongs to whoever owns
//! `src/titan/`.
//!
//! ## How this file knows which kind a titan is
//!
//! It reads the entity's [`Name`], which `titan::rig::build_rig` writes as
//! `titan_<kind>_<id>`, and looks the kind up in `titan.ron`. **That is a string contract
//! between two domains and nobody wrote it down** — there is no kind on a titan anywhere in
//! `shared/`, and `combat` may not read `titan/`. It is guarded by
//! `p5_a_husk_needs_exactly_three_strikes`, which runs against the **real** husk and goes red
//! the moment the name changes. The proper fix is a component in `shared/` carrying the kind
//! key; that file is not this job's, so the hole is reported rather than papered over.

use bevy::prelude::*;

use crate::data::{GameData, TitanKind};
use crate::shared::{Health, PlayerId, SimulationSystems, TitanId, TitanState};

/// What one titan's blow costs and how far it carries. **Resolved once**, at the first tick the
/// titan is seen, out of `titan.ron` and `scale.ron`.
///
/// Baked onto the entity and not looked up per tick, for the same reason `titan::brain` bakes
/// its timings: a `BTreeMap<String, _>` lookup per titan per tick costs nothing at three titans
/// and shows up at sixty.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StrikeTuning {
    /// `titan.ron: <kind>.damage` — what one landed strike takes off a player.
    pub damage: f32,
    /// `titan.ron: <kind>.attack_range_m`, on the ground plane. **The same number
    /// `titan::brain` commits to the attack on.**
    pub reach_m: f32,
    /// How high over the titan's own origin the blow still carries: the shoulder.
    pub top_m: f32,
}

impl StrikeTuning {
    /// The two numbers of one kind, plus the shoulder out of the scale table.
    ///
    /// `None` means the kind's size class is not in `scale.ron` — the caller says so loudly
    /// instead of building a strike with a reach of 0.
    pub fn of(kind: &TitanKind, data: &GameData) -> Option<Self> {
        let height_m = data.titan_height_m(kind)?;
        Some(StrikeTuning {
            damage: kind.damage,
            reach_m: kind.attack_range_m,
            top_m: data.scale.titan.shoulder_height_fraction * height_m,
        })
    }

    /// Whether a blow from a titan standing at `titan_m` reaches a player at `player_m`.
    ///
    /// Ground distance against [`reach_m`](Self::reach_m), height against
    /// [`top_m`](Self::top_m). Downwards the reach is `reach_m` as well: a player in a hole at
    /// the titan's feet is inside the arm's swing, a player on a roof over his head is not.
    pub fn reaches(&self, titan_m: Vec3, player_m: Vec3) -> bool {
        let to = player_m - titan_m;
        let ground_m = Vec3::new(to.x, 0.0, to.z).length();
        ground_m <= self.reach_m && to.y <= self.top_m && to.y >= -self.reach_m
    }
}

/// This titan's current strike has already been paid out. Comes off when he leaves `Strike`.
///
/// A marker and not a counter: what has to be true is "at most one subtraction per strike", and
/// a `u32` that is compared against a length would be a second place where `strike_s` lives.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StrikeSpent;

/// The kind of a titan out of its [`Name`] — see the module header for why it is read from
/// there and why that is reported as a hole.
///
/// Matches on the whole key plus the separator, so that a hypothetical `"husk_elite"` cannot be
/// answered with `"husk"`.
pub fn kind_of<'a>(name: &str, data: &'a GameData) -> Option<(&'a str, &'a TitanKind)> {
    let rest = name.strip_prefix("titan_")?;
    data.titans
        .kinds
        .iter()
        .find(|(key, _)| {
            rest.strip_prefix(key.as_str()).is_some_and(|tail| tail.starts_with('_'))
        })
        .map(|(key, kind)| (key.as_str(), kind))
}

/// Hangs [`StrikeTuning`] on every titan that does not have it yet.
///
/// The same shape as `blades::swing::equip`: the components a domain writes are the components
/// it may also install, and `src/titan/` belongs to another domain.
///
/// A body that carries a [`TitanId`] but no name this file can resolve gets a tuning of zero
/// **and one warning** — test fixtures do exactly that, and a silent zero would be
/// indistinguishable from a titan whose damage is missing from the file.
pub fn resolve_tuning(
    mut commands: Commands,
    data: Res<GameData>,
    fresh: Query<(Entity, &Name), (With<TitanId>, Without<StrikeTuning>)>,
) {
    for (entity, name) in &fresh {
        let tuning = kind_of(name.as_str(), &data)
            .and_then(|(_, kind)| StrikeTuning::of(kind, &data))
            .unwrap_or_else(|| {
                warn!(
                    "combat: entity {:?} carries a TitanId but no kind of titan.ron can be read \
                     off its name — its strikes take nothing. titan::rig names a titan \
                     `titan_<kind>_<id>`; anything else is a fixture or a renamed rig \
                     (src/combat/strike.rs).",
                    name.as_str()
                );
                StrikeTuning { damage: 0.0, reach_m: 0.0, top_m: 0.0 }
            });
        commands.entity(entity).insert(tuning);
    }
}

/// One booking per `Strike`: every player the blow reaches loses `damage`.
///
/// Runs in [`SimulationSystems::PostStep`], **after** `titan::brain::advance` in `Drive` has
/// decided this tick's state. In `Drive` the two systems would be unordered — the set has no
/// `.chain()` on purpose — and whether a blow lands in tick *n* or *n+1* would depend on the
/// order the scheduler happened to pick.
///
/// `Without` on both sides although nothing is a player and a titan at once: two queries that
/// touch the same component are only disjoint if the filters say so, and this file's neighbour
/// [`super::hitstop`] once cost the whole repository every app-building test over exactly that
/// (`B0001`).
pub fn land(
    mut commands: Commands,
    titans: Query<
        (Entity, &TitanState, &Transform, &StrikeTuning, Has<StrikeSpent>),
        (With<TitanId>, Without<PlayerId>),
    >,
    mut players: Query<(&PlayerId, &Transform, &mut Health), Without<TitanId>>,
) {
    for (entity, state, at, tuning, spent) in &titans {
        if *state != TitanState::Strike {
            // The blow is over. The next `Strike` is a new one and gets paid out again.
            if spent {
                commands.entity(entity).remove::<StrikeSpent>();
            }
            continue;
        }
        if spent {
            continue;
        }
        commands.entity(entity).insert(StrikeSpent);
        if tuning.damage <= 0.0 {
            continue;
        }
        for (id, player_at, mut health) in &mut players {
            if !tuning.reaches(at.translation, player_at.translation) {
                continue;
            }
            let left = health.damage(tuning.damage);
            info!(
                "strike: player {} takes {:.1} — {:.1}/{:.1} left",
                id.0, tuning.damage, left, health.max
            );
        }
    }
}

/// Registered from [`super::CombatPlugin`].
///
/// `.chain()`, because the tuning of a titan that appeared this tick has to be on the entity
/// before [`land`] reads it — Bevy's automatic sync point between two chained systems is what
/// makes that true in the same tick (the same reasoning as `blades::swing::equip`).
pub fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (resolve_tuning, land).chain().in_set(SimulationSystems::PostStep),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> StrikeTuning {
        // A husk: 34 damage, 6 m of reach, 10 m tall with the shoulder at 0.82 of that.
        StrikeTuning { damage: 34.0, reach_m: 6.0, top_m: 8.2 }
    }

    #[test]
    fn the_reach_is_a_ground_distance_with_a_ceiling() {
        let t = tuning();
        let titan = Vec3::new(10.0, 0.0, -20.0);
        assert!(t.reaches(titan, titan + Vec3::new(5.9, 1.0, 0.0)), "inside the arm");
        assert!(!t.reaches(titan, titan + Vec3::new(6.1, 1.0, 0.0)), "outside attack_range_m");
        // The one the ground plane alone gets wrong: 60 m straight up is 0 m away on the
        // ground, and a titan must not hit a player who is flying over him.
        assert!(!t.reaches(titan, titan + Vec3::new(0.0, 60.0, 0.0)), "over the shoulder");
        assert!(t.reaches(titan, titan + Vec3::new(0.0, 8.0, 0.0)), "at shoulder height");
        // Height is measured from the titan's own origin, not from the world's.
        assert!(t.reaches(titan + Vec3::Y * 30.0, titan + Vec3::new(1.0, 31.0, 0.0)));
    }

    #[test]
    fn the_kind_is_read_off_the_rig_name_and_nothing_else_is() {
        // The string contract with `titan::rig`. It is guarded here AND against the real husk
        // in tests/combat.rs — this half only says what the parser does.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
        let data = GameData::load(&dir);
        assert_eq!(kind_of("titan_husk_1", &data).map(|(k, _)| k), Some("husk"));
        assert_eq!(kind_of("titan_bellower_42", &data).map(|(k, _)| k), Some("bellower"));
        assert!(kind_of("fixture_titan", &data).is_none(), "a fixture is not a kind");
        assert!(kind_of("titan_husk", &data).is_none(), "no id, no match — the separator counts");
        assert!(kind_of("titan_hus_1", &data).is_none());
    }
}
