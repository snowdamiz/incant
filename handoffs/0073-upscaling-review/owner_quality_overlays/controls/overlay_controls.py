#!/usr/bin/env python3
"""0073 owner_quality_overlays: independent CPU controls, standard library only.

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_overlays/controls/overlay_controls.py \
        --out handoffs/0073-upscaling-review/owner_quality_overlays/controls/overlay_controls.json

Not a renderer, test binary or GPU run. Four groups, each an assertion:
1. output viewport and unjittered projection at internal 50/75% and odd DRS scales;
2. draw order, depth and premultiplied alpha of the moved composite;
3. routing table (pixel-camera bypass and default path unchanged);
4. exact admission arithmetic, overflow and the unchanged 671105024 B ceiling.
"""
import argparse
import json
import math
import random
import struct
import sys

sys.dont_write_bytecode = True
CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


# 1. Output viewport / unjittered projection -------------------------------------------

def perspective(aspect, fov=math.radians(50.0), near=0.1, far=120.0):
    f = 1.0 / math.tan(fov / 2)
    return (f / aspect, f, far / (near - far), near * far / (near - far))


def project(m, point):
    """Clip -> NDC for a view-space point (x, y, z<0): standard depth [0, 1]."""
    x, y, z = point
    cx, cy, cz, cw = m[0] * x, m[1] * y, m[2] * z + m[3], -z
    return cx / cw, cy / cw, cz / cw


def to_pixel(ndc, rect):
    return (rect[0] + (ndc[0] * 0.5 + 0.5) * rect[2], rect[1] + (0.5 - ndc[1] * 0.5) * rect[3])


def viewport_controls():
    rects = {"fixture_viewport": [37.5, 21.25, 885.0, 497.5], "kernel_rect": [3.25, 5.5, 65.0, 33.0],
             "full_960x540": [0.0, 0.0, 960.0, 540.0]}
    rows = {}
    for name, rect in rects.items():
        for n in (5, 7, 8, 9, 11, 12, 13, 15):
            s = f32(n / 16)
            source = [f32(f32(v) * s) for v in rect]
            out_aspect = f32(f32(rect[2]) / f32(rect[3]))
            in_aspect = f32(source[2] / source[3])
            m_out, m_in = perspective(out_aspect), perspective(in_aspect)
            worst = 0.0
            for p in ((-1.3, 0.7, -4.0), (2.2, -1.1, -9.5), (0.05, 0.5, -5.6), (-6.0, 3.9, -12.0)):
                a = to_pixel(project(m_out, p), rect)            # sprites/text and output depth re-raster
                b = to_pixel(project(m_in, p), source)           # internal unjittered overlay pass
                back = (rect[0] + (b[0] - source[0]) / s, rect[1] + (b[1] - source[1]) / s)
                worst = max(worst, abs(a[0] - back[0]), abs(a[1] - back[1]))
                da, db = project(m_out, p)[2], project(m_in, p)[2]
                worst = max(worst, abs(da - db) * 1e3)
            rows[f"{name}@{n}/16"] = {"aspect_out": out_aspect, "aspect_in": in_aspect,
                                       "max_output_pixel_or_kilo_depth_disagreement": worst}
            check(f"{name} at {n}/16: output-viewport projection equals the internal unjittered projection scaled",
                  worst < 1e-3, worst)
    # Unjittered: the output frame never carries the internal jitter (jitter.rs SAMPLES, internal pixels).
    for jitter in ((-0.375, -0.125), (0.375, 0.125)):
        rect, n = rects["fixture_viewport"], 8
        source_w = rect[2] * n / 16
        shift_internal_ndc = 2 * jitter[0] / source_w
        check("internal jitter would move an output-resolution overlay by a visible fraction of a pixel",
              abs(shift_internal_ndc * 0.5 * rect[2]) >= 0.25, shift_internal_ndc * 0.5 * rect[2])
    return rows


# 2. Draw order, depth, premultiplied alpha ---------------------------------------------

def over(src, dst):
    """PREMULTIPLIED_ALPHA_BLENDING: (One, OneMinusSrcAlpha) for colour and alpha."""
    return tuple(s + d * (1 - src[3]) for s, d in zip(src, dst))


def alpha_blend(src_straight, dst):
    """BLEND material pipeline ALPHA_BLENDING: colour (SrcAlpha, 1-SrcAlpha), alpha (One, 1-SrcAlpha)."""
    a = src_straight[3]
    return tuple([src_straight[i] * a + dst[i] * (1 - a) for i in range(3)] + [a + dst[3] * (1 - a)])


