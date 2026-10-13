#!/usr/bin/env python3
"""0073 owner_quality_blend_guard: CPU controls, standard library only.

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_blend_guard/controls/guard_controls.py \
        --out handoffs/0073-upscaling-review/owner_quality_blend_guard/controls/guard_controls.json

Not an engine run: a transcription of the patched ownership and composite rules.
1. Output-grid lifecycle: a reference-counted mirror of upscaling::State/Pending
   (prepare, offer_grid, accept, abandon, disable, resize, reuse) and the admission
   charge (retained grid + worst-case candidate) under 671105024 B.
2. Sanitation: composite.wgsl clean() arithmetic with the output-extent overlay and
   the source == output composite uniform, in IEEE f32, for adversarial overlay
   texels (NaN, +/-inf, negative, alpha > 1, alpha <= 0).
"""
import argparse
import json
import math
import struct
import sys

sys.dont_write_bytecode = True
CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


# 1. Lifecycle ----------------------------------------------------------------------------

class Grid:
    """Arc<GridBuffers>: identity, size and external references."""
    created = 0

    def __init__(self, dims, words):
        Grid.created += 1
        self.dims, self.words = dims, words
        self.refs = 0
        self.bytes = dims[0] * dims[1] * 24 * 4 * (1 + words)


def acquire(g):
    if g is not None:
        g.refs += 1
    return g


def release(g):
    if g is not None:
        g.refs -= 1


class State:
    def __init__(self):
        self.grid = None

    def grid_bytes(self):
        return self.grid.bytes if self.grid else 0

    def prepare(self):
        return {"retained": acquire(self.grid), "grid": None}

    def accept(self, p):
        release(p["retained"])
        if p["grid"] is not None:
            release(self.grid)
            self.grid = p["grid"]          # ownership moves from Pending to State
        p["grid"] = None

    def abandon(self, p):
        release(p["retained"])
        release(p["grid"])

    def drop(self):
        release(self.grid)
        self.grid = None


def prepare_output(retained, dims, words):
    """lighting::prepare_output: reuse a compatible retained grid, else allocate."""
    if retained is not None and retained.dims == dims and retained.words == words:
        return acquire(retained)
    return acquire(Grid(dims, words))


def offer(p, grid):
    if p["grid"] is None:
        p["grid"] = grid
    else:
        release(grid)


def lifecycle():
    log = {}
    s = State()
    p = s.prepare()
    offer(p, prepare_output(p["retained"], (14, 8), 128))
    s.accept(p)
    a = s.grid
    check("accepted BLEND frame: state is the only logical owner", a.refs == 1, a.refs)
    p = s.prepare()
    offer(p, prepare_output(p["retained"], (14, 8), 128))
    check("compatible next frame reuses the same grid (no allocation)", p["grid"] is a and Grid.created == 1)
    s.accept(p)
    check("reuse keeps one owner", a.refs == 1, a.refs)
    p = s.prepare()
    offer(p, prepare_output(p["retained"], (32, 32), 128))
    b = p["grid"]
    log["resize_candidate_bytes"] = b.bytes
    s.abandon(p)
    check("abandoned resize: candidate unowned, retained grid unchanged", b.refs == 0 and s.grid is a and a.refs == 1)
    p = s.prepare()
    s.accept(p)
    check("frame without BLEND keeps (and still charges) the retained grid", s.grid is a and s.grid_bytes() == a.bytes)
    queued = acquire(s.grid)               # a bind group in a submitted, unfinished command buffer
    s.drop()                               # upscaling disabled: RetainedScene drops its State
    check("disable releases logical ownership; the queued reference is the driver's, not admission's",
          a.refs == 1 and queued is a, {"remaining_refs": a.refs})
    release(queued)
    check("after GPU completion no reference remains", a.refs == 0)
    log["grids_created"] = Grid.created
    return log


P = 4 * 1024 * 1024
INST = 64 * 1024 * 1024
CEILING = 128 * P + 2 * INST + 16384
PER_CLUSTER = 4 * (1 + 128)


def allowance(rect):
    if not all(math.isfinite(v) for v in rect) or rect[2] <= 0 or rect[3] <= 0:
        return None
    c = math.ceil(rect[2] / 64) * math.ceil(rect[3] / 64) * 24
    return c * PER_CLUSTER if c <= 131072 else None


