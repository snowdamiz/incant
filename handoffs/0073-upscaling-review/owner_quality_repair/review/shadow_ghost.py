#!/usr/bin/env python3
"""0073 repair review: current shadow versus temporal contribution at motion step 20.

Standard library only, deterministic, read-only on retained evidence:

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_repair/review/shadow_ghost.py \
        --out handoffs/0073-upscaling-review/owner_quality_repair/review/shadow_ghost.json

Per output pixel at step 20 it ray-casts the review fixture's motion stage:
- camera_world(20);
- the sign quad (z = 0, x 2.3..4.5, y 0.15..2.35);
- the ground (y = 0);
- the cube (half 0.32 at mover_world(k));
- the sphere (1.3, 0.85, 1.8; r 0.85);
- the engine light direction world * -Z of sun_rotation(38, 30).

Every pixel is classified by its visible receiver:
- shadow_now: shadowed by the cube at step 20, so FULL holds the same current shadow.
- shadow_trail: shadowed by the cube at some step 15..19 but not at 20, so any
  extra darkening there is temporal.
- cube_trail: hidden by the cube from the step-k camera at some step 15..19 but
  visible at 20 (disocclusion).
- lit_control: never cube-shadowed or cube-hidden at steps 1..20.

It then compares Rec. 709 luma of 8-bit sRGB codes in the retained FULL, UP and
OWN step-20 PNGs of the repair run (gpu-002) and the failed run (gpu-001). The
PNGs are decoded in memory only; no image is written and nothing is rendered.
Geometry is the fixture's analytic scene; rasterization coverage, the shadow
filter and the motion blur of a moving shadow are not modelled. Regions keep a
1.5 px guard band from every classification boundary.
"""
import argparse
import functools
import hashlib
import json
import math
import os
import struct
import zlib

RUNS = {
    "repair_gpu002": os.path.expanduser("~/.codex/incant-evidence/0073-owner-quality-gpu-002/motion/artifacts"),
    "failed_gpu001": os.path.expanduser("~/.codex/incant-evidence/0073-owner-quality-gpu-001/motion/artifacts"),
}
FRAMES = {"full": "motion-full.png", "up": "motion-up50.png", "own": "motion-own50.png"}
W, H = 960, 540
FOV, PITCH = math.radians(50.0), -0.16
CAMERA_AT = (0.0, 2.4, 9.5)
MOVER_AT, HALF = (2.02, 1.05, 1.2), 0.32
SPHERE, RADIUS = (1.3, 0.85, 1.8), 0.85
SIGN_X, SIGN_Y = (2.3, 4.5), (0.15, 2.35)


def q_mul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return (aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz)


def q_rot(q, v):
    x, y, z, w = q
    t = (2 * (y * v[2] - z * v[1]), 2 * (z * v[0] - x * v[2]), 2 * (x * v[1] - y * v[0]))
    return (v[0] + w * t[0] + y * t[2] - z * t[1], v[1] + w * t[1] + z * t[0] - x * t[2],
            v[2] + w * t[2] + x * t[1] - y * t[0])


def q_axis(axis, angle):
    s = math.sin(angle / 2)
    return tuple(s if i == axis else 0.0 for i in range(3)) + (math.cos(angle / 2),)


SUN = q_mul(q_axis(1, math.radians(30.0)), q_axis(0, -math.radians(38.0)))
LIGHT = q_rot(SUN, (0.0, 0.0, -1.0))          # travel direction
TO_LIGHT = tuple(-c for c in LIGHT)


def camera(step):
    return (CAMERA_AT[0] + max(step - 14, 0) * 0.05, CAMERA_AT[1], CAMERA_AT[2])


def cube(step):
    x = MOVER_AT[0] + max(step - 8, 0) * 0.12
    return ((x - HALF, MOVER_AT[1] - HALF, MOVER_AT[2] - HALF), (x + HALF, MOVER_AT[1] + HALF, MOVER_AT[2] + HALF))


def ray_box(o, d, box, t_max=math.inf):
    lo, hi = box
    t0, t1 = 1e-6, t_max
    for i in range(3):
        if abs(d[i]) < 1e-12:
            if o[i] < lo[i] or o[i] > hi[i]:
                return None
            continue
        a, b = (lo[i] - o[i]) / d[i], (hi[i] - o[i]) / d[i]
        t0, t1 = max(t0, min(a, b)), min(t1, max(a, b))
        if t0 > t1:
            return None
    return t0


def ray_sphere(o, d):
    oc = tuple(o[i] - SPHERE[i] for i in range(3))
    b = sum(oc[i] * d[i] for i in range(3))
    c = sum(x * x for x in oc) - RADIUS * RADIUS
    disc = b * b - c
    if disc < 0:
        return None
    t = -b - math.sqrt(disc)
    return t if t > 1e-6 else None