def clean(v):
    if v[3] <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    return tuple([min(max(c, 0.0), 65504.0) for c in v[:3]] + [min(max(v[3], 0.0), 1.0)])


def order_controls():
    rng = random.Random(73)
    worst = 0.0
    for _ in range(2000):
        resolved = (rng.random(), rng.random(), rng.random(), rng.choice([1.0, 0.0, rng.random()]))
        resolved = clean(tuple(c * resolved[3] for c in resolved[:3]) + (resolved[3],))
        blend = [(rng.random(), rng.random(), rng.random(), rng.random()) for _ in range(rng.randint(0, 3))]
        sprites = [(lambda a: (rng.random() * a, rng.random() * a, rng.random() * a, a))(rng.random())
                   for _ in range(rng.randint(0, 3))]
        texts = [(lambda a: (0.9 * a, 0.9 * a, 0.9 * a, a))(rng.random()) for _ in range(rng.randint(0, 3))]
        # ce617: every overlay into the cleared internal overlay, then over the resolved output.
        t = (0.0, 0.0, 0.0, 0.0)
        for b in blend:
            t = alpha_blend(b, t)
        old = t
        for s in sprites + texts:
            old = over(s, old)
        old = clean(over(clean(old), resolved))
        # This patch: composite = BLEND overlay over resolved, then sprites, then text at output resolution.
        new = clean(over(clean(t), resolved))
        for s in sprites + texts:
            new = over(s, new)
        worst = max(worst, max(abs(a - b) for a, b in zip(old, new)))
    check("moved composite: sprites/text over (BLEND over resolved) equals the old single-overlay order",
          worst < 1e-12, worst)
    # Depth: sprites and text use LessEqual without depth writes against the re-raster.
    surface = 0.62
    cases = {"in_front": (0.40, True), "behind": (0.80, False), "coplanar_equal": (0.62, True)}
    for name, (z, visible) in cases.items():
        check(f"output depth LessEqual: {name}", (z <= surface) == visible)
    return {"max_order_difference": worst}


# 3. Routing table ------------------------------------------------------------------------

def routing():
    """Mirror of lib.rs draw_scene_scaled target selection for sprites/text and depth passes."""
    rows = []
    for upscaling in ("none", "bypassed_pixel_camera", "portable_plan"):
        for transparent in (False, True):
            for screen_overlays in (False, True):
                screen = upscaling == "portable_plan"
                target = "output composite + output depth + output rect" if screen else \
                    "unchanged ce617 target (internal colour/overlay, scene depth, scene rect)"
                accumulating = upscaling == "portable_plan"  # fixed temporal mirrors the 'none' row
                internal_depth = accumulating and (transparent or (screen_overlays and not screen))
                output_depth = screen and screen_overlays
                rows.append({"upscaling": upscaling, "transparent": transparent,
                             "sprites_or_text": screen_overlays, "sprites_text_target": target,
                             "internal_overlay_depth_pass": internal_depth, "output_depth_pass": output_depth})
    check("bypass and inactive upscaling keep the ce617 sprite/text targets",
          all(r["sprites_text_target"].startswith("unchanged") for r in rows if r["upscaling"] != "portable_plan"))
    check("output depth pass exists only with a portable plan and sprites/text",
          all(r["output_depth_pass"] == (r["upscaling"] == "portable_plan" and r["sprites_or_text"]) for r in rows))
    return rows


# 4. Admission ---------------------------------------------------------------------------

P = 4 * 1024 * 1024
INST = 64 * 1024 * 1024
CEILING = 128 * P + 2 * INST + 16384


def pixels(s, limit=8192):
    if 0 in s or any(d > min(limit, 8192) for d in s):
        raise ValueError("size")
    n = s[0] * s[1]
    if n > P:
        raise ValueError("pixels")
    return n


def overlap(old, cand, cap):
    if old > cap or cand > cap or old + cand > 2 * cap:
        raise ValueError("overlap")


def flagged(old, nxt, stride, optional, cap):
    size = lambda c: pixels(c[0]) * (0 if optional and not c[1] else stride)
    cur, prev = size(nxt), (size(old) if old else 0)
    overlap(prev, cur, cap)
    return prev + (0 if old == nxt else cur)


def component(old, nxt, stride, cap):
    cand, prev = pixels(nxt) * stride, (pixels(old) * stride if old else 0)
    overlap(prev, cand, cap)
    return prev + (0 if old == nxt else cand)


