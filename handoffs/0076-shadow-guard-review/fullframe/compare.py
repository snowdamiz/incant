"""Full-frame stationary comparison for 0076, CPU only, standard library. Committed
before execution; it reports descriptive metrics and has no pass threshold.

Each output pixel is classed by the view depth of the surface its centre ray
meets in the frozen fixture's stationary scene. The ray is cast analytically
against the ground, the 19 fence posts, cube, mover, sphere, wall, ramp, ramp
post and grille card (solid quad). The classes are the shadow cascades from
cascades.py: c0 (unchanged by the candidate), c1 and c2 (depth range only),
c3 (texel and centre), their blend bands, and none (no surface). For each of the
fixture's three PNGs it counts the baseline/candidate pixels that differ per
class and lists the connected clusters of differing pixels. It also records
whether the baseline temporal-off PNG equals 0072's retained stationary-off.png.

  python3 -I -B compare.py <baseline dir> <candidate dir> <new output.json>
"""
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve()
sys.path.insert(0, str(HERE.parents[1] / "visual" / "tools"))
sys.path.insert(0, str(HERE.parent))
import cascades  # noqa: E402
import pngread  # noqa: E402
import regions  # noqa: E402

W, H = 960, 540
PNGS = ["stationary-off.png", "stationary-on-f16.png", "stationary-activity.png"]
RETAINED_OFF = ("48a688c58f15ce2951292471c5d37cd2cc1fdc9b:"
                "handoffs/0072-motion-history-numeric/fixed-sharpness-screenshots/stationary-off.png")
SLOPE = math.radians(25)
RAMP = (3.4, 1.6 * math.sin(SLOPE), -1.2 - 1.6 * math.cos(SLOPE))
BOXES = ([((-4.5, 0.75, -4.0 - 4.0 * k), (0.12, 0.75, 0.12)) for k in range(19)]
         + [((-2.2, 0.5, 0.5), (0.5, 0.5, 0.5)), ((-2.6, 1.0, -1.0), (0.5, 0.5, 0.5)),
            ((-0.6, 0.6, -2.2), (1.4, 0.6, 0.01)), ((RAMP[0], RAMP[1] + 0.6, RAMP[2]), (0.06, 0.75, 0.06))])
QUADS = [(RAMP, (1.3, 0.0, 0.0), (0.0, 1.6 * math.sin(SLOPE), -1.6 * math.cos(SLOPE))),
         (regions.CARD_AT, (regions.CARD_HALF, 0.0, 0.0), (0.0, regions.CARD_HALF, 0.0))]


def classes():
    b = cascades.boundaries()
    blend = [e - 0.1 * (e - s) for s, e in zip(b[:-1], b[1:])]
    return [("c0", blend[0]), ("c0c1", b[1]), ("c1", blend[1]), ("c1c2", b[2]), ("c2", blend[2]),
            ("c2c3", b[3]), ("c3", blend[3]), ("c3fade", b[4]), ("beyond", math.inf)]


def depth_map():
    """View depth of the nearest surface per pixel centre (inf: none). Rays have
    view-space z = -1, so the ray parameter equals view depth."""
    origin, (right, up, back) = regions.camera(0)
    t = math.tan(regions.FOV / 2)
    rays = []
    for y in range(H):
        ny = 1 - (y + 0.5) / H * 2
        for x in range(W):
            nx = (x + 0.5) / W * 2 - 1
            v = (nx * t * W / H, ny * t, -1.0)
            rays.append(tuple(v[0] * right[k] + v[1] * up[k] + v[2] * back[k] for k in range(3)))
    depth = []
    for d in rays:  # ground y = 0, x in [-14, 14], z in [-110, 10]
        best = math.inf
        if d[1] < 0:
            s = -origin[1] / d[1]
            px, pz = origin[0] + s * d[0], origin[2] + s * d[2]
            if -14 <= px <= 14 and -110 <= pz <= 10:
                best = s
        depth.append(best)
    for centre, half in BOXES:
        for i in candidates(centre, half):
            d = rays[i]
            lo, hi = 0.0, math.inf
            for k in range(3):
                if abs(d[k]) < 1e-12:
                    if abs(origin[k] - centre[k]) > half[k]:
                        lo, hi = 1.0, 0.0
                    continue
                a = (centre[k] - half[k] - origin[k]) / d[k]
                b = (centre[k] + half[k] - origin[k]) / d[k]
                lo, hi = max(lo, min(a, b)), min(hi, max(a, b))
            if lo <= hi and 0 < lo < depth[i]:
                depth[i] = lo
    c, r = regions.SPHERE_AT, regions.SPHERE_RADIUS
    for i in candidates(c, (r, r, r)):
        d = rays[i]
        o = [origin[k] - c[k] for k in range(3)]
        a, bq, cq = sum(v * v for v in d), 2 * sum(o[k] * d[k] for k in range(3)), sum(v * v for v in o) - r * r
        disc = bq * bq - 4 * a * cq
        if disc >= 0:
            s = (-bq - math.sqrt(disc)) / (2 * a)
            if 0 < s < depth[i]:
                depth[i] = s
    for centre, u, v in QUADS:
        n = cross(u, v)
        half = tuple(abs(u[k]) + abs(v[k]) for k in range(3))
        for i in candidates(centre, half):
            d = rays[i]
            denom = sum(n[k] * d[k] for k in range(3))
            if abs(denom) < 1e-12:
                continue
            s = sum(n[k] * (centre[k] - origin[k]) for k in range(3)) / denom
            p = [origin[k] + s * d[k] - centre[k] for k in range(3)]
            if (0 < s < depth[i] and abs(sum(p[k] * u[k] for k in range(3))) <= sum(x * x for x in u)
                    and abs(sum(p[k] * v[k] for k in range(3))) <= sum(x * x for x in v)):
                depth[i] = s
    return depth


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def candidates(centre, half):
    """Pixels inside the screen rectangle of a box's projected corners."""
    pts = []
    for sx in (-1, 1):
        for sy in (-1, 1):
            for sz in (-1, 1):
                p = (centre[0] + sx * half[0], centre[1] + sy * half[1], centre[2] + sz * half[2])
                depth, (px, py) = regions.project(p, 0, (0.0, 0.0, float(W), float(H)))
                if depth <= 0:
                    return range(W * H)  # behind the camera: test everything
                pts.append((px, py))
    x0, x1 = max(0, int(min(p[0] for p in pts)) - 1), min(W, int(max(p[0] for p in pts)) + 2)
    y0, y1 = max(0, int(min(p[1] for p in pts)) - 1), min(H, int(max(p[1] for p in pts)) + 2)
    return [y * W + x for y in range(y0, y1) for x in range(x0, x1)]


