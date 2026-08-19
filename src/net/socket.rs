//! **The second transport: a UDP socket.** One direction, input only.
//!
//! `LocalOnly` was the seam with one implementation, which is another way of saying it had
//! never been tried. This is the second one, and it is deliberately the smallest honest step
//! there is — so that what it does and what it does not do can both be said in one paragraph
//! each.
//!
//! ## What it does
//!
//! Opens a UDP port. Every datagram that arrives is a [`wire::Frame`] — 37 bytes, one
//! player's wish for one tick. The **first** frame from an address opens a [`Seat`] and asks
//! `player` for a body (`shared::SeatPlayer`); every frame after that is pushed into the same
//! [`Inbox`](super::Inbox) the keyboard and the script driver write into, and from there the
//! simulation cannot tell the three apart. That is the seam paying for itself: no system
//! behind `deliver_intents` was touched to make a player arrive over a network.
//!
//! ## What it is NOT — read this before calling anything here multiplayer
//!
//! - **Nothing is sent back.** There is no state replication, no snapshot, no interpolation.
//!   A peer drives a body in *this* process and cannot see it. Two copies of this game do not
//!   yet make a co-op session; a sender and a host make **one** world with two players in it.
//! - **No reliability, no ordering, no reconnect handshake.** UDP delivers what it delivers.
//!   That is survivable *because* an [`Intent`](crate::shared::Intent) is absolute and
//!   idempotent — a lost frame costs one tick and the next one repairs it — and it is exactly
//!   why the aim spread travels as an angle and not as a wheel notch
//!   (`tests/multiplayer.rs::f023_a_dropped_packet_does_not_desync_the_aim_spread`).
//! - **No authentication and no encryption.** A datagram carries a `PlayerId` and this module
//!   **throws it away**: the seat belongs to the address the packet came from, so a peer
//!   cannot claim to be somebody else. That is the only security property here, it is one
//!   line, and it is not a substitute for the rest.
//! - **Not deterministic across machines.** The wire carries `Intent` and the simulation runs
//!   in `FixedUpdate`, so the *ingredients* of a reproducible run are all here — but a UDP
//!   frame arrives on whichever tick it arrives on, and nothing here delays it to a fixed one.
//!   **A run over this transport cannot be reproduced from the frames alone.** `--lag` is
//!   deterministic; the socket is not. Saying otherwise would be the expensive kind of wrong.
//!
//! ## Where the numbers are
//!
//! `game.ron: net` — the port, the seat spread, the timeout and the 120 s slot hold. Not one
//! of them is a literal in this file (§4).

use bevy::prelude::*;
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};

use super::session::{Roster, SeatKind};
use super::{wire, Inbox};
use crate::data::GameData;
use crate::shared::{Cli, HostRequest, IdCounter, PlayerId, SeatPlayer, Tick, UnseatPlayer};

/// How many datagrams one tick will read before it gives up and leaves the rest in the kernel
/// buffer.
///
/// **A cap and not a `while true`.** Twenty players at 60 Hz are 20 frames per tick; anything
/// that sends a thousand is either broken or hostile, and a receive loop with no ceiling
/// hands it the power to stop the simulation. What is left over is read next tick, in order.
const MAX_DATAGRAMS_PER_TICK: usize = 256;

/// The open door, or nothing.
///
/// ⚠️ **`net` is the only writer.** The lobby's *Host* row writes `shared::HostRequest` and
/// reads this to draw its label — the same seam the deployment pads and the lobby already
/// share for `DeployRequest` (`docs/architecture.md`).
#[derive(Resource, Default)]
pub struct Host {
    socket: Option<UdpSocket>,
    /// Which address owns which chair. **This map is the authentication** — see the module
    /// header.
    peers: BTreeMap<SocketAddr, PlayerId>,
}

impl Host {
    pub fn is_open(&self) -> bool {
        self.socket.is_some()
    }

    /// The port really bound, which is not necessarily the port that was asked for — the OS
    /// has the last word, and a lobby that shows the wish instead of the fact sends people to
    /// the wrong number.
    pub fn port(&self) -> Option<u16> {
        self.socket.as_ref().and_then(|s| s.local_addr().ok()).map(|a| a.port())
    }

    pub fn peers(&self) -> usize {
        self.peers.len()
    }

    /// Binds, or says why not. Closing an already open door and opening it again is how a
    /// port change happens.
    pub fn open(&mut self, port: u16) -> std::io::Result<u16> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        // **Non-blocking, and the whole design rests on it.** A blocking `recv_from` inside
        // `FixedPreUpdate` would stop the game for as long as nobody sends anything, which is
        // most of the time.
        socket.set_nonblocking(true)?;
        let bound = socket.local_addr()?.port();
        self.socket = Some(socket);
        Ok(bound)
    }

    pub fn close(&mut self) {
        self.socket = None;
        self.peers.clear();
    }
}

/// **Which port this run would open** — asked for, or `--port`, or the file.
///
/// One function and not two, because the lobby's row prints this number while the door is
/// still shut and [`open_or_close_the_door`] binds it a moment later. A label that computed
/// the answer itself would tell the player 34197 on a run started with `--port 40000`.
pub fn wanted_port(asked: Option<u16>, start: &Cli, data: &GameData) -> u16 {
    asked.or(start.port).unwrap_or(data.game.net.port)
}

