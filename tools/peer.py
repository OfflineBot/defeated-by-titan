#!/usr/bin/env python3
"""peer.py — a second player, from another process, over UDP.

**This is not a client.** It sends input and receives nothing, because the host replicates no
world state: whoever runs this drives a body in somebody else's game and cannot see it. Read
`docs/multiplayer.md` §"What this is NOT" before expecting more. It exists for two reasons:

- it is the **only** thing that can prove the wire works end to end, from outside the process
  and outside the language (`docs/FINDINGS.md` FIND-128);
- it is how a second player is put into a headless run for evidence, where nobody can click.

Start the game with the door open, then this:

    ./target/debug/defeated_by_titan --host --headless --ticks 900 &
    python3 tools/peer.py --forward 10        # ten seconds of holding W

The frame is `src/net/wire.rs`: 37 bytes, little-endian, one version byte. The struct format
below is that layout and nothing else — if it ever disagrees, `wire.rs` is right.
"""

import argparse
import socket
import struct
import time

VERSION = 1
# B version | I player | Q tick | 5f move_x move_y yaw pitch spread | I buttons
FRAME = "<BIQfffffI"
FRAME_BYTES = 37

# `Buttons` from src/shared/intent.rs, in the same order.
BUTTONS = {
    "jump": 1 << 0,
    "hook_left": 1 << 1,
    "hook_right": 1 << 2,
    "reel_in": 1 << 3,
    "boost": 1 << 4,
    "slash_left": 1 << 5,
    "slash_right": 1 << 6,
    "dodge": 1 << 7,
    "mark": 1 << 8,
}


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=34197, help="game.ron: net.port")
    p.add_argument("--seconds", type=float, default=10.0)
    p.add_argument("--hz", type=float, default=60.0, help="one frame per simulation tick")
    p.add_argument("--move-x", type=float, default=0.0, help="strafe, -1..1")
    p.add_argument("--move-y", type=float, default=0.0, help="forward, -1..1")
    p.add_argument("--forward", type=float, help="shorthand: --move-y 1 --seconds N")
    p.add_argument("--yaw", type=float, default=0.0, help="radians, 0 looks down -Z")
    p.add_argument("--pitch", type=float, default=0.0, help="radians, + is up")
    p.add_argument("--spread", type=float, default=11.0, help="aim spread ceiling, degrees")
    p.add_argument(
        "--press",
        action="append",
        default=[],
        choices=sorted(BUTTONS),
        help="hold a button for the whole run; may be given more than once",
    )
    args = p.parse_args()

    if args.forward is not None:
        args.move_y, args.seconds = 1.0, args.forward

    buttons = 0
    for name in args.press:
        buttons |= BUTTONS[name]

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    target = (args.host, args.port)
    period = 1.0 / args.hz
    ticks = int(args.seconds * args.hz)
    print(f"peer: {ticks} frames to {target[0]}:{target[1]} at {args.hz:g} Hz")

    for tick in range(ticks):
        # ⚠️ The player id is a claim and the host ignores it: the seat belongs to the address
        # this datagram comes from (`src/net/socket.rs`). 0 is written to make that visible.
        frame = struct.pack(
            FRAME,
            VERSION,
            0,
            tick,
            args.move_x,
            args.move_y,
            args.yaw,
            args.pitch,
            args.spread,
            buttons,
        )
        assert len(frame) == FRAME_BYTES, f"{len(frame)} bytes — wire.rs says {FRAME_BYTES}"
        sock.sendto(frame, target)
        time.sleep(period)
    print("peer: done — the host holds the seat for game.ron: net.slot_hold_s seconds")


if __name__ == "__main__":
    main()
