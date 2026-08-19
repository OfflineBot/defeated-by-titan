//! **Who is here** — the session, and the seat that outlives the connection.
//!
//! A [`Seat`] is not a player and not a body. It is a *chair*: it is created the moment a
//! peer's first packet arrives — before anything exists in the world — and it stays his for
//! two minutes after his line drops (bible F-158a, `game.ron: net.slot_hold_s`). That is the
//! whole of rule 7 in one type: *his state hangs on a `PlayerId`, not on a connection*
//! (`docs/multiplayer.md`).
//!
//! ## Why a `Resource` here is not a breach of rule 3
//!
//! Rule 3 forbids **player state** in a resource — gas, intent, velocity, health. None of
//! that is here. What is here is *membership*: which ids are in this session, what to call
//! them, and when each was last heard from. That is session state, exactly like
//! `menu::Screen` is screen state, and the body it points at carries everything else.
//! `tests/multiplayer.rs` is what keeps the line where it is.
//!
//! ⚠️ **One writer: `net`.** `menu` reads this to draw the lobby's squad list and writes
//! nothing (`docs/architecture.md`, allow list).

use bevy::prelude::*;
use std::collections::BTreeMap;
use std::net::SocketAddr;

use crate::shared::PlayerId;

/// Where a seat's intents come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeatKind {
    /// This machine's keyboard, or the `--script` driver standing in for it.
    Local,
    /// A UDP peer. The address is where his frames come from and where nothing is sent back
    /// yet — see `net::socket`.
    Remote(SocketAddr),
}

impl SeatKind {
    pub fn is_local(self) -> bool {
        self == SeatKind::Local
    }
}

/// One chair in the session.
#[derive(Clone, Debug)]
pub struct Seat {
    pub kind: SeatKind,
    /// What the lobby calls him. Not an identity — [`PlayerId`] is the identity.
    pub name: String,
    /// The tick his last frame arrived on. A local seat is refreshed every tick by definition.
    pub last_heard_tick: u64,
    /// Since when the line has been quiet, or `None` while he is connected. The seat is his
    /// until `slot_hold_ticks` after this.
    pub quiet_since: Option<u64>,
}

impl Seat {
    /// Whether he is answering right now. A disconnected seat is still **his** — it is drawn
    /// greyed out, not removed.
    pub fn connected(&self) -> bool {
        self.quiet_since.is_none()
    }
}

/// Everybody in this session, in id order.
///
/// `BTreeMap` and not a `HashMap`: the lobby draws this list and two machines have to draw it
/// in the same order. A hash order is a different order per run and per build.
#[derive(Resource, Debug, Default)]
pub struct Roster {
    seats: BTreeMap<PlayerId, Seat>,
}

impl Roster {
    /// Sit somebody down, or move him back into the chair he already had.
    ///
    /// **Returns whether this is a new seat** — the caller needs to know, because a new seat
    /// needs a body and a returning one already has his (`shared::SeatPlayer`).
    pub fn seat(&mut self, id: PlayerId, kind: SeatKind, name: String, tick: u64) -> bool {
        match self.seats.get_mut(&id) {
            Some(seat) => {
                // A reconnect inside the hold window. The kind is refreshed because the port
                // he comes back on is not the port he left from.
                seat.kind = kind;
                seat.last_heard_tick = tick;
                seat.quiet_since = None;
                false
            }
            None => {
                self.seats.insert(
                    id,
                    Seat { kind, name, last_heard_tick: tick, quiet_since: None },
                );
                true
            }
        }
    }

    /// A frame arrived from him.
    pub fn heard(&mut self, id: PlayerId, tick: u64) {
        if let Some(seat) = self.seats.get_mut(&id) {
            seat.last_heard_tick = tick;
            seat.quiet_since = None;
        }
    }

    /// Marks everybody who has gone quiet, and frees the seats whose hold has run out.
    ///
    /// **Two timers and not one, and that is the bible's own shape** (F-158a): silence makes
    /// a seat *disconnected* after `timeout_ticks` — the lobby greys him out and the world
    /// keeps his body — and only `hold_ticks` later does the chair become free again.
    ///
    /// A local seat is never swept: there is no line to it that can drop.
    ///
    /// Returns the ids whose seats were freed, so the caller can take their bodies out.
    pub fn sweep(&mut self, tick: u64, timeout_ticks: u64, hold_ticks: u64) -> Vec<PlayerId> {
        let mut freed = Vec::new();
        for (id, seat) in &mut self.seats {
            if seat.kind.is_local() {
                continue;
            }
            match seat.quiet_since {
                None => {
                    if tick.saturating_sub(seat.last_heard_tick) > timeout_ticks {
                        info!("net: player {} went quiet — his seat is held", id.0);
                        seat.quiet_since = Some(tick);
                    }
                }
                Some(since) => {
                    if tick.saturating_sub(since) > hold_ticks {
                        freed.push(*id);
                    }
                }
            }
        }
        for id in &freed {
            info!("net: player {}'s slot hold ran out — the chair is free", id.0);
            self.seats.remove(id);
        }
        freed
    }

    /// Drop everybody who is not sitting at this keyboard — closing the door.
    pub fn clear_remote(&mut self) -> Vec<PlayerId> {
        let going: Vec<PlayerId> = self
            .seats
            .iter()
            .filter(|(_, seat)| !seat.kind.is_local())
            .map(|(id, _)| *id)
            .collect();
        for id in &going {
            self.seats.remove(id);
        }
        going
    }

    pub fn get(&self, id: PlayerId) -> Option<&Seat> {
        self.seats.get(&id)
    }

    pub fn contains(&self, id: PlayerId) -> bool {
        self.seats.contains_key(&id)
    }

