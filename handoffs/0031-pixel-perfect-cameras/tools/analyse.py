#!/usr/bin/env python3
"""Measures pixel_lookdev.py output. Pure standard library; reads PNGs only.

Usage (repo root):
  python3 -I handoffs/0031-pixel-perfect-cameras/tools/analyse.py RUN_DIR OUT_JSON [OTHER_RUN_DIR]

OUT_JSON must not exist. With OTHER_RUN_DIR, every screenshot and live frame is
also compared byte for byte with the other from-scratch run.
"""

import hashlib
import json
import math
import struct
import sys
import zlib
from collections import Counter
from pathlib import Path

LETTERBOX = (20, 21, 25, 255)  # the renderer's neutral backdrop


def read_png(path):
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, width = 8, b"", None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, ctype = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and ctype == 6, (depth, ctype)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride, bpp = width * 4, 4
    rows, prev = [], bytearray(stride)
    for y in range(height):
        f = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        if f == 1:
            for i in range(bpp, stride):
                line[i] = (line[i] + line[i - bpp]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                line[i] = (line[i] + (a + prev[i]) // 2) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - bpp] if i >= bpp else 0
                b = prev[i]
                c = prev[i - bpp] if i >= bpp else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(bytes(line))
        prev = line
    return width, height, rows


def px(rows, x, y):
    return tuple(rows[y][x * 4:x * 4 + 4])


def colours(rows):
    counts = Counter()
    for row in rows:
        for i in range(0, len(row), 4):
            counts[row[i:i + 4]] += 1
    return counts


def fit(w, h, lw, lh):
    s = min(w // lw, h // lh)
    return s, (w - lw * s) // 2, (h - lh * s) // 2


def replication(img, logical):
    """Exact check against nearest integer replication of `logical` with centred letterbox."""
    w, h, rows = img
    lw, lh, lrows = logical
    s, x0, y0 = fit(w, h, lw, lh)
    lb = bytes(LETTERBOX)
    inside_bad = outside_bad = 0
    for y in range(h):
        if y0 <= y < y0 + lh * s:
            src = lrows[(y - y0) // s]
            expected = lb * x0 + b"".join(src[i:i + 4] * s for i in range(0, lw * 4, 4)) + lb * (w - x0 - lw * s)
        else:
            expected = lb * w
        if rows[y] != expected:
            for x in range(w):
                if rows[y][x * 4:x * 4 + 4] != expected[x * 4:x * 4 + 4]:
                    if x0 <= x < x0 + lw * s and y0 <= y < y0 + lh * s:
                        inside_bad += 1
                    else:
                        outside_bad += 1
    return {"scale": s, "x0": x0, "y0": y0, "right": w - x0 - lw * s, "bottom": h - y0 - lh * s,
            "mismatched_inside": inside_bad, "mismatched_letterbox": outside_bad}


def bands(img):
    """Letterbox widths measured from the image alone: whole rows/columns of backdrop colour."""
    w, h, rows = img
    lb = bytes(LETTERBOX)
    full_row = [rows[y] == lb * w for y in range(h)]
    col = [all(rows[y][x * 4:x * 4 + 4] == lb for y in range(h)) for x in range(w)]

    def lead(flags):
        n = 0
        while n < len(flags) and flags[n]:
            n += 1
        return n
    return {"left": lead(col), "right": lead(col[::-1]), "top": lead(full_row), "bottom": lead(full_row[::-1])}


def crop(img, x0, y0, cw, ch):
    _, _, rows = img
    return cw, ch, [rows[y][x0 * 4:(x0 + cw) * 4] for y in range(y0, y0 + ch)]


def best_shift(a, b, margin=4, exclude=None, search=(3, 3)):
    """Integer (dx, dy) such that b[x][y] == a[x - dx][y - dy] over the interior.

    Positive dx means content moved right, positive dy down. `exclude` is a screen
    rectangle (x0, x1, y0, y1) ignored wherever it lies in either frame (the mover).
    """
    w, h, ar = a
    _, _, br = b

    def segments(y, dx, dy):
        cuts = []
        if exclude:
            x0, x1, y0, y1 = exclude
            if y0 <= y < y1:
                cuts.append((x0, x1))
            if y0 <= y - dy < y1:
                cuts.append((x0 + dx, x1 + dx))
        segs, start = [], margin
        for c0, c1 in sorted(cuts):
            if c0 > start:
                segs.append((start, min(c0, w - margin)))
            start = max(start, c1)
        if start < w - margin:
            segs.append((start, w - margin))
        return segs

    def row_equal(y, dx, dy):
        return all(br[y][x0 * 4:x1 * 4] == ar[y - dy][(x0 - dx) * 4:(x1 - dx) * 4] for x0, x1 in segments(y, dx, dy))

    ys = range(margin, h - margin)
    best = None
    for dy in range(-search[1], search[1] + 1):
        for dx in range(-search[0], search[0] + 1):
            bad_rows = sum(1 for y in ys if not row_equal(y, dx, dy))
            if best is None or bad_rows < best[0]:
                best = (bad_rows, dx, dy)
    _, dx, dy = best
    bad = compared = 0
    for y in ys:
        for x0, x1 in segments(y, dx, dy):
            compared += x1 - x0
            if br[y][x0 * 4:x1 * 4] != ar[y - dy][(x0 - dx) * 4:(x1 - dx) * 4]:
                for x in range(x0, x1):
                    bad += br[y][x * 4:x * 4 + 4] != ar[y - dy][(x - dx) * 4:(x - dx) * 4 + 4]
    return {"dx": dx, "dy": dy, "mismatched": bad, "compared": compared}


def runs_along(values):
    out, cur, n = [], None, 0
    for v in values:
        if v == cur:
            n += 1
        else:
            if cur is not None:
                out.append((cur, n))
            cur, n = v, 1
    out.append((cur, n))
    return out


def gauges(img, bg):
    """Run lengths of non-background pixels across the bar gauges, checkers and frame ring."""
    w, h, rows = img
    vertical = [px(rows, x, 20) != bg for x in range(12, 107)]
    horizontal = [px(rows, 20, y) != bg for y in range(44, 133)]
    lit_v = [n for on, n in runs_along(vertical) if on]
    gap_v = [n for on, n in runs_along(vertical) if not on]
    lit_h = [n for on, n in runs_along(horizontal) if on]
    gap_h = [n for on, n in runs_along(horizontal) if not on]
    checker_ok = {}
    for n, ox in ((1, 120), (2, 140), (4, 160)):
        good = 0
        for j in range(16):
            for i in range(16):
                want = ((i // n) + (j // n)) % 2 == 0
                good += (px(rows, ox + i, 10 + j) != bg) == want
        checker_ok[f"{n}px"] = f"{good}/256"
    ring = [px(rows, x, 0) for x in range(w)] + [px(rows, x, h - 1) for x in range(w)] + \
        [px(rows, 0, y) for y in range(h)] + [px(rows, w - 1, y) for y in range(h)]
    corners = {"top_left": px(rows, 4, 4), "top_right": px(rows, w - 5, 4),
               "bottom_left": px(rows, 4, h - 5), "bottom_right": px(rows, w - 5, h - 5)}
    return {"vertical_bars": lit_v, "vertical_gaps": gap_v, "horizontal_bars": lit_h, "horizontal_gaps": gap_h,
            "checkers_correct": checker_ok, "frame_ring_colours": dict(Counter(map(str, ring))), "corners": corners}


def mover_box(img, green, region=(24, 64, 140, 170)):
    """Screen rectangle (x0, x1, y0, y1) of the mover's colour near its home position."""
    w, h, rows = img
    xs, ys = [], []
    for y in range(region[2], region[3]):
        for x in range(region[0], region[1]):
            if px(rows, x, y) == green:
                xs.append(x)
                ys.append(y)
    return [min(xs), max(xs) + 1, min(ys), max(ys) + 1] if xs else None


def snap(v):
    """Nearest integer, ties away from zero: the rule the static tie cameras measure."""
    return int(math.copysign(math.floor(abs(v) + 0.5), v))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    run, out = Path(sys.argv[1]), Path(sys.argv[2])
    other = Path(sys.argv[3]) if len(sys.argv) > 3 else None
    if out.exists():
        raise SystemExit("Output exists; choose a new file.")
    scene = json.loads((run / "scene.json").read_text())
    lw, lh = scene["logical"]
    cache = {}

    def shot(name):
        if name not in cache:
            cache[name] = read_png(run / "shots" / f"{name}-r1.png")
        return cache[name]

    report = {"run": str(run)}
    base = shot("base-320x180")
    hist = colours(base[2])
    report["palette_1x"] = {str(tuple(k)): v for k, v in hist.most_common()}
    bg = px(base[2], 60, 40)
    green = px(base[2], 44, 155)
    report["background"], report["mover_colour"] = bg, green
    report["gauges_1x"] = gauges(base, bg)

    # Replication and letterboxing for the base camera.
    rep = {}
    for p in sorted((run / "shots").glob("base-*-r1.png")):
        name = p.name[:-7]
        img = shot(name)
        r = replication(img, base)
        r["measured_bands"] = bands(img)
        r["distinct_colours"] = len(colours(img[2]))
        rep[name] = r
    report["base_replication"] = rep

    # Other pixel-perfect configurations: logical frame cut from their 1x capture.
    configs = {}
    for cam, (cw, ch), native in (("ppu16-160x90", (160, 90), "ppu16-160x90-160x90"),
                                  ("ppu32-256x224", (256, 224), "ppu32-256x224-256x224"),
                                  ("ppu32-255x143", (255, 143), "ppu32-255x143-320x180")):
        img = shot(native)
        s, x0, y0 = fit(img[0], img[1], cw, ch)
        assert s == 1
        logical = crop(img, x0, y0, cw, ch)
        entry = {}
        for p in sorted((run / "shots").glob(f"{cam}-*-r1.png")):
            name = p.name[:-7]
            r = replication(shot(name), logical)
            r["measured_bands"] = bands(shot(name))
            entry[name] = r
        configs[cam] = entry
    report["other_configs"] = configs

    # Camera motion at 1x relative to the base camera. Expected: content moves by
    # -round(dx_px) horizontally and +round(dy_px) vertically (screen y points down).
    motion = {}
    for cam in ("x+0.25", "x+0.49", "x-0.49", "x+0.50", "x-0.50", "x+0.51", "x+1.00", "x+1.49", "y+0.49",
                "y+0.50", "y-0.50", "y-0.51", "xy+2.3", "depth+3.3"):
        img = shot(f"{cam}-320x180")
        m = best_shift(base, img)
        m["identical_to_base"] = img[2] == base[2]
        m["hash_equal_to_base"] = sha(run / "shots" / f"{cam}-320x180-r1.png") == sha(run / "shots" / "base-320x180-r1.png")
        r3 = replication(shot(f"{cam}-960x540"), img)
        m["3x_replicates_its_1x"] = r3["mismatched_inside"] == 0 and r3["mismatched_letterbox"] == 0
        motion[cam] = m
    report["motion_1x"] = motion

    # Roll: a directly rolled camera and the same roll inherited from a parent rig.
    roll90 = shot("roll90-320x180")
    rig = shot("rig90-base-320x180")
    report["roll"] = {
        "roll90_equals_rig90_base": roll90[2] == rig[2],
        "rig90+0.3_vs_rig90_base": best_shift(rig, shot("rig90+0.3-320x180")),
        "rig90+0.7_vs_rig90_base": best_shift(rig, shot("rig90+0.7-320x180")),
        "roll10_distinct_colours_1x": len(colours(shot("roll10-320x180")[2])),
        "roll10_3x_replicates_its_1x": replication(shot("roll10-960x540"), shot("roll10-320x180")),
    }

    # Ordinary projections at the same pose: retained and unaffected.
    def diff(a, b):
        return sum(1 for y in range(a[1]) for x in range(a[0]) if a[2][y][x * 4:x * 4 + 4] != b[2][y][x * 4:x * 4 + 4])
    report["ordinary_projections"] = {
        "ortho_320x180_vs_pixel_320x180_mismatched": diff(shot("ortho-320x180"), base),
        "ortho_960x540_vs_pixel_960x540_mismatched": diff(shot("ortho-960x540"), shot("base-960x540")),
        "ortho_960x540_distinct_colours": len(colours(shot("ortho-960x540")[2])),
        "pixel_960x540_distinct_colours": len(colours(shot("base-960x540")[2])),
        "ortho_960x540_bands": bands(shot("ortho-960x540")),
        "persp_960x540_bands": bands(shot("persp-960x540")),
        "persp_vs_pixel_960x540_mismatched": diff(shot("persp-960x540"), shot("base-960x540")),
    }

    # Live sequences. Frame N is the state after N ticks: camera and mover at
    # direction * N * step. Pixel frames are compared with frame 0 shifted by the
    # snapped camera; the mover is masked there and measured on its own.
    mover_home = mover_box(base, green)
    mask = (mover_home[0] - 4, mover_home[1] + 4, mover_home[2] - 4, mover_home[3] + 4)
    live = {"mover_home": mover_home}
    ppu, step = scene["ppu"], scene["live"]["step"]
    for key, spec in scene["live"]["runs"].items():
        script = scene["live"]["scripts"][spec["script"]]
        ux, uy = script["direction"]
        switches = {int(t): p for t, p in script["switches"].items()}
        frames = sorted((run / f"live-{key}-r1").glob("frame-*.png"))
        if spec["size"] != [lw, lh]:
            continue
        f0 = read_png(frames[0])
        rows_out, summary = [], {"pixel_frames": 0, "pixel_frames_exact": 0, "mover_offsets": Counter()}
        for p in frames:
            tick = int(p.stem.split("-")[1])
            kinds = [v for t, v in sorted(switches.items()) if t < tick]
            kind = (kinds[-1] or {"kind": "perspective"})["kind"] if kinds else "pixel_perfect"
            img = read_png(p)
            cx, cy = ux * tick * step * ppu, uy * tick * step * ppu
            expect = (-snap(cx), snap(cy))
            entry = {"tick": tick, "projection": kind, "camera_px": [cx, cy], "expected_shift": expect,
                     "mover": mover_box(img, green)}
            if kind != "perspective":
                entry.update(best_shift(f0, img, margin=16, exclude=mask, search=(14, 8)))
            if kind == "pixel_perfect":
                summary["pixel_frames"] += 1
                summary["pixel_frames_exact"] += (entry["dx"], entry["dy"]) == expect and entry["mismatched"] == 0
                if entry["mover"]:
                    summary["mover_offsets"][f"{entry['mover'][0] - mover_home[0]},{entry['mover'][2] - mover_home[2]}"] += 1
            rows_out.append(entry)
        summary["mover_offsets"] = dict(summary["mover_offsets"])
        summary["mover_moved_ticks"] = [e["tick"] for e in rows_out if e["projection"] == "pixel_perfect"
                                        and e["mover"] and e["mover"][:3:2] != mover_home[:3:2]]
        live[key] = {"summary": summary, "frames": rows_out}
    live["frame0_equals_static_base"] = read_png(run / "live-1x-r1" / "frame-000000.png")[2] == base[2]
    static_1 = shot("x+1.00-320x180")
    live["x_frames_equal_to_static_x+1.00_outside_mover"] = [
        p.name for p in sorted((run / "live-1x-r1").glob("frame-*.png"))
        if best_shift(static_1, read_png(p), exclude=mask, search=(0, 0))["mismatched"] == 0]
    # 3x odd run: replication of the matching 1x frame for every tick both runs captured.
    odd = []
    for p in sorted((run / "live-3x-odd-r1").glob("frame-*.png")):
        img = read_png(p)
        tick = int(p.stem.split("-")[1])
        one = run / "live-1x-r1" / f"frame-{tick:06d}.png"
        entry = {"tick": tick, "bands": bands(img)}
        if one.exists():
            r = replication(img, read_png(one))
            entry.update({"replicates_1x_frame": r["mismatched_inside"] == 0 and r["mismatched_letterbox"] == 0,
                          "scale": r["scale"], "x0": r["x0"], "y0": r["y0"], "right": r["right"], "bottom": r["bottom"]})
        odd.append(entry)
    live["frames_3x_odd"] = odd
    report["live"] = live

    if other:
        same = {}
        for p in sorted((run / "shots").glob("*.png")):
            q = other / "shots" / p.name
            same[p.name] = q.exists() and sha(p) == sha(q)
        for key in sorted(d.name for d in run.glob("live-*-r1") if d.is_dir()):
            for p in sorted((run / key).glob("*.png")):
                q = other / key / p.name
                same[f"{key}/{p.name}"] = q.exists() and sha(p) == sha(q)
        report["from_scratch_comparison"] = {"other": str(other), "compared": len(same),
                                             "equal": sum(same.values()),
                                             "different": [k for k, v in same.items() if not v]}
    out.write_text(json.dumps(report, indent=1, default=list), encoding="utf-8")
    print(json.dumps({k: report[k] for k in ("background", "base_replication")}, default=list)[:3000])


if __name__ == "__main__":
    main()