def admit_records(source, old_motion, output, old_output, old_overlay, out=36):
    s, o = (source, True), (output, True)
    total = 2 * INST + 16384 + flagged(old_motion, s, 24, False, 24 * P) + flagged(old_motion, s, 4, True, 4 * P) \
        + flagged(old_output, o, out, False, out * P) + flagged(old_output, o, 16, True, 16 * P) \
        + component(old_overlay, source, 8, 8 * P)
    if total > CEILING:
        raise ValueError("ceiling")
    return total


def preflight(output, source, old_output, old_overlay, old_motion, out=36):
    o, s = pixels(output), pixels(source)
    if old_output > out * P or old_overlay > 8 * P or old_motion > 24 * P:
        raise ValueError("old")
    total = old_output + old_overlay + old_motion + out * o + 32 * s + 2 * INST + 16384
    if total > CEILING:
        raise ValueError("ceiling")
    return total


def attempt(f, *a, **k):
    try:
        return f(*a, **k)
    except ValueError as e:
        return f"refused:{e}"


def admission():
    old = lambda s: (s, True)
    out = {}
    expected = {((2048, 2048), (2048, 2048), (2048, 2048), (2048, 2048)): 503_332_864,
                ((1920, 1920), (2048, 2048), (2048, 2048), (2048, 2048)): 636_043_264,
                ((2560, 1440), (1920, 1080), (2560, 1440), (1920, 1080)): 641_114_112,
                ((1280, 800), (1280, 720), (2560, 1600), (2560, 1440)): 608_960_512}
    for (src, psrc, outp, pout), want in expected.items():
        got = admit_records(list(src), old(list(psrc)), list(outp), old(list(pout)), list(psrc))
        out[f"records {src}<-{psrc} out {outp}<-{pout}"] = got
        check(f"admit_records {src}: exact patched total", got == want, got)
        check(f"admit_records {src}: ce617 stride 32 reproduces the ce617 test value",
              admit_records(list(src), old(list(psrc)), list(outp), old(list(pout)), list(psrc), 32)
              == {503_332_864: 486_555_648, 636_043_264: 619_266_048, 641_114_112: 618_074_112,
                  608_960_512: 577_830_912}[want])
    check("fresh 2048x1920 records admission", admit_records([2048, 1920], None, [2048, 1920], None, None) == 480_264_192)
    check("2048x1920 after 2048^2 still refused",
          str(attempt(admit_records, [2048, 1920], old([2048, 2048]), [2048, 1920], old([2048, 2048]), [2048, 2048]))
          .startswith("refused"))
    check("full-scale 2048^2 after a maximal old set is refused",
          str(attempt(preflight, [2048, 2048], [2048, 2048], 36 * P, 8 * P, 24 * P)).startswith("refused"))
    over = 36 * P + 8 * P + 24 * P + 36 * P + 32 * P + 2 * INST + 16384 - CEILING
    check("full-scale 2048^2 after a maximal old set exceeds the ceiling by 2*4*2^22 B", over == 2 * 4 * P, over)
    check("1923^2 full scale after a maximal old set fits; 1924^2 is refused",
          preflight([1923, 1923], [1923, 1923], 36 * P, 8 * P, 24 * P) == 670_905_956
          and str(attempt(preflight, [1924, 1924], [1924, 1924], 36 * P, 8 * P, 24 * P)).startswith("refused"))
    check("2048^2 output at 8/16 after a maximal old set fits",
          preflight([2048, 2048], [1024, 1024], 36 * P, 8 * P, 24 * P) == 603_996_160)
    for size in ([2 ** 32 - 1, 2 ** 32 - 1], [0, 1], [8193, 1], [65536, 65536]):
        check(f"overflow-safe refusal {size}", str(attempt(preflight, size, size, 0, 0, 0)).startswith("refused"))
    check("ceiling unchanged", CEILING == 671_105_024)
    out["output_bytes_per_pixel"] = {"portable": 36, "owner": 52, "ce617_portable": 32, "ce617_owner": 48}
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    result = {"model": "0073 owner_quality_overlays CPU controls (no renderer, Cargo or GPU)",
              "viewport": viewport_controls(), "order": order_controls(), "routing": routing(),
              "admission": admission(), "checks": CHECKS}
    result["all_checks_pass"] = all(c["ok"] for c in CHECKS)
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    failed = [c for c in CHECKS if not c["ok"]]
    print(f"checks={len(CHECKS)} failed={len(failed)}")
    for c in failed:
        print("FAILED", c)
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