/// `--host`, and the lobby's *Host* row.
///
/// In `Update`: a lobby has `Time<Virtual>` stopped, so a system in a fixed schedule could
/// never answer the button that opened it — the same reason `mission` reads `DeployRequest`
/// there (`menu::apply_screen`).
pub fn open_or_close_the_door(
    mut requests: MessageReader<HostRequest>,
    mut host: ResMut<Host>,
    mut roster: ResMut<Roster>,
    mut unseat: MessageWriter<UnseatPlayer>,
    start: Res<Cli>,
    data: Res<GameData>,
) {
    for request in requests.read() {
        if !request.open {
            for id in roster.clear_remote() {
                unseat.write(UnseatPlayer { player: id });
            }
            host.close();
            info!("net: the door is closed");
            continue;
        }
        if host.is_open() {
            continue;
        }
        let want = wanted_port(request.port, &start, &data);
        match host.open(want) {
            Ok(bound) => info!("net: hosting on UDP port {bound} — intents only, no world"),
            // **Loud and not fatal.** A busy port is the normal case when a second copy of
            // the game is already running, and a game that exits over it is worse than one
            // that says so and stays single-player.
            Err(e) => error!("net: cannot host on port {want}: {e}"),
        }
    }
}

/// Reads what arrived and turns it into intents.
///
/// Runs in `IntentSystems::Source`, i.e. **before** the keyboard and before delivery: a
/// remote frame and a local key press land in the same inbox in the same tick, and the
/// simulation reads one channel.
pub fn receive_frames(
    mut host: ResMut<Host>,
    mut inbox: ResMut<Inbox>,
    mut roster: ResMut<Roster>,
    mut ids: ResMut<IdCounter>,
    mut seat_player: MessageWriter<SeatPlayer>,
    tick: Res<Tick>,
    data: Res<GameData>,
) {
    if !host.is_open() {
        return;
    }
    // A datagram bigger than a frame is not a frame. The buffer is deliberately larger than
    // `FRAME_BYTES` so that `decode` gets to say `Length(n)` instead of the kernel silently
    // truncating an oversized packet into something that parses.
    let mut buffer = [0u8; 1500];
    for _ in 0..MAX_DATAGRAMS_PER_TICK {
        let Some(socket) = host.socket.as_ref() else {
            return;
        };
        let (len, from) = match socket.recv_from(&mut buffer) {
            Ok(got) => got,
            Err(e) if e.kind() == ErrorKind::WouldBlock => return,
            Err(e) => {
                debug!("net: recv failed: {e}");
                return;
            }
        };
        let frame = match wire::decode(&buffer[..len]) {
            Ok(frame) => frame,
            Err(e) => {
                debug!("net: dropped a datagram from {from}: {e}");
                continue;
            }
        };

        // ⚠️ **`frame.player` is not read.** The chair belongs to the address, so a peer
        // cannot send an intent in somebody else's name (module header).
        let seat = match host.peers.get(&from) {
            Some(id) => *id,
            None => {
                let id = ids.next_player();
                host.peers.insert(from, id);
                let index = roster.len() as f32;
                roster.seat(id, SeatKind::Remote(from), from.to_string(), tick.0);
                let pos = Vec3::new(index * data.game.net.seat_spread_m, 2.0, 0.0);
                info!("net: {from} joined as player {} at {pos:?}", id.0);
                seat_player.write(SeatPlayer {
                    player: id,
                    local: false,
                    pos_x: pos.x,
                    pos_y: pos.y,
                    pos_z: pos.z,
                });
                id
            }
        };
        roster.heard(seat, tick.0);
        inbox.push(seat, frame.intent, tick.0);
    }
}

/// Notices a line that has gone quiet, and after the bible's 120 s gives the chair away.
pub fn sweep_peers(
    mut host: ResMut<Host>,
    mut roster: ResMut<Roster>,
    mut unseat: MessageWriter<UnseatPlayer>,
    tick: Res<Tick>,
    data: Res<GameData>,
) {
    if !host.is_open() {
        return;
    }
    let hz = data.game.simulation_hz;
    let timeout = (f64::from(data.game.net.peer_timeout_s) * hz).round() as u64;
    let hold = (f64::from(data.game.net.slot_hold_s) * hz).round() as u64;
    // Change detection, by hand and for the same reason as `session::seat_the_local_player`:
    // `sweep` takes `&mut self` and this runs every tick, so an untouched `ResMut` would mark
    // the roster changed sixty times a second and rebuild the lobby's squad list with it.
    // What a reader cares about is whether a seat appeared, vanished or went quiet.
    let connected = |r: &Roster| r.iter().filter(|(_, seat)| seat.connected()).count();
    let before = connected(&roster);
    let freed = roster.bypass_change_detection().sweep(tick.0, timeout, hold);
    let after = connected(&roster);
    if !freed.is_empty() || before != after {
        roster.set_changed();
    }
    for id in freed {
        host.peers.retain(|_, seated| *seated != id);
        unseat.write(UnseatPlayer { player: id });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_a_bound_port_reports_what_the_os_gave_it() {
        // Port 0 is "any free one". A lobby that showed the wish instead of the fact would
        // send people to a port nobody is listening on.
        let mut host = Host::default();
        assert!(!host.is_open());
        let bound = host.open(0).expect("binding an ephemeral port must work");
        assert_ne!(bound, 0, "the OS gave a real port and the wish was 0");
        assert_eq!(host.port(), Some(bound));
        host.close();
        assert!(!host.is_open());
        assert_eq!(host.port(), None);
    }
}
