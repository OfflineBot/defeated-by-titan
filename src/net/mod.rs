//! net — **the seam for multiplayer.** One transport today, client and server later.
//!
//! The net code is not part of this commission. **But the place it will one day stand in
//! exists from day 1, and it is empty** — instead of cutting through five domains later
//! (`prompts/init.md` §6, `docs/multiplayer.md`).
//!
//! ```text
//! Keyboard ─┐
//! Script   ─┼─► Inbox ─► deliver_intents ─► Intent on the player ─► Simulation
//! (net)    ─┘   (PlayerId → Intent)   FixedPreUpdate
//! ```
//!
//! **Three sources, one channel.** The script driver is not a second, wrong way to play —
//! every system behind it is the real one. And because nobody in this environment can
//! click, this channel gets built anyway: **one effort, two problems solved.**
//!
//! The **latency switch** (`--lag 200`) lives here too. It belongs in the tooling and not
//! in some later ticket: "feels good locally" is not an acceptance (bible T-019).

pub mod local;

use bevy::prelude::*;
use std::collections::{BTreeMap, VecDeque};

use crate::shared::{IntentSystems, Intent, PlayerId, Cli, Tick};

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        let start = app
            .world()
            .get_resource::<Cli>()
            .cloned()
            .unwrap_or_default();

        // A fixed 60 Hz simulation -> one tick is 16.67 ms. Rounded up so that `--lag 200`
        // never simulates LESS than 200 ms: too little latency in a test is the more
        // dangerous direction.
        let lag_ticks = (start.lag_ms as f64 / 1000.0 * 60.0).ceil() as u64;

        app.insert_resource(Transport::LocalOnly)
            .insert_resource(Inbox::with_lag(lag_ticks))
            .init_resource::<crate::shared::LookOverride>()
            .init_resource::<local::MouseSinceTick>()
            .configure_sets(
                FixedPreUpdate,
                (IntentSystems::Source, IntentSystems::Collect, IntentSystems::Deliver).chain(),
            )
            // **Per frame, not per tick** — and that is the entire point of this line.
            // `AccumulatedMouseMotion` is refreshed once per frame; `FixedPreUpdate` runs
            // 0..n times in one. Whoever moves this system into a fixed schedule brings
            // `B-002` back, and `tests/input.rs` says so with a number.
            .add_systems(
                RunFixedMainLoop,
                local::gather_mouse_motion.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(FixedPreUpdate, local::read_input.in_set(IntentSystems::Collect))
            .add_systems(
                FixedPreUpdate,
                (advance_tick, deliver_intents)
                    .chain()
                    .in_set(IntentSystems::Deliver),
            );
    }
}

/// Where the intents come from.
///
/// Today there is exactly one variant. It stands there as an enum anyway, because
/// otherwise the day the second one arrives is the day somebody rebuilds `net` instead of
/// extending it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transport {
    #[default]
    LocalOnly,
}

/// The one channel. **Nobody writes an `Intent` straight onto a player** — everybody posts
/// here, and [`deliver_intents`] delivers.
#[derive(Resource, Debug, Default)]
pub struct Inbox {
    /// How many ticks a message stays put before it is delivered (`--lag`).
    pub lag_ticks: u64,
    queue: VecDeque<(u64, PlayerId, Intent)>,
    /// The last intent delivered per player. Kept so that a player without a new message
    /// does not stop dead — over a network that is exactly the stutter people see as
    /// "lag".
    last: BTreeMap<PlayerId, Intent>,
}

impl Inbox {
    /// An inbox with a fixed latency. The only way to set `lag_ticks` from outside — the
    /// queue belongs to this domain and to nobody else.
    pub fn with_lag(lag_ticks: u64) -> Self {
        Inbox { lag_ticks, ..default() }
    }

    /// Post an intent. Delivery is allowed from `current + lag_ticks` on.
    pub fn push(&mut self, player: PlayerId, intent: Intent, current: u64) {
        self.queue
            .push_back((current + self.lag_ticks, player, intent));
    }

    /// Take everything that is due. The order is preserved (FIFO per player).
    pub fn drain_due(&mut self, current: u64) -> Vec<(PlayerId, Intent)> {
        let mut done = Vec::new();
        while let Some(&(due, _, _)) = self.queue.front() {
            if due > current {
                break;
            }
            let (_, player, intent) = self.queue.pop_front().expect(
                // Reason: `front()` just returned Some, and nobody else holds a reference
                // here.
                "front() returned Some",
            );
            self.last.insert(player, intent);
            done.push((player, intent));
        }
        done
    }

    pub fn last(&self, player: PlayerId) -> Option<Intent> {
        self.last.get(&player).copied()
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

/// The tick counts up in `FixedPreUpdate` — **before** everything that reads it.
fn advance_tick(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}

/// Delivers due intents to the players.
///
/// By [`PlayerId`], not by `Entity`: an `Entity` means something different on another
/// machine (§6 rule 7).
fn deliver_intents(
    tick: Res<Tick>,
    mut inbox: ResMut<Inbox>,
    mut players: Query<(&PlayerId, &mut Intent)>,
) {
    let due = inbox.drain_due(tick.0);
    if due.is_empty() {
        return;
    }
    for (id, mut intent) in &mut players {
        if let Some((_, new_intent)) = due.iter().rev().find(|(w, _)| w == id) {
            *intent = *new_intent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_lag_delivery_is_immediate() {
        let mut p = Inbox::default();
        p.push(PlayerId(1), Intent { tick: 5, ..default() }, 5);
        let out = p.drain_due(5);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, PlayerId(1));
    }

    #[test]
    fn lag_holds_back_exactly_that_many_ticks() {
        // 200 ms at 60 Hz are 12 ticks — that is the number every movement feature is
        // checked against (bible T-019).
        let mut p = Inbox::with_lag(12);
        p.push(PlayerId(1), Intent::default(), 100);
        for current in 100..112 {
            assert!(p.drain_due(current).is_empty(), "not due yet at tick {current}");
        }
        assert_eq!(p.drain_due(112).len(), 1);
    }

    #[test]
    fn several_players_get_their_own_mail() {
        // There is no such thing as "the player" (§6 rule 3).
        let mut p = Inbox::default();
        p.push(PlayerId(1), Intent { yaw: 1.0, ..default() }, 0);
        p.push(PlayerId(2), Intent { yaw: 2.0, ..default() }, 0);
        let out = p.drain_due(0);
        assert_eq!(out.len(), 2);
        assert_eq!(p.last(PlayerId(1)).map(|i| i.yaw), Some(1.0));
        assert_eq!(p.last(PlayerId(2)).map(|i| i.yaw), Some(2.0));
    }

    #[test]
    fn order_is_preserved() {
        let mut p = Inbox::default();
        for t in 0..5u64 {
            p.push(PlayerId(1), Intent { tick: t, ..default() }, 0);
        }
        let ticks: Vec<u64> = p.drain_due(0).into_iter().map(|(_, i)| i.tick).collect();
        assert_eq!(ticks, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn nothing_is_lost_if_nobody_collects() {
        let mut p = Inbox::with_lag(3);
        p.push(PlayerId(1), Intent::default(), 0);
        p.push(PlayerId(1), Intent::default(), 1);
        assert_eq!(p.pending(), 2);
        assert_eq!(p.drain_due(10).len(), 2, "draining later collects both");
        assert_eq!(p.pending(), 0);
    }
}
