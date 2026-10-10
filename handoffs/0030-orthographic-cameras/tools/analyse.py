#!/usr/bin/env python3
"""Measure ortho_lookdev.py output. Stdlib only; reads PNGs, never writes into RUN_DIR.

For each screenshot:
  * pillar silhouettes, segmented by hue (red / teal / amber), as pixel bounding
    boxes, compared with the analytic projection of each pillar's 8 corners;
  * the two rails (blue) fitted as straight lines by principal axis, giving each
    rail's image angle, the angle between the rails and the RMS distance of rail
    pixels from the fitted line (straightness);
and between pairs of frames, the count and maximum of differing RGB channels.

Usage: python3 -I analyse.py RUN_DIR OUT_JSON   (OUT_JSON must not exist)
"""

import json
import math
import struct
import sys
import zlib
from pathlib import Path


def read_png(path):
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, width = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and ctype in (2, 6), (depth, ctype)
            channels = 4 if ctype == 6 else 3
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - channels] if i >= channels else 0
            b = prev[i]
            c = prev[i - channels] if i >= channels else 0
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
    return width, height, channels, rows


def classify(r, g, b):
    mx, mn = max(r, g, b), min(r, g, b)
    if mx < 40 or mx - mn < 0.4 * mx:
        return None
    if b == mx and b > r + 40 and b > g + 20:
        return "rail"
    if r == mx and g < 0.55 * r and b < 0.5 * r:
        return "red"
    if g >= r and b > 0.6 * g and r < 0.6 * g:
        return "teal"
    if r == mx and 0.45 * r <= g <= 0.9 * r and b < 0.62 * r:
        return "amber"
    return None


def measure(path):
    w, h, ch, rows = read_png(path)
    boxes, rail = {}, []
    for y, line in enumerate(rows):
        for x in range(w):
            k = classify(line[x * ch], line[x * ch + 1], line[x * ch + 2])
            if k is None:
                continue
            if k == "rail":
                rail.append((x, y))
                continue
            bx = boxes.setdefault(k, [x, y, x, y, 0])
            bx[0], bx[1], bx[2], bx[3], bx[4] = min(bx[0], x), min(bx[1], y), max(bx[2], x), max(bx[3], y), bx[4] + 1
    return w, h, rows, ch, {k: {"x0": v[0], "y0": v[1], "x1": v[2], "y1": v[3], "pixels": v[4]} for k, v in boxes.items()}, rail


def fit(points):
    n = len(points)
    mx = sum(p[0] for p in points) / n
    my = sum(p[1] for p in points) / n
    sxx = sum((p[0] - mx) ** 2 for p in points) / n
    syy = sum((p[1] - my) ** 2 for p in points) / n
    sxy = sum((p[0] - mx) * (p[1] - my) for p in points) / n
    theta = 0.5 * math.atan2(2 * sxy, sxx - syy)
    nx, ny = -math.sin(theta), math.cos(theta)
    rms = math.sqrt(sum(((p[0] - mx) * nx + (p[1] - my) * ny) ** 2 for p in points) / n)
    return math.degrees(theta), rms, n


def split_rails(points, scene, cam, w, h):
    """Assign each rail pixel to the nearer analytic rail centre line."""
    lines = []
    for x in scene["rails"]:
        a = project(scene, cam, (x, 0.07, scene["rail_z"][0]), w, h)
        b = project(scene, cam, (x, 0.07, scene["rail_z"][1]), w, h)
        lines.append((a, b))
    groups = [[], []]
    for p in points:
        d = []
        for (a, b) in lines:
            dx, dy = b[0] - a[0], b[1] - a[1]
            d.append(abs((p[0] - a[0]) * dy - (p[1] - a[1]) * dx) / math.hypot(dx, dy))
        groups[0 if d[0] < d[1] else 1].append(p)
    return groups


def rotate(q, v):
    x, y, z, w = q
    ux, uy, uz = v
    tx, ty, tz = 2 * (y * uz - z * uy), 2 * (z * ux - x * uz), 2 * (x * uy - y * ux)
    return (ux + w * tx + y * tz - z * ty, uy + w * ty + z * tx - x * tz, uz + w * tz + x * ty - y * tx)


def project(scene, cam, p, w, h):
    q = scene["rotation"]
    eye = cam["eye"]
    rel = [p[i] - eye[i] for i in range(3)]
    right, up, back = rotate(q, (1, 0, 0)), rotate(q, (0, 1, 0)), rotate(q, (0, 0, 1))
    x = sum(rel[i] * right[i] for i in range(3))
    y = sum(rel[i] * up[i] for i in range(3))
    depth = -sum(rel[i] * back[i] for i in range(3))
    proj = cam["projection"]
    if proj and proj["kind"] == "orthographic":
        half_h = proj["vertical_size"] / 2
        nx, ny = x / (half_h * w / h), y / half_h
    else:
        t = math.tan(math.radians(scene["fov"]) / 2)
        nx, ny = x / (depth * t * w / h), y / (depth * t)
    return ((nx + 1) / 2 * w, (1 - ny) / 2 * h)


