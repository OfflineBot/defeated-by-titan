//! The swing state machine — **the blade cuts for eight ticks of twenty-one, not always.**
//!
//! Without `active_from_s`/`active_to_s` the blade cuts during its own wind-up, and a slash
//! stops being a movement and becomes a hitbox that is on whenever the button is down. Without
//! `cooldown_s` it is autofire. All four numbers stand in `assets/data/gear.ron`; what stands
//! here is the arithmetic and the edge.
//!
//! **Seconds in the file, ticks in the code, converted once at the boundary** — the same rule
//! as [`HitStop`](crate::shared::HitStop)'s and `titan::brain::TitanTiming`'s. Nothing in this
//! file ever reads `Time::delta_secs()`: two machines that reach tick *n* must have swung the
//! same number of times (`docs/multiplayer.md` rule 4).

use bevy::prelude::*;

use crate::data::{BladeTuning, GameData};
use crate::shared::{Buttons, HitStop, Intent, MovementState, PlayerId, Side};

/// The swing, **in ticks**, resolved once at spawn.
///
/// Baked onto the player and not looked up per tick: a `GameData` field read is cheap, a
/// second conversion from seconds is not — it is a second truth (`docs/conventions.md`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BladeTiming {
    /// The whole swing, from the button to the arm back at rest.
    pub swing_ticks: u32,
    /// First tick of the swing on which the blade cuts.
    pub active_from_tick: u32,
    /// First tick on which it does **not** cut any more — the window is `from..to`.
    pub active_to_tick: u32,
    /// Ticks after the swing has finished before the next one may start.
    pub cooldown_ticks: u32,
}

impl BladeTiming {
    pub fn of(blades: &BladeTuning, simulation_hz: f64) -> Self {
        BladeTiming {
            swing_ticks: ticks(blades.swing_s, simulation_hz),
            active_from_tick: ticks(blades.active_from_s, simulation_hz),
            active_to_tick: ticks(blades.active_to_s, simulation_hz),
            cooldown_ticks: ticks(blades.cooldown_s, simulation_hz),
        }
    }

    /// How many ticks of one swing actually cut. The number `F-030`'s hit rate hangs on.
    pub fn active_ticks(&self) -> u32 {
        self.active_to_tick.saturating_sub(self.active_from_tick)
    }
}

/// Seconds from the file into ticks. **Rounded, once**, so that 0.35 s at 60 Hz is 21 ticks
/// and not 20 or 22 depending on where the multiplication happened.
///
/// Deliberately the same arithmetic as `titan::brain::ticks` and written out again rather than
/// borrowed: an edge from `blades` to `titan` for four lines of `round()` would be the first
/// entry in an allow list that then never stops growing.
pub fn ticks(seconds: f32, simulation_hz: f64) -> u32 {
    let n = (seconds as f64 * simulation_hz).round();
    if n.is_finite() && n > 0.0 { n as u32 } else { 0 }
}

/// One blade. **A tick accumulator, not a `Timer`.**
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swing {
    /// Ticks already spent in the running swing; `None` means the arm is at rest. The entry
    /// tick of a swing is `Some(0)`, and on that tick the blade does not cut yet.
    pub ticks_in_swing: Option<u32>,
    /// Ticks before the next swing may start.
    pub cooldown_left: u32,
    /// **The swing has found the cortex, and it is over as a cutting thing.** Without this a
    /// blade that is active for eight ticks writes eight `TitanHit`s for one cut, and the
    /// damage of a slash silently depends on how long the target stays inside the capsule.
    pub has_cut: bool,
    /// The swing has already reported a **non-cortex** hit.
    ///
    /// Separate from [`Self::has_cut`] on purpose, and this is not bookkeeping — it is the
    /// difference between a game with a kill in it and one without. Every titan is wider than
    /// his own neck: a husk's body capsule has a radius of 1.25 m
    /// (`width_fraction × height / 2`) and his cortex a radius of 0.55 m *inside* it, so a
    /// blade sweeping in **always meets the body one or more ticks before the nape**. One
    /// single "this swing has landed" flag therefore ends every swing on the shoulder, and the
    /// cortex becomes unreachable on every kind. A blade that grazes the body and then finds
    /// the nape has found the nape.
    pub has_grazed: bool,
}

