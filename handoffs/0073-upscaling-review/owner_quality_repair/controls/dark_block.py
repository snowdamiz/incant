#!/usr/bin/env python3
"""0073 owner_quality_repair: CPU control for the step-20 motion dark block.

Standard library only, deterministic, read-only on evidence:

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_repair/controls/dark_block.py \
        --out handoffs/0073-upscaling-review/owner_quality_repair/controls/dark_block.json

1. Geometry. Replays the review fixture's motion stage (owner_quality/review/
   upscaling_owner_review.rs: camera_world, mover_world, sun_rotation(38, 30),
   project, sign_mesh, mover_mesh) and the engine's light convention
   (lighting/scene.rs: direction = world * -Z). It projects the moving cube's
   shadow on the sign plane z = 0 into step-20 output pixels.
2. Retained evidence. Decodes the three step-20 PNGs from the accepted run
   (owner_quality/result.json) in memory and compares the mean Rec. 709 luma of
   sRGB codes inside that shadow polygon with a lit band of the same sign
   quadrant above it, for FULL, UP and OWN. No image is written; nothing is
   rendered.
"""
import argparse
import hashlib
import json
import math
import os
import struct
import zlib

EVIDENCE = os.path.expanduser("~/.codex/incant-evidence/0073-owner-quality-gpu-001/motion/artifacts")
FRAMES = {
    "full": ("motion-full.png", "9eeb1222becd6f4d2cec7afa88343ccb528ea9f6e27246cb3410a42bade35b52"),
    "up": ("motion-up50.png", "fd13d7b902ad8b7bb311a1db5282de2006c9840f3c12b894da38aace626bc073"),
    "own": ("motion-own50.png", "1c8b673b2b3cb8a757881a1a70407a40b77a875661729ee615407942b3678c53"),
}
W, H = 960, 540
FOV, PITCH = math.radians(50.0), -0.16
CAMERA_AT = (0.0, 2.4, 9.5)
MOVER_AT, MOVER_HALF = (2.02, 1.05, 1.2), 0.32
SIGN_X, SIGN_Y = (2.3, 4.5), (0.15, 2.35)
QUADRANT_X, QUADRANT_Y = (2.3, 3.4), (0.15, 1.25)  # rings, bottom-left quadrant


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


def light_direction():
    # sun_rotation(38, 30) = Ry(30) * Rx(-38); the engine shines along world * -Z.
    sun = q_mul(q_axis(1, math.radians(30.0)), q_axis(0, -math.radians(38.0)))
    d = q_rot(sun, (0.0, 0.0, -1.0))
    n = math.sqrt(sum(c * c for c in d))
    return tuple(c / n for c in d)


def project(step, p):
    """The fixture's project(): perspective_rh(50 deg, 960/540) of camera_world(step)."""
    pan = max(step - 14, 0) * 0.05
    v = (p[0] - CAMERA_AT[0] - pan, p[1] - CAMERA_AT[1], p[2] - CAMERA_AT[2])
    v = q_rot(q_axis(0, -PITCH), v)
    f = 1.0 / math.tan(FOV / 2)
    x = (f * H / W) * v[0] / -v[2]
    y = f * v[1] / -v[2]
    return ((x * 0.5 + 0.5) * W, (0.5 - y * 0.5) * H)


def cube_x(step):
    return MOVER_AT[0] + max(step - 8, 0) * 0.12


def hull(points):
    pts = sorted(set(points))
    if len(pts) < 3:
        return pts
    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def clip(poly, axis, value, keep_greater):
    """Sutherland-Hodgman clip of a convex polygon against one axis line."""
    out = []
    for i, a in enumerate(poly):
        b = poly[(i + 1) % len(poly)]
        ina = a[axis] >= value if keep_greater else a[axis] <= value
        inb = b[axis] >= value if keep_greater else b[axis] <= value
        if ina:
            out.append(a)
        if ina != inb:
            t = (value - a[axis]) / (b[axis] - a[axis])
            out.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
    return out


def shadow_on_sign(step, d):
    """Cube shadow on the plane z = 0, clipped to the sign quad (world x, y)."""
    c = cube_x(step)
    pts = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                x, y, z = c + sx * MOVER_HALF, MOVER_AT[1] + sy * MOVER_HALF, MOVER_AT[2] + sz * MOVER_HALF
                t = z / -d[2]
                pts.append((x + d[0] * t, y + d[1] * t))
    poly = hull(pts)
    for axis, value, greater in [(0, SIGN_X[0], True), (0, SIGN_X[1], False),
                                 (1, SIGN_Y[0], True), (1, SIGN_Y[1], False)]:
        poly = clip(poly, axis, value, greater) if poly else poly
    return poly