def accounting():
    out = {}
    base = 36 * P + 8 * P + 24 * P + 44 * (2048 * 2048) + 24 * (1024 * 1024) + 2 * INST + 16384
    old = allowance([0, 0, 2048, 2048])
    total = base + old + allowance([0, 0, 2048, 2048])
    out["2048_half_after_maximal_old_with_retained_and_candidate_grid"] = total
    check("2048^2 at 8/16 after maximal old set, retained + candidate grid fits", total <= CEILING,
          CEILING - total)
    full = 36 * P + 8 * P + 24 * P + 44 * 1923 * 1923 + 24 * 1923 * 1923 + 2 * INST + 16384
    check("1923^2 full scale fits without grids but not with the candidate grid",
          full <= CEILING and full + allowance([0, 0, 1923, 1923]) > CEILING, CEILING - full)
    check("disabled upscaling holds no output grid to charge (state dropped)", State().grid_bytes() == 0)
    check("ceiling unchanged", CEILING == 671_105_024)
    return out


# 2. Sanitation ---------------------------------------------------------------------------

def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def finite4(v):
    return all(c == c and abs(c) <= 3.402823e38 for c in v)


def clean(v):
    if not finite4(v) or v[3] <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    return tuple([min(max(c, 0.0), 65504.0) for c in v[:3]] + [min(max(v[3], 0.0), 1.0)])


def composite(p, rect, overlay, under):
    """composite.wgsl with source == output == rect, overlay indexed by integer texel."""
    if not (rect[0] <= p[0] < rect[0] + rect[2] and rect[1] <= p[1] < rect[1] + rect[3]):
        return (0.0, 0.0, 0.0, 0.0), None
    src = [f32(f32(rect[i]) + f32(f32(f32(p[i]) - f32(rect[i])) / f32(rect[i + 2])) * f32(rect[i + 2]))
           for i in range(2)]
    q = [f32(c - 0.5) for c in src]
    base = [math.floor(c) for c in q]
    f = [f32(q[i] - base[i]) for i in range(2)]
    lo = [math.ceil(rect[i] - 0.5) for i in range(2)]
    hi = [math.ceil(rect[i] + rect[i + 2] - 0.5) - 1 for i in range(2)]
    over = [0.0] * 4
    for y in (0, 1):
        for x in (0, 1):
            w = (f[0] if x else 1 - f[0]) * (f[1] if y else 1 - f[1])
            t = (min(max(base[0] + x, lo[0]), hi[0]), min(max(base[1] + y, lo[1]), hi[1]))
            c = clean(overlay(t))
            over = [o + ci * w for o, ci in zip(over, c)]
    u = clean(under)
    return clean(tuple(o + ui * (1 - over[3]) for o, ui in zip(over, u))), max(abs(f[0] - round(f[0])),
                                                                                abs(f[1] - round(f[1])))


def sanitation():
    bad = [(float("nan"), 0.2, 0.2, 0.5), (float("inf"), 0.0, 0.0, 1.0), (-4.0, 0.5, 0.5, 0.5),
           (0.4, 0.4, 0.4, 7.0), (9.0, 9.0, 9.0, 0.0), (-0.0, 1e38, 0.3, -1.0), (0.1, 0.2, 0.3, float("-inf"))]
    worst_leak = 0.0
    canonical = True
    for rect in ([37.5, 21.25, 885.0, 497.5], [3.25, 5.5, 65.0, 33.0], [0.0, 0.0, 8191.0, 3.0]):
        for py in range(int(rect[1]), int(rect[1] + rect[3]), max(1, int(rect[3]) // 7)):
            for px in range(int(rect[0]), int(rect[0] + rect[2]), max(1, int(rect[2]) // 97)):
                for b in bad:
                    out, leak = composite((px + 0.5, py + 0.5), rect, lambda t, b=b: b, (0.2, 0.3, 0.4, 1.0))
                    canonical &= finite4(out) and all(c >= 0 for c in out) and out[3] <= 1
                    if leak is not None:
                        worst_leak = max(worst_leak, leak)
    check("adversarial output-resolution BLEND texels always leave the composite canonical", canonical)
    check("source == output composite reads the overlay 1:1 up to f32 rounding of (p-o)/w*w",
          worst_leak < 2e-3, worst_leak)
    return {"worst_neighbour_weight": worst_leak,
            "note": "post-condition of the unchanged clean() calls; no claim about observed NaN inputs"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    result = {"model": "0073 owner_quality_blend_guard CPU controls (transcription, not execution)",
              "lifecycle": lifecycle(), "accounting": accounting(), "sanitation": sanitation(), "checks": CHECKS}
    result["all_checks_pass"] = all(c["ok"] for c in CHECKS)
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    failed = [c for c in CHECKS if not c["ok"]]
    print(f"checks={len(CHECKS)} failed={len(failed)}")
    for c in failed:
        print("FAILED", c)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