impl Swing {
    /// Is the blade cutting **this** tick?
    pub fn is_active(&self, timing: &BladeTiming) -> bool {
        match self.ticks_in_swing {
            Some(t) => t >= timing.active_from_tick && t < timing.active_to_tick,
            None => false,
        }
    }

    /// May a fresh swing start? At rest **and** off cooldown.
    pub fn can_start(&self) -> bool {
        self.ticks_in_swing.is_none() && self.cooldown_left == 0
    }

    /// Begins a swing. Does nothing when one is already running or the cooldown is not over —
    /// a caller that has to remember to ask first is a caller that forgets.
    pub fn start(&mut self) {
        if self.can_start() {
            self.ticks_in_swing = Some(0);
            self.has_cut = false;
            self.has_grazed = false;
        }
    }

    /// One tick of the arm. The swing ends **after** `swing_ticks` ticks and then owes the
    /// cooldown.
    pub fn advance(&mut self, timing: &BladeTiming) {
        self.cooldown_left = self.cooldown_left.saturating_sub(1);
        let Some(t) = self.ticks_in_swing else {
            return;
        };
        let next = t.saturating_add(1);
        if next >= timing.swing_ticks {
            self.ticks_in_swing = None;
            self.cooldown_left = timing.cooldown_ticks;
            self.has_cut = false;
            self.has_grazed = false;
        } else {
            self.ticks_in_swing = Some(next);
        }
    }
}

/// Both blades of one player. Two of them, **independently**, exactly like the two hooks.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Swings {
    pub left: Swing,
    pub right: Swing,
}

impl Swings {
    pub fn side(&self, side: Side) -> &Swing {
        match side {
            Side::Left => &self.left,
            Side::Right => &self.right,
        }
    }

    pub fn side_mut(&mut self, side: Side) -> &mut Swing {
        match side {
            Side::Left => &mut self.left,
            Side::Right => &mut self.right,
        }
    }

    /// Which button starts which blade (`src/shared/intent.rs:66-67`).
    pub fn button(side: Side) -> Buttons {
        match side {
            Side::Left => Buttons::SLASH_LEFT,
            Side::Right => Buttons::SLASH_RIGHT,
        }
    }
}

/// Where this tick's blade sweep **starts**: the player's position at the end of the previous
/// tick.
///
/// Not `Velocity * dt`: the velocity is what the body has *after* the step, and the clamp
/// (`F-012`) and every contact of the step sit between the two. The displacement is what
/// really happened, and it is the only thing a swept cast may be swept along.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SweptFrom(pub Vec3);

/// Hangs the blade state on every player that does not have it yet.
///
/// **Not in `player::spawn_player`**, and that is not a workaround: `src/player/` belongs to
/// another domain, and the components a domain writes are the components it may also install.
/// A player without them simply never cuts — no panic, no silent zero.
pub fn equip(
    mut commands: Commands,
    data: Res<GameData>,
    fresh: Query<(Entity, &Transform), (With<PlayerId>, Without<Swings>)>,
) {
    let timing = BladeTiming::of(&data.gear.blades, data.game.simulation_hz);
    for (entity, transform) in &fresh {
        commands.entity(entity).insert((
            Swings::default(),
            timing,
            // Seeded with the current position, so that the first sweep is one tick long and
            // not a 400 m line from the world origin to the player.
            SweptFrom(transform.translation),
        ));
    }
}

