#!/usr/bin/env python3
"""Glue two screenshots side by side into one PNG.

    python3 tools/two_panels.py left.png right.png out.png [--label-height 0]

**Why this exists.** Some acceptance criteria are a *difference* and not a state — the F-053
telegraph is the clear case: a still frame of a titan with his arm down is indistinguishable
from a titan with no telegraph at all (`docs/PLAN-GAME.md` §8 F-053). The evidence for such a
criterion is two frames of one run at two ticks, in one file, from one camera.

`--offscreen` writes one image per run, so the two panels come from two runs of the same
script at two `--ticks` values (`scripts/f053-windup.txt` writes the two commands down).

**Why it is 90 lines of PNG instead of three lines of Pillow.** Machine A has neither Pillow
nor ImageMagick (`docs/environment.md`), and a piece of evidence that can only be assembled on
a machine with an extra dependency is a piece of evidence nobody re-makes. Only what Bevy's
screenshot actually writes is supported — 8 bit, non-interlaced, RGB or RGBA — and anything
else is refused by name instead of being guessed at.
"""

import struct
import sys
import zlib


def read_png(path):
    """-> (width, height, channels, bytes). Refuses anything it cannot read exactly."""
    with open(path, "rb") as f:
        blob = f.read()
    if blob[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"{path} is not a PNG")
    pos, header, idat = 8, None, bytearray()
    while pos < len(blob):
        (length,) = struct.unpack(">I", blob[pos:pos + 4])
        kind = blob[pos + 4:pos + 8]
        body = blob[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
        pos += 12 + length
    if header is None:
        raise SystemExit(f"{path} has no IHDR")
    width, height, depth, colour, compression, filt, interlace = header
    if depth != 8 or interlace != 0 or colour not in (2, 6):
        raise SystemExit(
            f"{path}: {depth} bit, colour type {colour}, interlace {interlace} — this reader "
            f"does only 8 bit RGB/RGBA, non-interlaced, which is what Bevy's screenshot writes"
        )
    channels = 3 if colour == 2 else 4
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    out = bytearray(height * stride)
    previous = bytearray(stride)
    at = 0
    for y in range(height):
        method = raw[at]
        line = bytearray(raw[at + 1:at + 1 + stride])
        at += 1 + stride
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = previous[x]
            c = previous[x - channels] if x >= channels else 0
            if method == 1:
                line[x] = (line[x] + a) & 0xFF
            elif method == 2:
                line[x] = (line[x] + b) & 0xFF
            elif method == 3:
                line[x] = (line[x] + ((a + b) >> 1)) & 0xFF
            elif method == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pred = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pred) & 0xFF
            elif method != 0:
                raise SystemExit(f"{path}: unknown filter {method} on row {y}")
        out[y * stride:(y + 1) * stride] = line
        previous = line
    return width, height, channels, out


def write_png(path, width, height, channels, pixels):
    raw = bytearray()
    stride = width * channels
    for y in range(height):
        raw.append(0)  # filter "none": the file is evidence, not a download
        raw += pixels[y * stride:(y + 1) * stride]

    def chunk(kind, body):
        return (
            struct.pack(">I", len(body))
            + kind
            + body
            + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)
        )

    colour = 2 if channels == 3 else 6
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n")
        f.write(chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, colour, 0, 0, 0)))
        f.write(chunk(b"IDAT", zlib.compress(bytes(raw), 9)))
        f.write(chunk(b"IEND", b""))


def main(argv):
    if len(argv) != 4:
        raise SystemExit(__doc__)
    left_path, right_path, out_path = argv[1:4]
    lw, lh, lc, left = read_png(left_path)
    rw, rh, rc, right = read_png(right_path)
    if (lh, lc) != (rh, rc):
        raise SystemExit(
            f"the two panels do not match: {lw}x{lh}x{lc} against {rw}x{rh}x{rc}. Two panels of "
            f"one criterion have to come out of the same camera and the same run"
        )
    gap = 8
    width, height = lw + gap + rw, lh
    out = bytearray(width * height * lc)
    for y in range(height):
        row = y * width * lc
        out[row:row + lw * lc] = left[y * lw * lc:(y + 1) * lw * lc]
        start = row + (lw + gap) * lc
        out[start:start + rw * lc] = right[y * rw * lc:(y + 1) * rw * lc]
    write_png(out_path, width, height, lc, out)
    print(f"{out_path}: {width}x{height}, {left_path} | {right_path}")


if __name__ == "__main__":
    main(sys.argv)
