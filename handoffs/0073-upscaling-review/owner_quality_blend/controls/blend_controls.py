#!/usr/bin/env python3
"""0073 owner_quality_blend: CPU controls, standard library only (no Cargo, GPU or renderer).

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_blend/controls/blend_controls.py \
        --out handoffs/0073-upscaling-review/owner_quality_blend/controls/blend_controls.json

1. cluster coordinates: output fragments index the output-viewport grid; reading the
   internal grid with output pixels would address the wrong tiles;
2. scalar blend algebra of the new order (not raster equivalence);
3. routing table of targets and passes;
4. exact output-grid admission bytes and the boundary they move.
"""
import argparse
import json
import math
import random
import sys

sys.dont_write_bytecode = True
CHECKS = []


def check(name, ok, detail=None):
    CHECKS.append({"check": name, "ok": bool(ok), "detail": detail})


def tiles(extent):
    return math.ceil(extent / 64)


def tile(pixel, origin, count):
    """shade.wgsl: min(u32(max(pixel - viewport.xy, 0) / 64), dimensions - 1)."""
    return min(int(max(pixel - origin, 0.0) / 64), count - 1)


def clusters():
    rows = {}
    for name, rect in {"fixture_viewport": [37.5, 21.25, 885.0, 497.5],
                       "kernel_rect": [3.25, 5.5, 65.0, 33.0], "full": [0.0, 0.0, 960.0, 540.0]}.items():
        for n in (8, 12, 5, 13):
            s = n / 16
            source = [v * s for v in rect]
            out_dims = (tiles(rect[2]), tiles(rect[3]))
            in_dims = (tiles(source[2]), tiles(source[3]))
            wrong = total = 0
            covered = set()
            for py in range(int(rect[1]), int(rect[1] + rect[3])):
                for px in range(int(rect[0]), int(rect[0] + rect[2])):
                    x, y = px + 0.5, py + 0.5
                    if not (rect[0] <= x < rect[0] + rect[2] and rect[1] <= y < rect[1] + rect[3]):
                        continue
                    total += 1
                    # Correct: the output grid with the output viewport.
                    t = (tile(x, rect[0], out_dims[0]), tile(y, rect[1], out_dims[1]))
                    covered.add(t)
                    # The output tile is exactly the 64-pixel output cell of the fragment.
                    assert int((x - rect[0]) // 64) == t[0] and int((y - rect[1]) // 64) == t[1]
                    # Misuse: output pixels against the internal grid uniform.
                    internal_right = (tile(source[0] + (x - rect[0]) * s, source[0], in_dims[0]),
                                      tile(source[1] + (y - rect[1]) * s, source[1], in_dims[1]))
                    misuse = (tile(x, source[0], in_dims[0]), tile(y, source[1], in_dims[1]))
                    wrong += misuse != internal_right
            rows[f"{name}@{n}/16"] = {"output_tiles": out_dims, "internal_tiles": in_dims,
                                      "covered_output_tiles": len(covered),
                                      "misuse_wrong_tile_fraction": wrong / total}
            check(f"{name} {n}/16: every output tile is addressed and in range",
                  len(covered) == out_dims[0] * out_dims[1])
            if n <= 13 and rect[2] > 128:
                check(f"{name} {n}/16: output pixels on the internal grid would hit wrong tiles",
                      wrong / total > 0.2, wrong / total)
    return rows


def over(src, dst):
    return tuple(a + b * (1 - src[3]) for a, b in zip(src, dst))


def alpha_blend(straight, dst):
    a = straight[3]
    return tuple([straight[i] * a + dst[i] * (1 - a) for i in range(3)] + [a + dst[3] * (1 - a)])


def clean(v):
    if v[3] <= 0:
        return (0.0, 0.0, 0.0, 0.0)
    return tuple([min(max(c, 0.0), 65504.0) for c in v[:3]] + [min(max(v[3], 0.0), 1.0)])


def blending():
    rng = random.Random(1773)
    worst = 0.0
    for _ in range(3000):
        a = rng.choice([1.0, 0.0, rng.random()])
        resolved = clean((rng.random() * a, rng.random() * a, rng.random() * a, a))
        blends = [(rng.random(), rng.random(), rng.random(), rng.random()) for _ in range(rng.randint(0, 3))]
        sprites = [(lambda q: (rng.random() * q, rng.random() * q, rng.random() * q, q))(rng.random())
                   for _ in range(rng.randint(0, 2))]
        texts = [(lambda q: (0.9 * q, 0.9 * q, 0.9 * q, q))(rng.random()) for _ in range(rng.randint(0, 2))]
        # ce617/f075: BLEND into the cleared internal overlay, composite over resolved, then sprites, text.
        t = (0.0, 0.0, 0.0, 0.0)
        for b in blends:
            t = alpha_blend(b, t)
        old = clean(over(clean(t), resolved))
        for x in sprites + texts:
            old = over(x, old)
        # This patch: composite of an empty overlay, BLEND straight onto it, then sprites, text.
        new = clean(over((0.0, 0.0, 0.0, 0.0), resolved))
        for b in blends:
            new = alpha_blend(b, new)
        for x in sprites + texts:
            new = over(x, new)
        worst = max(worst, max(abs(p - q) for p, q in zip(old, new)))
    check("scalar algebra: BLEND then sprites then text over the composite equals the f075 order",
          worst < 1e-12, worst)
    return {"max_difference": worst, "note": "scalar blend algebra only, not raster equivalence"}


def routing():
    rows = []
    for plan in ("none", "pixel_camera_bypass", "portable_plan"):
        for transparent in (False, True):
            for screen in (False, True):
                up = plan == "portable_plan"
                rows.append({
                    "plan": plan, "blend": transparent, "sprites_or_text": screen,
                    "blend_target": ("output composite, output depth, output rect, output grid" if up
                                     else "f075 target (internal overlay or scene colour)") if transparent else None,
                    "output_grid_prepared": up and transparent,
                    "output_cluster_pass": up and transparent,
                    "internal_overlay_depth_pass": False if up else "f075 rule (accumulating and any overlay)",
                    "output_depth_pass": up and (transparent or screen),
                    "models_order": "resolve, overlay clear, composite, output depth, BLEND" if up else "f075",
                })
    check("no plan keeps every f075 target", all(r["plan"] == "portable_plan" or not r["output_grid_prepared"]
                                                for r in rows))
    check("an output grid exists only for portable plans with BLEND draws",
          all(r["output_grid_prepared"] == (r["plan"] == "portable_plan" and r["blend"]) for r in rows))
    return rows


MAX_CLUSTERS = 131072
PER_CLUSTER = 4 * (1 + 4096 // 32)
P = 4 * 1024 * 1024
INST = 64 * 1024 * 1024
CEILING = 128 * P + 2 * INST + 16384


def allowance(rect):
    if not all(math.isfinite(v) for v in rect) or rect[2] <= 0 or rect[3] <= 0:
        return None
    c = tiles(rect[2]) * tiles(rect[3]) * 24
    return c * PER_CLUSTER if c <= MAX_CLUSTERS else None


def admission():
    out = {"per_cluster_bytes": PER_CLUSTER, "2048x2048": allowance([0, 0, 2048, 2048]),
           "fixture_viewport": allowance([37.5, 21.25, 885, 497.5]), "1923x1923": allowance([0, 0, 1923, 1923])}
    check("2048^2 output grid allowance", out["2048x2048"] == 12_681_216, out["2048x2048"])
    check("fixture viewport grid allowance", out["fixture_viewport"] == 1_387_008, out["fixture_viewport"])
    f075_1923 = 36 * P + 8 * P + 24 * P + 36 * 1923 * 1923 + 32 * 1923 * 1923 + 2 * INST + 16384
    check("f075 boundary 1923^2 fit with 199068 B headroom; the grid allowance now refuses it",
          CEILING - f075_1923 == 199_068 and f075_1923 + out["1923x1923"] > CEILING,
          {"headroom": CEILING - f075_1923, "grid": out["1923x1923"]})
    largest = max(n for n in range(1700, 1924)
                  if 36 * P + 8 * P + 24 * P + 68 * n * n + 2 * INST + 16384 + allowance([0, 0, n, n]) <= CEILING)
    out["largest_full_scale_square_after_maximal_old_with_grid"] = largest
    check("a full-scale square still fits after a maximal old set", largest >= 1800, largest)
    for bad in ([0, 0, 0, 1], [0, 0, float("nan"), 1], [0, 0, float("inf"), 1], [0, 0, 8192 * 64, 64]):
        check(f"refused grid {bad}", allowance(bad) is None)
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    result = {"model": "0073 owner_quality_blend CPU controls", "clusters": clusters(), "blending": blending(),
              "routing": routing(), "admission": admission(), "checks": CHECKS}
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