def predicted_box(scene, cam, name, w, h):
    (px, pz), _ = scene["pillars"][name]
    sx, sy, sz = scene["pillar"]
    pts = [project(scene, cam, (px + dx * sx / 2, dy * sy, pz + dz * sz / 2), w, h)
           for dx in (-1, 1) for dy in (0, 1) for dz in (-1, 1)]
    return {"x0": min(p[0] for p in pts), "y0": min(p[1] for p in pts),
            "x1": max(p[0] for p in pts), "y1": max(p[1] for p in pts)}


def diff(a, b):
    wa, ha, ca, ra = read_png(a)
    wb, hb, cb, rb = read_png(b)
    assert (wa, ha) == (wb, hb)
    pixels, maximum, total = 0, 0, 0
    for la, lb in zip(ra, rb):
        if la == lb:
            continue
        for x in range(wa):
            d = max(abs(la[x * ca + i] - lb[x * cb + i]) for i in range(3))
            if d:
                pixels += 1
                total += d
                maximum = max(maximum, d)
    return {"pixels": pixels, "fraction": round(pixels / (wa * ha), 6), "max_channel": maximum,
            "mean_channel_over_changed": round(total / pixels, 3) if pixels else 0}


def main():
    run, out = Path(sys.argv[1]), Path(sys.argv[2])
    if out.exists():
        raise SystemExit(f"{out} exists")
    scene = json.loads((run / "scene.json").read_text(encoding="utf-8"))
    sizes = {"16x9": (960, 540), "4x3": (800, 600)}
    shots = {}
    for name, cam in scene["cameras"].items():
        for size, (w, h) in sizes.items():
            path = run / "shots" / f"{name}-{size}-r1.png"
            pw, ph, _, _, boxes, rail = measure(path)
            assert (pw, ph) == (w, h)
            pillars = {}
            for pname, (_, hue) in scene["pillars"].items():
                m = boxes.get(hue)
                p = predicted_box(scene, cam, pname, w, h)
                pillars[pname] = {"measured": m, "predicted": {k: round(v, 2) for k, v in p.items()},
                                  "measured_height_px": (m["y1"] - m["y0"] + 1) if m else None,
                                  "predicted_height_px": round(p["y1"] - p["y0"], 2),
                                  "max_edge_error_px": round(max(abs(m["x0"] - p["x0"]), abs(m["x1"] + 1 - p["x1"]),
                                                                 abs(m["y0"] - p["y0"]), abs(m["y1"] + 1 - p["y1"])), 2)
                                  if m and m["x0"] > 0 and m["y0"] > 0 and m["x1"] < w - 1 and m["y1"] < h - 1 else None}
            rails = []
            for group in split_rails(rail, scene, cam, w, h) if rail else []:
                if len(group) > 50:
                    angle, rms, n = fit(group)
                    rails.append({"angle_deg": round(angle, 3), "rms_px": round(rms, 3), "pixels": n})
            shots[f"{name}-{size}"] = {"pillars": pillars, "rails": rails,
                                       "rail_angle_between_deg": round(abs(rails[0]["angle_deg"] - rails[1]["angle_deg"]), 3)
                                       if len(rails) == 2 else None}
    pairs = {}
    for size in sizes:
        for a, b in (("ortho-d16-v9", "ortho-d24-v9"), ("ortho-d16-v9", "ortho-d34-v9"), ("ortho-d24-v9", "ortho-d34-v9"),
                     ("persp-d16", "ortho-d16-v9")):
            pairs[f"{a}|{b}|{size}"] = diff(run / "shots" / f"{a}-{size}-r1.png", run / "shots" / f"{b}-{size}-r1.png")
    live = {}
    for size in sizes:
        frames = sorted((run / f"live-{size}-r1").glob("*.png"))
        live[size] = [f.name for f in frames]
        # Live frames against the static camera with the same pose and projection.
        refs = {"persp-d16": 5, "ortho-d16-v9": 15, "ortho-d24-v9": 25}
        for ref, tick in refs.items():
            match = [f for f in frames if f.stem == f"frame-{tick:06d}"]
            if match:
                pairs[f"live@{match[0].name}|{ref}|{size}"] = diff(match[0], run / "shots" / f"{ref}-{size}-r1.png")
    out.write_text(json.dumps({"shots": shots, "pairs": pairs, "live_frames": live}, indent=1) + "\n", encoding="utf-8")
    print(out)


if __name__ == "__main__":
    main()