def pixel_ray(step, x, y):
    f = math.tan(FOV / 2)
    d = (((x / W) * 2 - 1) * f * W / H, (1 - (y / H) * 2) * f, -1.0)
    d = q_rot(q_axis(0, PITCH), d)
    n = math.sqrt(sum(c * c for c in d))
    return camera(step), tuple(c / n for c in d)


def receiver(x, y):
    """First visible receiver of the step-20 camera ray through (x, y): ('sign'|'ground', point) or None."""
    o, d = pixel_ray(20, x, y)
    hits = []
    if abs(d[2]) > 1e-12:
        t = -o[2] / d[2]
        p = tuple(o[i] + t * d[i] for i in range(3))
        if t > 0 and SIGN_X[0] <= p[0] <= SIGN_X[1] and SIGN_Y[0] <= p[1] <= SIGN_Y[1]:
            hits.append((t, "sign", p))
    if abs(d[1]) > 1e-12:
        t = -o[1] / d[1]
        p = tuple(o[i] + t * d[i] for i in range(3))
        if t > 0:
            hits.append((t, "ground", p))
    if not hits:
        return None
    t, kind, p = min(hits)
    tc = ray_box(o, d, cube(20), t)
    ts = ray_sphere(o, d)
    if tc is not None or (ts is not None and ts < t):
        return None
    return kind, p


def shadowed(p, step):
    return ray_box(p, TO_LIGHT, cube(step)) is not None


def hidden(p, step):
    o = camera(step)
    d = tuple(p[i] - o[i] for i in range(3))
    return ray_box(o, d, cube(step), 1.0 - 1e-6) is not None


@functools.lru_cache(maxsize=None)
def classify(x, y):
    r = receiver(x, y)
    if r is None:
        return None
    kind, p = r
    if ray_sphere(p, TO_LIGHT) is not None:
        return None                        # static sphere shadow: excluded everywhere
    if shadowed(p, 20):
        return f"{kind}_shadow_now"
    if any(shadowed(p, k) for k in range(15, 20)):
        return f"{kind}_shadow_trail"
    if any(hidden(p, k) for k in range(15, 20)):
        return f"{kind}_cube_trail"
    if not any(shadowed(p, k) or hidden(p, k) for k in range(1, 21)):
        return f"{kind}_lit_control"
    return None


def decode_png(path):
    raw = open(path, "rb").read()
    offset, idat = 8, b""
    while offset < len(raw):
        n = struct.unpack(">I", raw[offset:offset + 4])[0]
        if raw[offset + 4:offset + 8] == b"IDAT":
            idat += raw[offset + 8:offset + 8 + n]
        offset += 12 + n
    data, stride, rows, prev = zlib.decompress(idat), W * 4, [], bytearray(W * 4)
    for y in range(H):
        f, line = data[y * (stride + 1)], bytearray(data[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b, c = prev[i], prev[i - 4] if i >= 4 else 0
            if f == 1:
                line[i] = (line[i] + a) & 255
            elif f == 2:
                line[i] = (line[i] + b) & 255
            elif f == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return rows, hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    # Classify pixel centres in the window around the sign base and the cube's path.
    x0, x1, y0, y1 = 540, 820, 230, 360
    labels = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            centre = classify(x + 0.5, y + 0.5)
            if centre is None:
                continue
            # 1.5 px guard band: all eight neighbours at 1.5 px share the class.
            ring = [classify(x + 0.5 + dx, y + 0.5 + dy) for dx in (-1.5, 0, 1.5) for dy in (-1.5, 0, 1.5) if dx or dy]
            if all(r == centre for r in ring):
                labels.setdefault(centre, []).append((x, y))
    result = {"model": "0073 repair review: step-20 shadow and temporal-trail luma control (CPU only)",
              "light_travel_direction": [round(c, 4) for c in LIGHT],
              "window": [x0, y0, x1, y1], "region_pixels": {k: len(v) for k, v in sorted(labels.items())},
              "region_bbox": {k: [min(p[0] for p in v), min(p[1] for p in v), max(p[0] for p in v), max(p[1] for p in v)]
                              for k, v in sorted(labels.items())},
              "units": "Rec. 709 luma of 8-bit sRGB codes; mean and standard deviation per region",
              "runs": {}}
    for run, root in RUNS.items():
        result["runs"][run] = {}
        for name, file in FRAMES.items():
            rows, digest = decode_png(os.path.join(root, file))
            stats = {}
            for region, pixels in sorted(labels.items()):
                v = [0.2126 * rows[y][4 * x] + 0.7152 * rows[y][4 * x + 1] + 0.0722 * rows[y][4 * x + 2] for x, y in pixels]
                m = sum(v) / len(v)
                stats[region] = {"mean": round(m, 2), "sd": round(math.sqrt(sum((a - m) ** 2 for a in v) / len(v)), 2)}
            result["runs"][run][name] = {"png": file, "sha256": digest, "regions": stats}
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(json.dumps(result["region_pixels"]))
    for run in RUNS:
        for name in FRAMES:
            print(run, name, json.dumps(result["runs"][run][name]["regions"]))


if __name__ == "__main__":
    main()