def classify(depth):
    bounds = classes()
    out = []
    for d in depth:
        if d == math.inf:
            out.append("none")
            continue
        out.append(next(name for name, limit in bounds if d <= limit))
    return out


def step(a, b, i):
    return max(abs(a[i * 4 + k] - b[i * 4 + k]) for k in range(3))


def clusters(a, b, labels, threshold, top=15):
    hot = {i for i in range(W * H) if step(a, b, i) > threshold}
    seen, found = set(), []
    for start in sorted(hot):
        if start in seen:
            continue
        seen.add(start)
        stack, members = [start], []
        while stack:
            i = stack.pop()
            members.append(i)
            y, x = divmod(i, W)
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    j = (y + dy) * W + x + dx
                    if 0 <= y + dy < H and 0 <= x + dx < W and j in hot and j not in seen:
                        seen.add(j)
                        stack.append(j)
        xs, ys = [i % W for i in members], [i // W for i in members]
        by = {}
        for i in members:
            by[labels[i]] = by.get(labels[i], 0) + 1
        found.append({"pixels": len(members), "bbox": [min(xs), min(ys), max(xs) + 1, max(ys) + 1],
                      "max": max(step(a, b, i) for i in members), "classes": dict(sorted(by.items()))})
    found.sort(key=lambda c: (-c["pixels"], -c["max"], c["bbox"]))
    return {"threshold": threshold, "pixels": len(hot), "components": len(found), "largest": found[:top]}


def main(base_dir, cand_dir, out_path):
    labels = classify(depth_map())
    names = [n for n, _ in classes()] + ["none"]
    result = {"class_pixels": {n: labels.count(n) for n in names}, "pngs": {}}
    for name in PNGS:
        a = pngread.read(Path(base_dir) / name)[2]
        b = pngread.read(Path(cand_dir) / name)[2]
        per = {n: {"pixels": 0, "differing": 0, "over_2": 0, "over_8": 0, "max": 0} for n in names}
        for i in range(W * H):
            v = step(a, b, i)
            row = per[labels[i]]
            row["pixels"] += 1
            row["differing"] += v > 0
            row["over_2"] += v > 2
            row["over_8"] += v > 8
            row["max"] = max(row["max"], v)
        result["pngs"][name] = {"identical": a == b, "per_class": per,
                                "differing_pixels": sum(r["differing"] for r in per.values()),
                                "clusters_over_0": clusters(a, b, labels, 0),
                                "clusters_over_8": clusters(a, b, labels, 8)}
    retained = subprocess.run(["git", "cat-file", "blob", RETAINED_OFF], cwd=HERE.parents[3],
                              capture_output=True, check=True).stdout
    base_off = (Path(base_dir) / PNGS[0]).read_bytes()
    result["baseline_off_equals_0072_retained_off"] = hashlib.sha256(base_off).hexdigest() == hashlib.sha256(
        retained).hexdigest()
    with open(out_path, "x") as out:
        json.dump(result, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({k: {"identical": v["identical"], "differing": v["differing_pixels"]}
                      for k, v in result["pngs"].items()}, indent=1))


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    main(*sys.argv[1:])
