#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
#
# check-model-observe.py: compares two frames of the camera for `check-model.sh --observe`.
#
#   scripts/check-model-observe.py <before.pgm> <after.pgm>
#
# The frames are binary greyscale PGM (P5, 8 bit) of the same size, as ffmpeg writes them.
# Prints one line: a verdict and the measurements it rests on.
#
#   moved   the picture shifted (phase correlation), was scaled (zoom) or changed so much that
#           the two frames no longer match (a move that leaves the old view)
#   still   none of these
#   dark    one of the frames is (nearly) black, as the picture of a sleeping camera
#
# Measurements: scale=<s> (`after` is `before` magnified by s) and shift=<dx>,<dy> px (content
# of `after` relative to `before` at that scale, x right, y down) with the height of their
# phase-correlation peak,
# ncc=<n> (normalised correlation of the two frames), mirror=<n> (correlation of `after` with
# `before` flipped left to right), mean=<before>-><after> (brightness 0-255) and
# sharp=<before>-><after> (variance of the Laplacian: lower is blurrier).
# Needs Python 3 and numpy. Exit: 0 a verdict was printed; 2 usage or unreadable frames.

import sys

try:
    import numpy as np
except ImportError:
    print("check-model-observe.py: numpy is needed", file=sys.stderr)
    sys.exit(2)

# A shift of at least this many pixels (of a 320 px wide frame: 1.25 %) with a clear peak, a
# scale off 1 by 4 % or a correlation below 0.9 is a moved picture. Measured on the Tiny 2
# Lite in a dim room: two frames of an unchanged view correlate at 0.94 to 1.00, a tilt of
# 16 degrees at 0.73 (its phase-correlation peak too low to count). Unchanged pairs peak at
# 0.33 to 0.44 with no shift and a scale within one step of 1 (1.01); a 5 degree pan shifts by
# 23 px at a peak of 0.17. Unchanged frames of a dim scene correlate at 0.94 only, and a person
# who moves a lot can fall below 0.9: check-model.sh therefore passes a move only after a still
# baseline, and asks otherwise.
SHIFT_PX = 4.0
PEAK_MIN = 0.05
SCALE_MIN = 0.04
NCC_MOVED = 0.9
DARK_MEAN = 4.0


def read_pgm(path):
    with open(path, "rb") as f:
        data = f.read()
    fields = []
    pos = 0
    # header: P5, width, height, maxval, separated by whitespace (comments are not written)
    while len(fields) < 4:
        while pos < len(data) and data[pos : pos + 1].isspace():
            pos += 1
        start = pos
        while pos < len(data) and not data[pos : pos + 1].isspace():
            pos += 1
        if start == pos:
            raise ValueError("short header")
        fields.append(data[start:pos])
    pos += 1
    if fields[0] != b"P5" or int(fields[3]) > 255:
        raise ValueError("not an 8-bit P5 file")
    w, h = int(fields[1]), int(fields[2])
    pixels = np.frombuffer(data, dtype=np.uint8, count=w * h, offset=pos)
    return pixels.reshape(h, w).astype(np.float64)


def ncc(a, b):
    a = a - a.mean()
    b = b - b.mean()
    d = np.sqrt((a * a).sum() * (b * b).sum())
    return float((a * b).sum() / d) if d > 0 else 0.0


def laplacian_var(a):
    lap = -4 * a[1:-1, 1:-1] + a[:-2, 1:-1] + a[2:, 1:-1] + a[1:-1, :-2] + a[1:-1, 2:]
    return float(lap.var())


def phase_shift(a, b):
    """The shift of b's content relative to a's, and the height of the correlation peak."""
    h, w = a.shape
    win = np.outer(np.hanning(h), np.hanning(w))
    fa = np.fft.fft2((a - a.mean()) * win)
    fb = np.fft.fft2((b - b.mean()) * win)
    cross = fb * np.conj(fa)
    cross /= np.abs(cross) + 1e-9
    r = np.fft.ifft2(cross).real
    y, x = np.unravel_index(np.argmax(r), r.shape)
    peak = float(r[y, x])
    if y > h // 2:
        y -= h
    if x > w // 2:
        x -= w
    return float(x), float(y), peak


def magnified(a, s):
    """a magnified by s about its centre (bilinear), the same size."""
    h, w = a.shape
    cy, cx = (h - 1) / 2, (w - 1) / 2
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float64)
    sy = np.clip(cy + (yy - cy) / s, 0, h - 1.001)
    sx = np.clip(cx + (xx - cx) / s, 0, w - 1.001)
    y0, x0 = np.floor(sy).astype(int), np.floor(sx).astype(int)
    fy, fx = sy - y0, sx - x0
    return (
        a[y0, x0] * (1 - fy) * (1 - fx)
        + a[y0, x0 + 1] * (1 - fy) * fx
        + a[y0 + 1, x0] * fy * (1 - fx)
        + a[y0 + 1, x0 + 1] * fy * fx
    )


def scale_and_shift(a, b):
    """The magnification s (0.5 to 2) of a that b matches best, with the shift and peak of
    that match: the s whose phase correlation has the highest peak (a shift does not move the
    peak's height, so a move and a zoom are told apart)."""
    best = (1.0, 0.0, 0.0, -1.0)
    for s in np.exp(np.linspace(np.log(0.5), np.log(2.0), 97)):
        dx, dy, peak = phase_shift(magnified(a, s) if abs(s - 1) > 1e-6 else a, b)
        if peak > best[3]:
            best = (float(s), dx, dy, peak)
    return best


def compare(a, b):
    mean_a, mean_b = float(a.mean()), float(b.mean())
    text = f"mean={mean_a:.0f}->{mean_b:.0f}"
    if min(mean_a, mean_b) < DARK_MEAN or min(a.std(), b.std()) < 1.0:
        return f"dark {text}"
    s, dx, dy, peak = scale_and_shift(a, b)
    n = ncc(a, b)
    m = ncc(b, a[:, ::-1])
    moved = (
        (peak >= PEAK_MIN and np.hypot(dx, dy) >= SHIFT_PX)
        or abs(s - 1) >= SCALE_MIN
        or n < NCC_MOVED
    )
    return (
        f"{'moved' if moved else 'still'} shift={dx:+.0f},{dy:+.0f} peak={peak:.2f} "
        f"scale={s:.2f} ncc={n:.2f} mirror={m:.2f} {text} "
        f"sharp={laplacian_var(a):.0f}->{laplacian_var(b):.0f}"
    )


def main(argv):
    if len(argv) != 3:
        print("usage: check-model-observe.py <before.pgm> <after.pgm>", file=sys.stderr)
        return 2
    try:
        a, b = read_pgm(argv[1]), read_pgm(argv[2])
    except (OSError, ValueError) as e:
        print(f"check-model-observe.py: {e}", file=sys.stderr)
        return 2
    if a.shape != b.shape:
        print("check-model-observe.py: the frames differ in size", file=sys.stderr)
        return 2
    print(compare(a, b))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
