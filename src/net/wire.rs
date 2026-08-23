//! **The wire format** — what one tick of one player looks like as bytes.
//!
//! This is the first thing in this repository that is not a Rust value passed between two
//! systems. It is the reason [`Intent`] was built the way it was on day one: bare `f32`, a
//! `u32` of buttons, no `Vec3`, no `Entity`, no handle (`docs/multiplayer.md` rules 7 and 8).
//! Here that decision is cashed in — the encoder is a straight line of `to_le_bytes` and
//! there is nothing in the struct it cannot carry.
//!
//! ```text
//! byte 0        1..5        5..13     13..29                29..33
//! ┌─────────┬───────────┬──────────┬─────────────────────┬──────────┐
//! │ version │ player id │   tick   │ 4 × f32 (move, look)│ buttons  │
//! │   0x01  │    u32    │   u64    │                     │   u32    │
//! └─────────┴───────────┴──────────┴─────────────────────┴──────────┘
//!                            33 bytes, always
//! ```
//!
//! ## Why by hand and not with a serde format
//!
//! `ron` is already a dependency and `Intent` already derives `Serialize` — so this could
//! have been three lines. It is not, for two reasons that are worth 60:
//!
//! - **A fixed size is a property, not an accident.** 33 bytes at 60 Hz is 2.0 kB/s per
//!   player, and twenty players are 40 kB/s in each direction. That number has to be
//!   *knowable* before the netcode is designed around it, and a text format makes it depend
//!   on how many decimal places a yaw happens to need. [`FRAME_BYTES`] is a constant and
//!   `wire_a_frame_is_always_the_same_size` is the test that keeps it one.
//! - **Little-endian, explicitly.** `to_le_bytes` says what goes on the line; a derive says
//!   "whatever the format does". Two machines of different endianness are not a scenario
//!   today and this is the one line that keeps it from becoming one.
//!
//! ## What this does NOT do, and it matters
//!
//! No checksum, no sequence number, no encryption, no compression, and **no authentication**
//! — a frame carries a [`PlayerId`], and [`decode`] will happily hand you the one the sender
//! wrote. `net::socket` therefore **throws that field away** and uses the seat the packet's
//! address already owns. Whoever moves this codec anywhere near a public port reads that
//! paragraph again first.

use crate::shared::{Buttons, Intent, PlayerId};

/// The version byte every frame starts with.
///
/// A frame from a client of another build is dropped and not misread. It is one byte and it
/// is the difference between "your friend has an old version" and a player who twitches.
pub const VERSION: u8 = 0x01;

/// How many bytes one frame is. **Always** — see the module header.
pub const FRAME_BYTES: usize = 1 + 4 + 8 + 4 * 4 + 4;

/// One player's wish for one tick, as it travels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Who the **sender says** he is. `net::socket` does not believe it — see the module
    /// header.
    pub player: PlayerId,
    pub intent: Intent,
}

/// Why a packet was thrown away.
///
/// An enum and not a `bool`, because the three cases mean three different things to whoever
/// reads the log: a wrong version is a build mismatch, a wrong length is a truncated or
/// padded datagram, and neither is "somebody is attacking us".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireError {
    /// Not [`FRAME_BYTES`] long.
    Length(usize),
    /// A first byte that is not [`VERSION`].
    Version(u8),
}

impl std::fmt::Display for WireError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WireError::Length(n) => write!(f, "{n} bytes, expected {FRAME_BYTES}"),
            WireError::Version(v) => write!(f, "version {v:#04x}, expected {VERSION:#04x}"),
        }
    }
}