/// One tick of both blades: count the arm on, then decide whether a new swing starts.
///
/// **Advance first, start second.** The other way round a swing that begins this tick would
/// already be one tick old, and `active_from_tick` would be off by one for every cut in the
/// game.
pub fn advance(
    mut players: Query<(
        &Intent,
        &MovementState,
        &BladeTiming,
        &mut Swings,
        Option<&HitStop>,
    )>,
) {
    for (intent, state, timing, mut swings, stop) in &mut players {
        // **The hit stop freezes the arm too** (`F-034`). Without this the blade keeps
        // cutting through the freeze and one slash lands seven more times while the world
        // stands still — which is the impact frame turning into a damage multiplier.
        if stop.is_some_and(HitStop::is_frozen) {
            continue;
        }
        for side in Side::ALL {
            let swing = swings.side_mut(side);
            swing.advance(timing);
            // Out of the fight is out of the fight. A `Downed` player is a state with a
            // timer, not a removed entity (`shared::MovementState`) — so he is still here,
            // still queried, and must not swing.
            if *state == MovementState::Downed {
                continue;
            }
            // `pressed`, not `just_pressed`: `cooldown_s` is what stops autofire
            // (`gear.ron`), and holding the button is how a player cuts twice in a pass.
            if intent.pressed(Swings::button(side)) {
                swing.start();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing() -> BladeTiming {
        // The numbers gear.ron gives at 60 Hz: 0.35 / 0.08 / 0.22 / 0.30 s.
        BladeTiming {
            swing_ticks: 21,
            active_from_tick: 5,
            active_to_tick: 13,
            cooldown_ticks: 18,
        }
    }

    #[test]
    fn seconds_become_ticks_exactly_once() {
        assert_eq!(ticks(0.35, 60.0), 21);
        assert_eq!(ticks(0.08, 60.0), 5); // 4.8 rounds up
        assert_eq!(ticks(0.22, 60.0), 13); // 13.2 rounds down
        assert_eq!(ticks(0.30, 60.0), 18);
        // No negative and no NaN duration ever becomes a huge u32.
        assert_eq!(ticks(-1.0, 60.0), 0);
        assert_eq!(ticks(f32::NAN, 60.0), 0);
    }

    #[test]
    fn the_blade_does_not_cut_during_its_own_windup() {
        // The failure this window exists for: an "active whenever the button is down" blade.
        let t = timing();
        let mut s = Swing::default();
        s.start();
        for tick in 0..t.swing_ticks {
            let wanted = tick >= t.active_from_tick && tick < t.active_to_tick;
            assert_eq!(
                s.is_active(&t),
                wanted,
                "tick {tick} of the swing: active = {}, expected {wanted}",
                s.is_active(&t)
            );
            s.advance(&t);
        }
        assert_eq!(s.ticks_in_swing, None, "the swing did not end after {} ticks", t.swing_ticks);
        assert_eq!(s.cooldown_left, t.cooldown_ticks);
        assert_eq!(t.active_ticks(), 8, "gear.ron gives 8 cutting ticks out of 21");
    }

    #[test]
    fn a_held_button_is_not_autofire() {
        // Holding SLASH must produce one swing per `swing_s + cooldown_s`, not one per tick.
        let t = timing();
        let mut s = Swing::default();
        let mut swings = 0;
        for _ in 0..(t.swing_ticks + t.cooldown_ticks) * 3 {
            s.advance(&t);
            if s.can_start() {
                s.start();
                swings += 1;
            }
        }
        assert_eq!(
            swings, 3,
            "a held button produced {swings} swings in three full swing+cooldown windows"
        );
    }

    #[test]
    fn a_second_press_during_a_swing_changes_nothing() {
        let t = timing();
        let mut s = Swing::default();
        s.start();
        s.advance(&t);
        let before = s;
        s.start();
        assert_eq!(s, before, "a press during a running swing restarted it");
    }

    #[test]
    fn one_swing_lands_once_per_zone() {
        // Eight active ticks must not become eight hits — and the graze must not eat the cut.
        let t = timing();
        let mut s = Swing::default();
        s.start();
        for _ in 0..t.active_from_tick {
            s.advance(&t);
        }
        assert!(s.is_active(&t) && !s.has_cut && !s.has_grazed);
        // The body is met first: every titan is wider than his own neck.
        s.has_grazed = true;
        for _ in 0..t.active_ticks() {
            s.advance(&t);
            if s.ticks_in_swing.is_some() {
                assert!(s.has_grazed, "the graze flag was lost inside the same swing");
                assert!(
                    !s.has_cut,
                    "a graze closed the swing — the cortex is now unreachable on every titan \
                     whose body is wider than his neck, which is all of them"
                );
            }
        }
        // And the next swing starts clean. Run the arm to the end of the swing first — a
        // running swing cannot be restarted, which is its own assertion above.
        while s.ticks_in_swing.is_some() {
            s.advance(&t);
        }
        s.cooldown_left = 0;
        s.start();
        assert!(!s.has_cut && !s.has_grazed, "the next swing inherited the previous swing's hits");
    }
}