def inside(poly, x, y):
    sign = 0
    for i, a in enumerate(poly):
        b = poly[(i + 1) % len(poly)]
        c = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        if c != 0:
            if sign == 0:
                sign = 1 if c > 0 else -1
            elif (c > 0) != (sign > 0):
                return False
    return True


def decode_png(path):
    raw = open(path, "rb").read()
    assert raw[:8] == b"\x89PNG\r\n\x1a\n"
    offset, idat, header = 8, b"", None
    while offset < len(raw):
        n = struct.unpack(">I", raw[offset:offset + 4])[0]
        kind, payload = raw[offset + 4:offset + 8], raw[offset + 8:offset + 8 + n]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", payload)
        elif kind == b"IDAT":
            idat += payload
        offset += 12 + n
    assert header == (W, H, 8, 6, 0, 0, 0), header
    data, stride, rows, prev = zlib.decompress(idat), W * 4, [], bytearray(W * 4)
    for y in range(H):
        f, line = data[y * (stride + 1)], bytearray(data[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - 4] if i >= 4 else 0
            b = prev[i]
            c = prev[i - 4] if i >= 4 else 0
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


def luma(rows, x, y):
    r, g, b = rows[y][4 * x:4 * x + 3]
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def region_stats(rows, poly):
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    values = [luma(rows, x, y)
              for y in range(max(0, int(min(ys))), min(H, int(max(ys)) + 1))
              for x in range(max(0, int(min(xs))), min(W, int(max(xs)) + 1))
              if inside(poly, x + 0.5, y + 0.5)]
    mean = sum(values) / len(values)
    sd = math.sqrt(sum((v - mean) ** 2 for v in values) / len(values))
    return {"pixels": len(values), "mean_luma": round(mean, 2), "luma_sd": round(sd, 2)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    d = light_direction()
    cube20 = [project(20, (cube_x(20) + sx * MOVER_HALF, MOVER_AT[1] + sy * MOVER_HALF, MOVER_AT[2] + sz * MOVER_HALF))
              for sx in (-1, 1) for sy in (-1, 1) for sz in (-1, 1)]
    steps = {}
    for step in range(1, 21):
        poly = shadow_on_sign(step, d)
        steps[step] = [tuple(round(c, 1) for c in project(20, (x, y, 0.0))) for x, y in poly] if poly else []
    shadow20 = [project(20, (x, y, 0.0)) for x, y in shadow_on_sign(20, d)]
    # Lit comparison band: same quadrant, world y 0.65..1.05, same x span as the shadow.
    wx = [p[0] for p in shadow_on_sign(20, d)]
    lit = [project(20, (x, y, 0.0)) for x, y in
           [(min(wx), 0.65), (max(wx), 0.65), (max(wx), 1.05), (min(wx), 1.05)]]
    frames = {}
    for name, (file, expected) in FRAMES.items():
        rows, digest = decode_png(os.path.join(EVIDENCE, file))
        frames[name] = {"png": file, "sha256": digest, "sha256_matches_result_json": digest == expected,
                        "shadow_polygon": region_stats(rows, shadow20), "lit_band_above": region_stats(rows, lit)}
    result = {
        "model": "0073 owner_quality_repair dark-block geometry and retained-PNG control (CPU only)",
        "light_travel_direction": [round(c, 4) for c in d],
        "cube_step20_screen_bbox": [round(min(p[0] for p in cube20), 1), round(min(p[1] for p in cube20), 1),
                                    round(max(p[0] for p in cube20), 1), round(max(p[1] for p in cube20), 1)],
        "recorded_dark_block_bbox": [620, 290, 655, 312],
        "cube_shadow_on_sign_step20_screen_polygon": [[round(c, 1) for c in p] for p in shadow20],
        "cube_shadow_on_sign_by_step_in_step20_pixels": steps,
        "lit_band_polygon": [[round(c, 1) for c in p] for p in lit],
        "frames_step20": frames,
        "scope": "geometry replay plus in-memory decode of three retained PNGs; no render, GPU or new image",
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(json.dumps({k: result[k] for k in ["cube_step20_screen_bbox", "recorded_dark_block_bbox",
                                              "cube_shadow_on_sign_step20_screen_polygon"]}))
    for name, f in frames.items():
        print(name, f["sha256_matches_result_json"], "shadow", f["shadow_polygon"], "lit", f["lit_band_above"])


if __name__ == "__main__":
    main()