/// Writes a frame. Never fails and never allocates more than [`FRAME_BYTES`].
pub fn encode(frame: &Frame) -> [u8; FRAME_BYTES] {
    let mut out = [0u8; FRAME_BYTES];
    let i = &frame.intent;
    out[0] = VERSION;
    out[1..5].copy_from_slice(&frame.player.0.to_le_bytes());
    out[5..13].copy_from_slice(&i.tick.to_le_bytes());
    for (slot, value) in [i.move_x, i.move_y, i.yaw, i.pitch]
        .iter()
        .enumerate()
    {
        let at = 13 + slot * 4;
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    out[29..33].copy_from_slice(&i.buttons.0.to_le_bytes());
    out
}

/// Reads a frame, or says why not.
///
/// **It never panics on a hostile datagram**, and that is the whole contract: every slice
/// index below is inside a buffer whose length has already been checked, and every `f32`
/// comes out of `from_le_bytes`, which has no invalid bit patterns. A UDP port is reachable
/// by anybody on the machine.
pub fn decode(bytes: &[u8]) -> Result<Frame, WireError> {
    if bytes.len() != FRAME_BYTES {
        return Err(WireError::Length(bytes.len()));
    }
    if bytes[0] != VERSION {
        return Err(WireError::Version(bytes[0]));
    }
    let u32_at = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    let f32_at = |at: usize| f32::from_bits(u32_at(at));
    let mut tick = [0u8; 8];
    tick.copy_from_slice(&bytes[5..13]);
    Ok(Frame {
        player: PlayerId(u32_at(1)),
        intent: Intent {
            tick: u64::from_le_bytes(tick),
            move_x: f32_at(13),
            move_y: f32_at(17),
            yaw: f32_at(21),
            pitch: f32_at(25),
            buttons: Buttons(u32_at(29)),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_frame() -> Frame {
        let mut buttons = Buttons::NONE;
        buttons.set(Buttons::HOOK_LEFT, true);
        buttons.set(Buttons::BOOST, true);
        Frame {
            player: PlayerId(3),
            intent: Intent {
                move_x: -1.0,
                move_y: 0.5,
                yaw: 1.234_5,
                pitch: -0.678_9,
                buttons,
                tick: 4_294_967_400, // deliberately past u32 — the tick is a u64
            },
        }
    }

    #[test]
    fn wire_a_frame_survives_the_round_trip_bit_for_bit() {
        let sent = a_frame();
        let got = decode(&encode(&sent)).expect("a frame we wrote ourselves");
        assert_eq!(got, sent, "what went in is not what came out");
    }

    #[test]
    fn wire_a_frame_is_always_the_same_size() {
        // 37 bytes at 60 Hz is 2.2 kB/s per player. Twenty of them are 44 kB/s, and that
        // number has to stay knowable — see the module header.
        assert_eq!(FRAME_BYTES, 33);
        assert_eq!(encode(&a_frame()).len(), FRAME_BYTES);
        assert_eq!(
            encode(&Frame { player: PlayerId(0), intent: Intent::default() }).len(),
            FRAME_BYTES,
            "an empty intent takes exactly as many bytes as a full one"
        );
    }

    #[test]
    fn wire_a_short_datagram_is_an_error_and_not_a_panic() {
        let full = encode(&a_frame());
        for len in [0usize, 1, 12, FRAME_BYTES - 1] {
            assert_eq!(decode(&full[..len]), Err(WireError::Length(len)));
        }
        let mut too_long = full.to_vec();
        too_long.push(0);
        assert_eq!(decode(&too_long), Err(WireError::Length(FRAME_BYTES + 1)));
    }

    #[test]
    fn wire_a_frame_from_another_build_is_refused() {
        let mut bytes = encode(&a_frame());
        bytes[0] = 0x02;
        assert_eq!(decode(&bytes), Err(WireError::Version(0x02)));
    }

    #[test]
    fn wire_every_byte_matters() {
        // A codec that ignores a field passes a round-trip test and loses a button on the
        // line. Flipping any single byte has to change what comes out.
        let base = encode(&a_frame());
        let reference = decode(&base).expect("valid");
        for byte in 1..FRAME_BYTES {
            let mut mutated = base;
            mutated[byte] ^= 0b0000_0001;
            let got = decode(&mutated).expect("still a valid frame");
            assert_ne!(
                got, reference,
                "byte {byte} is not read by decode() — it could carry anything"
            );
        }
    }
}