    /// Everybody, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (PlayerId, &Seat)> {
        self.seats.iter().map(|(id, seat)| (*id, seat))
    }

    pub fn len(&self) -> usize {
        self.seats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }
}

/// Puts the local player in the roster once he exists.
///
/// **Not a second writer of anything.** `player` spawns the body — at `Startup`, and two
/// `Startup` systems have no ordering between them, which is exactly why this is a running
/// system and not a startup one. It notices the body on the first frame it is there and never
/// writes it.
pub fn seat_the_local_player(
    mut roster: ResMut<Roster>,
    tick: Res<crate::shared::Tick>,
    local: Query<&PlayerId, With<crate::shared::LocalPlayer>>,
) {
    for id in &local {
        // ⚠️ **`contains` before `seat`, and it is not a micro-optimisation.** `seat` takes
        // `&mut self`, and a `DerefMut` on a resource marks it changed **for every reader** —
        // this system runs every frame, and `menu::despawn_menu` rebuilds the lobby plate on a
        // changed roster. Without this line the squad list would be torn down and rebuilt
        // sixty times a second (§6 rule 6, and the same trap `read_input` documents for
        // `PlayerSettings`).
        if !roster.contains(*id) {
            roster.seat(*id, SeatKind::Local, "you".to_string(), tick.0);
            info!("net: seat {} is this machine", id.0);
        }
    }
}

/// **Once per second, and only while the door is open: where everybody is.**
///
/// This is the one thing a `--headless` run can show about a session, and it is why it exists:
/// a peer drives a body in this process and has no screen to look at, so the host's log is the
/// only place two players can be *seen* being two players. It prints the id, the seat and the
/// position of every body in the world.
///
/// **Per second and not per frame** (§6 rule 6, `docs/lessons/performance.md`). The interval
/// comes out of `game.ron: simulation_hz`, so it stays one second if the tick rate ever moves.
pub fn report_the_squad(
    roster: Res<Roster>,
    host: Res<crate::net::Host>,
    tick: Res<crate::shared::Tick>,
    data: Res<crate::data::GameData>,
    players: Query<(&PlayerId, &Transform)>,
) {
    let every = data.game.simulation_hz.round() as u64;
    if !host.is_open() || every == 0 || tick.0 % every != 0 {
        return;
    }
    for (id, at) in &players {
        let seat = roster.get(*id);
        let who = seat.map_or("no seat", |s| s.name.as_str());
        let state = match seat {
            Some(s) if !s.connected() => " quiet",
            _ => "",
        };
        info!(
            "net: tick {} player {} [{who}{state}] at {:.1} {:.1} {:.1}",
            tick.0, id.0, at.translation.x, at.translation.y, at.translation.z
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr() -> SocketAddr {
        "127.0.0.1:40000".parse().expect("a literal address")
    }

    #[test]
    fn session_a_new_peer_gets_a_new_seat_and_a_returning_one_does_not() {
        let mut roster = Roster::default();
        assert!(roster.seat(PlayerId(2), SeatKind::Remote(addr()), "peer".into(), 10));
        assert!(
            !roster.seat(PlayerId(2), SeatKind::Remote(addr()), "peer".into(), 20),
            "the same id twice must not ask for a second body"
        );
        assert_eq!(roster.len(), 1);
    }

    #[test]
    fn f158a_a_dropped_connection_holds_the_slot_and_only_then_frees_it() {
        // The bible's 120 s at 60 Hz are 7200 ticks; the timeout is 5 s, i.e. 300.
        let (timeout, hold) = (300u64, 7200u64);
        let mut roster = Roster::default();
        roster.seat(PlayerId(2), SeatKind::Remote(addr()), "peer".into(), 0);

        assert!(roster.sweep(300, timeout, hold).is_empty(), "300 ticks of silence is the edge");
        assert!(roster.get(PlayerId(2)).expect("seated").connected());

        assert!(roster.sweep(301, timeout, hold).is_empty(), "quiet is not gone");
        assert!(!roster.get(PlayerId(2)).expect("still seated").connected());
        assert!(roster.contains(PlayerId(2)), "the chair is still his");

        // He is back inside the window: same chair, same id, no second body.
        assert!(!roster.seat(PlayerId(2), SeatKind::Remote(addr()), "peer".into(), 4000));
        assert!(roster.get(PlayerId(2)).expect("seated").connected());

        // And this time he does not come back.
        assert!(roster.sweep(4301, timeout, hold).is_empty());
        assert!(roster.sweep(4301 + hold, timeout, hold).is_empty(), "the hold is inclusive");
        assert_eq!(
            roster.sweep(4302 + hold, timeout, hold),
            vec![PlayerId(2)],
            "after 120 s of silence the chair is free"
        );
        assert!(roster.is_empty());
    }

    #[test]
    fn session_the_local_seat_is_never_swept() {
        // There is no line to this machine that can drop, and a sweep that took the local
        // player's chair away would delete the person playing.
        let mut roster = Roster::default();
        roster.seat(PlayerId(1), SeatKind::Local, "you".into(), 0);
        assert!(roster.sweep(1_000_000, 300, 7200).is_empty());
        assert!(roster.get(PlayerId(1)).expect("still here").connected());
        assert!(roster.clear_remote().is_empty(), "closing the door keeps this machine seated");
        assert_eq!(roster.len(), 1);
    }

    #[test]
    fn session_the_list_is_in_id_order_on_every_run() {
        // The lobby draws this list, and two machines have to draw it the same way.
        let mut roster = Roster::default();
        for id in [7u32, 2, 5, 1] {
            roster.seat(PlayerId(id), SeatKind::Remote(addr()), format!("p{id}"), 0);
        }
        let order: Vec<u32> = roster.iter().map(|(id, _)| id.0).collect();
        assert_eq!(order, vec![1, 2, 5, 7]);
    }
}
