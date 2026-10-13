"""CPU-only, fail-closed comparison of the main224 baseline and the f636a2d5
shadow receiver guard candidate on the frozen viewport case (handoff 0076).
Standard library only; never starts a renderer, compiler or GPU child.

Adapted from 0072's sharpness.py (48a688c5): the image reader, edge mask,
Gaussian blur and off-edge bins are reused unchanged; the comparison is new for
this slice. A comparison is only valid when it is exact: the two runs differ in
source commit alone (same fixture, features, toolchain, host, adapter, config
and viewport) and their temporal-on frames are byte-identical, because the
candidate changes only the temporal-off shadow fit. Otherwise it fails and
nothing is attributed to the fix. Every check records its value, limit and
outcome; the verdict passes only if every required check passes. Inherited
temporal-quality results are recorded separately and never decide it.
"""

import hashlib
import json
import math
from collections import deque
from pathlib import Path

import pngread
import regions

LUT = [(v / 255 / 12.92) if v / 255 <= 0.04045 else ((v / 255 + 0.055) / 1.055) ** 2.4 for v in range(256)]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Image:
    def __init__(self, path=None, size=None, rgba=None):
        self.path = Path(path) if path else None
        if path is not None:
            self.w, self.h, self.rgba = pngread.read(self.path)
        else:
            (self.w, self.h), self.rgba = size, bytes(rgba)
        self._lum = None

    @property
    def lum(self):
        """Linear-light luminance per pixel (0..1)."""
        if self._lum is None:
            d = self.rgba
            self._lum = [0.2126 * LUT[d[i]] + 0.7152 * LUT[d[i + 1]] + 0.0722 * LUT[d[i + 2]]
                         for i in range(0, len(d), 4)]
        return self._lum


def step(d, p, q):
    return max(abs(d[p] - d[q]), abs(d[p + 1] - d[q + 1]), abs(d[p + 2] - d[q + 2]))


def edge_mask(img, threshold=24, dilate=True):
    """The fixture's reference edges: max-channel neighbour step above the threshold
    marks both pixels; then a 3x3 dilation."""
    w, h, d = img.w, img.h, img.rgba
    raw = bytearray(w * h)
    for y in range(h):
        for x in range(w):
            i = y * w + x
            if x + 1 < w and step(d, i * 4, i * 4 + 4) > threshold:
                raw[i] = raw[i + 1] = 1
            if y + 1 < h and step(d, i * 4, (i + w) * 4) > threshold:
                raw[i] = raw[i + w] = 1
    if not dilate:
        return raw
    out = bytearray(raw)
    for y in range(h):
        for x in range(w):
            if raw[y * w + x]:
                for ny in range(max(0, y - 1), min(h, y + 2)):
                    for nx in range(max(0, x - 1), min(w, x + 2)):
                        out[ny * w + nx] = 1
    return out


def step2(a, b, p):
    return max(abs(a[p] - b[p]), abs(a[p + 1] - b[p + 1]), abs(a[p + 2] - b[p + 2]))


def off_edge_bins(on, off, threshold=12, inside=None):
    """On-minus-off differences at pixels at least 3 px (8-connected) from any edge of
    either image (neighbour step above `threshold`): counts at 3-8, 9-16, >16 steps.
    With `inside` (a pixel-index set), the counts are also split inside/outside it."""
    w, h = on.w, on.h
    e = bytearray(w * h)
    for img in (on, off):
        mask = edge_mask(img, threshold, dilate=False)
        for i, v in enumerate(mask):
            if v:
                e[i] = 1
    far = 1 << 30
    dist = [far] * (w * h)
    q = deque()
    for i, v in enumerate(e):
        if v:
            dist[i] = 0
            q.append(i)
    while q:
        i = q.popleft()
        y, x = divmod(i, w)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                ny, nx = y + dy, x + dx
                if 0 <= ny < h and 0 <= nx < w and dist[ny * w + nx] > dist[i] + 1:
                    dist[ny * w + nx] = dist[i] + 1
                    q.append(ny * w + nx)
    parts = {"all": [0, 0, 0, 0]}
    if inside is not None:
        parts.update({"inside": [0, 0, 0, 0], "outside": [0, 0, 0, 0]})
    for i in range(w * h):
        if dist[i] >= 3:
            v = step2(on.rgba, off.rgba, i * 4)
            keys = ["all"] + ([] if inside is None else ["inside" if i in inside else "outside"])
            for key in keys:
                bins = parts[key]
                bins[3] = max(bins[3], v)
                if 3 <= v <= 8:
                    bins[0] += 1
                elif 9 <= v <= 16:
                    bins[1] += 1
                elif v > 16:
                    bins[2] += 1
    return {key: {"3-8": b[0], "9-16": b[1], ">16": b[2], "max": b[3]} for key, b in parts.items()}


# ------------------------------------------------------------------ region metrics

def kernel(sigma):
    r = max(1, int(math.ceil(3 * sigma)))
    k = [math.exp(-(t * t) / (2 * sigma * sigma)) for t in range(-r, r + 1)]
    s = sum(k)
    return r, [v / s for v in k]


def blurred(lum, w, h, box, sigma):
    """Separable Gaussian (edge-clamped) of a luminance plane, evaluated on box
    [x0, y0, x1, y1) only; returns {index: value}. sigma 0 is the identity."""
    x0, y0, x1, y1 = box
    if sigma <= 0:
        return {y * w + x: lum[y * w + x] for y in range(y0, y1) for x in range(x0, x1)}
    r, k = kernel(sigma)
    rows = {}
    for y in range(max(0, y0 - r), min(h, y1 + r)):
        for x in range(x0, x1):
            rows[(x, y)] = sum(k[t + r] * lum[y * w + min(max(x + t, 0), w - 1)] for t in range(-r, r + 1))
    out = {}
    for y in range(y0, y1):
        for x in range(x0, x1):
            out[y * w + x] = sum(k[t + r] * rows[(x, min(max(y + t, max(0, y0 - r)), min(h, y1 + r) - 1))]
                                 for t in range(-r, r + 1))
    return out


def region_metrics(on, off, idx, spec):
    """Shadow agreement inside one region (linear-light luminance 0..1):
    - mean_gap: |mean(on) - mean(off)| / mean(off), the amount of shadow;
    - lowpass_max: max |G(on) - G(off)| at sigma spec["lowpass_sigma_px"];
    - blur_residual: min over spec["blur_sigmas_px"] of RMS(on - G_sigma(off)),
      the part of the difference that blurring the reference cannot explain;
    plus the per-pixel max-channel step and counts above 8 and 16 steps."""
    w, h = off.w, off.h
    box = regions.bbox(idx, w)
    a, b = on.lum, off.lum
    n = len(idx)
    mean_on = sum(a[i] for i in idx) / n
    mean_off = sum(b[i] for i in idx) / n
    steps = [step2(on.rgba, off.rgba, i * 4) for i in idx]
    ga = blurred(a, w, h, box, spec["lowpass_sigma_px"])
    gb = blurred(b, w, h, box, spec["lowpass_sigma_px"])
    low = [abs(ga[i] - gb[i]) for i in idx]
    fits = []
    for sigma in spec["blur_sigmas_px"]:
        g = blurred(b, w, h, box, sigma)
        fits.append((math.sqrt(sum((a[i] - g[i]) ** 2 for i in idx) / n), sigma))
    best = min(fits)
    return {"pixels": n, "bbox": box, "mean_on": round(mean_on, 5), "mean_off": round(mean_off, 5),
            "mean_gap": round(abs(mean_on - mean_off) / mean_off, 5),
            "lowpass_max": round(max(low), 5), "lowpass_mean": round(sum(low) / n, 5),
            "blur_residual": round(best[0], 5), "blur_sigma_px": best[1],
            "rms_unblurred": round(fits[0][0], 5) if spec["blur_sigmas_px"][0] == 0 else None,
            "max_step": max(steps), "over_8": sum(s > 8 for s in steps), "over_16": sum(s > 16 for s in steps)}


def encode(v):
    v = min(max(v, 0.0), 1.0)
    return int(round(255 * (12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055)))


def blurred_copy(img, sigma, box):
    """Calibration only: a copy of img whose box holds a linear-light Gaussian
    blur of each colour channel (edge-clamped), re-encoded to 8-bit sRGB."""
    out = bytearray(img.rgba)
    for c in range(3):
        plane = [LUT[v] for v in img.rgba[c::4]]
        for i, v in blurred(plane, img.w, img.h, box, sigma).items():
            out[i * 4 + c] = encode(v)
    return Image(size=(img.w, img.h), rgba=out)


def agreement(metrics, limits):
    """Which agreement limits a region's metrics satisfy."""
    return {"mean_gap": metrics["mean_gap"] <= limits["mean_gap_max"],
            "blur_residual": metrics["blur_residual"] <= limits["blur_residual_max"],
            "lowpass_max": metrics["lowpass_max"] <= limits["lowpass_max_max"]}


# ------------------------------------------------------------------ guards

def outside_pixels(img, rect):
    x0, y0, w, h = (int(v) for v in rect)
    return [y * img.w + x for y in range(img.h) for x in range(img.w)
            if not (x0 <= x < x0 + w and y0 <= y < y0 + h)]


def outside_report(a, b, rect, canvas):
    """Outside the viewport: byte equality of the two images and with the canvas."""
    idx = outside_pixels(a, rect)
    differ = sum(a.rgba[i * 4:i * 4 + 4] != b.rgba[i * 4:i * 4 + 4] for i in idx)
    want = bytes(canvas) + b"\xff"
    off_canvas = sum(a.rgba[i * 4:i * 4 + 4] != want for i in idx) + sum(
        b.rgba[i * 4:i * 4 + 4] != want for i in idx)
    return {"pixels": len(idx), "differing": differ, "not_canvas": off_canvas}


def band_split(a, b, rect, band):
    """Max-channel differences inside the viewport: the `band` px just inside its
    border against the rest."""
    x0, y0, w, h = (int(v) for v in rect)
    worst = {"band": [0, 0], "inner": [0, 0]}
    for y in range(y0, y0 + h):
        for x in range(x0, x0 + w):
            i = (y * a.w + x) * 4
            v = step2(a.rgba, b.rgba, i)
            key = "inner" if (x0 + band <= x < x0 + w - band and y0 + band <= y < y0 + h - band) else "band"
            worst[key][0] = max(worst[key][0], v)
            worst[key][1] += v > 2
    return {k: {"max": v[0], "over_2": v[1]} for k, v in worst.items()}


def bar_mask(img, idx, rule):
    """Grille-bar pixels: green-dominant by rule["green_over_max_rb"] and at least
    rule["green_min"]; the alpha-tested bars are lit by the cool point light and
    ambient fill, not by the sun, so their classification is fit-independent."""
    d = img.rgba
    return {i for i in idx if d[i * 4 + 1] >= rule["green_min"]
            and d[i * 4 + 1] >= rule["green_over_max_rb"] * max(d[i * 4], d[i * 4 + 2])}


def clusters(a, b, threshold, rect, top=12):
    """Connected (8-neighbour) components of max-channel differences above
    `threshold` inside rect: size, bbox, max and peak location, largest first."""
    x0, y0, w, h = (int(v) for v in rect)
    W = a.w
    hot = {(y * W + x) for y in range(y0, y0 + h) for x in range(x0, x0 + w)
           if step2(a.rgba, b.rgba, (y * W + x) * 4) > threshold}
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
                    if j in hot and j not in seen:
                        seen.add(j)
                        stack.append(j)
        peak = max(members, key=lambda i: step2(a.rgba, b.rgba, i * 4))
        found.append({"pixels": len(members), "bbox": regions.bbox(members, W),
                      "max": step2(a.rgba, b.rgba, peak * 4), "peak": [peak % W, peak // W]})
    found.sort(key=lambda c: (-c["pixels"], -c["max"]))
    return {"threshold": threshold, "pixels": len(hot), "components": len(found), "largest": found[:top]}


# ------------------------------------------------------------------ inherited

def inherited(report, pan, limits):
    """The 0072 viewport temporal-quality rules, unchanged, as observations: they
    measure the resolve, which the candidate does not touch. `pan` is the pan f24
    pair's off_edge_bins(...)["all"]."""
    obs = report["observations"]
    bins = obs["still_activity_frames_9_16"]
    late = obs["still_consecutive"][7:]
    values = {"still_activity_ge33": bins["33-63"] + bins["64+"], "still_activity_saturated": bins["64+"],
              "still_frames_9_16_edge_mean": round(max(c["delta"]["edge"]["mean_delta"] for c in late), 3),
              "still_frames_9_16_edge_max": max(c["delta"]["edge"]["max_delta"] for c in late),
              "still_frames_9_16_interior_changed": max(c["delta"]["interior"]["changed_over_threshold"]
                                                         for c in late),
              "still_same_phase_interior_changed":
                  obs["still_same_phase_f08_vs_f16"]["interior"]["changed_over_threshold"],
              "pan_off_edge_9_16": pan["9-16"], "pan_off_edge_over_16": pan[">16"]}
    rules = {"still_activity_ge33": limits["activity_ge33_max"], "still_activity_saturated": limits["saturated_max"],
             "still_frames_9_16_edge_mean": limits["edge_mean_max"], "still_frames_9_16_edge_max": limits["edge_max_max"],
             "still_frames_9_16_interior_changed": limits["interior_changed_max"],
             "still_same_phase_interior_changed": limits["same_phase_interior_max"],
             "pan_off_edge_9_16": 0, "pan_off_edge_over_16": 0}
    return {k: {"value": values[k], "limit": rules[k], "within": values[k] <= rules[k]} for k in values}


# ------------------------------------------------------------------ runs

class Run:
    """One role's viewport report, PNGs and build records, located by name."""

    def __init__(self, label, report, pngs, build):
        self.label, self.report_path, self.pngs, self.build = label, report, pngs, build
        self._images = {}

    def report(self):
        path = self.report_path
        return json.loads(Path(path).read_text()) if path and Path(path).is_file() else None

    def image(self, name):
        if name not in self._images:
            path = self.pngs.get(name)
            self._images[name] = Image(path) if path and Path(path).is_file() else None
        return self._images[name]

    def record(self, name):
        path = self.build.get(name)
        return json.loads(Path(path).read_text()) if path and Path(path).is_file() else None


def run_from_captures(label, evidence, role, case):
    """A live run: captures/<role>/<case>/report.json with its PNGs; build/<role>/*.json."""
    evidence = Path(evidence)
    output = evidence / "captures" / role / case
    pngs = {p.name: p for p in sorted(output.glob("*.png"))}
    build = {p.name: p for p in sorted((evidence / "build" / role).glob("*.json"))}
    return Run(label, output / "report.json", pngs, build)


# ------------------------------------------------------------------ comparison

def frame_hashes(report, prefix):
    return {f["name"]: f.get("rgba_sha256") for f in report.get("frames", []) if f["name"].startswith(prefix)}


def png_rgba(report, name):
    return {p["name"]: p.get("rgba_sha256") for p in report.get("pngs", [])}.get(name)


def compare(baseline, candidate, plan):
    spec = plan["comparison"]
    review = plan["review"]
    checks, observations = [], {}

    def check(section, name, value, limit, ok):
        checks.append({"section": section, "check": name, "value": value, "limit": limit, "pass": bool(ok)})
        return bool(ok)

    reports = {run.label: run.report() for run in (baseline, candidate)}
    images = {run.label: (run.image(spec["on"]), run.image(spec["off"])) for run in (baseline, candidate)}
    present = all(reports.values()) and all(i is not None for pair in images.values() for i in pair)
    check("exactness", "reports_and_pngs_present", {k: bool(v) for k, v in reports.items()}, "both runs", present)
    if not present:
        return finish(checks, observations)
    rb, rc = reports[baseline.label], reports[candidate.label]
    (bon, boff), (con, coff) = images[baseline.label], images[candidate.label]

    # -------------------------------------------------- exactness
    sources = plan["sources"]
    commits = [(run.record("source.json") or {}).get("source_commit") for run in (baseline, candidate)]
    check("exactness", "source_commits", commits, [sources["baseline"]["commit"], sources["candidate"]["commit"]],
          commits == [sources["baseline"]["commit"], sources["candidate"]["commit"]])
    reviews = [run.record("review.json") or {} for run in (baseline, candidate)]
    helpers = [r.get("helper_sha256") for r in reviews]
    check("exactness", "same_fixture", helpers, plan["fixture"]["sha256"],
          helpers == [plan["fixture"]["sha256"]] * 2)
    frozen = [(r.get("frozen") or {}).get(review["binary"], {}) for r in reviews]
    features = [sorted(f.get("features") or []) for f in frozen]
    check("exactness", "same_features", features, sorted(plan["builds"]["expected_features"]),
          features == [sorted(plan["builds"]["expected_features"])] * 2)
    argv = [[a for a in ((r.get("builds") or [{}])[0].get("argv") or [])[1:]] for r in reviews]
    check("exactness", "same_build_arguments", argv, "equal after the checkout's tools/cargo path",
          argv[0] == argv[1] and bool(argv[0]))
    srcs = [run.record("source.json") or {} for run in (baseline, candidate)]
    lock = [s.get("cargo_lock_sha256") for s in srcs]
    toolchain_file = [s.get("rust_toolchain") for s in srcs]
    check("exactness", "same_lock_and_toolchain_file", lock, "equal",
          lock[0] is not None and lock[0] == lock[1] and toolchain_file[0] == toolchain_file[1])

    def toolchain(run):
        cmds = (run.record("toolchain.json") or {}).get("commands", {})
        return {k: (cmds.get(k, {}).get("stdout") or "").strip().splitlines()[:2]
                for k in ("rustc", "cargo", "sw_vers", "hw_model")}
    tools = [toolchain(baseline), toolchain(candidate)]
    check("exactness", "same_toolchain_and_host", tools, "equal", tools[0] == tools[1] and bool(tools[0]["rustc"]))
    adapters = [(r.get("renderer") or {}).get("adapter") for r in (rb, rc)]
    check("exactness", "same_adapter", adapters, "equal, backend " + plan["expected_backend"],
          adapters[0] == adapters[1] and bool(adapters[0])
          and adapters[0].get("backend") == plan["expected_backend"])
    configs = [r.get("review_config") for r in (rb, rc)]
    want = plan["review_config"]
    check("exactness", "same_review_config", configs, want,
          all(c and set(c) == set(want) and all(abs(float(c[k]) - float(want[k])) <= 1e-6 * max(1.0, abs(want[k]))
                                                 for k in want) for c in configs))
    geometry = [[r.get("viewport_rect"), r.get("internal_size")] for r in (rb, rc)]
    check("exactness", "same_viewport", geometry, [spec["viewport_rect"], spec["internal_size"]],
          geometry == [[spec["viewport_rect"], spec["internal_size"]]] * 2)
    on_b, on_c = frame_hashes(rb, "on-"), frame_hashes(rc, "on-")
    same_on = sorted(on_b) == sorted(spec["on_frames"]) and on_b == on_c and all(on_b.values())
    check("exactness", "temporal_on_frames_identical",
          {"frames": len(on_b), "differing": sorted(k for k in on_b if on_b.get(k) != on_c.get(k))},
          f"{len(spec['on_frames'])} frames, byte-identical (the candidate only changes the temporal-off fit)",
          same_on and png_rgba(rb, spec["on"]) == png_rgba(rc, spec["on"]) and bon.rgba == con.rgba)

    # -------------------------------------------------- guards
    rect, canvas = spec["viewport_rect"], spec["canvas_srgb"]
    for label, report in ((baseline.label, rb), (candidate.label, rc)):
        value = report["observations"]["outside_viewport_max_delta"]
        check("guards", f"outside_viewport_static_{label}", value, "== 0", value == 0)
    for name, (a, b) in ((spec["on"], (bon, con)), (spec["off"], (boff, coff))):
        out = outside_report(a, b, rect, canvas)
        check("guards", f"outside_viewport_identical_{name}", out, "0 differing, all canvas",
              out["differing"] == 0 and out["not_canvas"] == 0)
    for key in ("still_f16_border_vs_off", "pan_f24_border_vs_off"):
        split = rc["observations"][key]
        band, inner = split["border_band_3px"]["max_delta"], split["inner"]["max_delta"]
        check("guards", f"candidate_{key}_band_not_above_inner", [band, inner], "band <= inner", band <= inner)
    moved = band_split(boff, coff, rect, spec["border_band_px"])
    check("guards", "off_change_not_concentrated_at_border", moved, "band max <= inner max",
          moved["band"]["max"] <= moved["inner"]["max"])
    card = set(regions.mask([tuple(v) for v in regions.pose(spec["pose"])["card"]], 0, rect))
    bars = [bar_mask(img, card, spec["bars"]) for img in (boff, coff)]
    check("guards", "mask_card_bars_identical_off", {"pixels": [len(b) for b in bars],
                                                      "differing": len(bars[0] ^ bars[1])},
          f"identical, at least {spec['bars']['min_pixels']} bar pixels",
          bars[0] == bars[1] and len(bars[0]) >= spec["bars"]["min_pixels"])
    observations["mask_card_bar_colours_differing_off"] = sum(
        boff.rgba[i * 4:i * 4 + 3] != coff.rgba[i * 4:i * 4 + 3] for i in bars[0] & bars[1])

    # -------------------------------------------------- agreement and defect
    idx = regions.region(spec["pose"], spec["region"])
    limits = spec["thresholds"]["agreement"]
    metrics = {baseline.label: region_metrics(bon, boff, idx, spec["metrics"]),
               candidate.label: region_metrics(con, coff, idx, spec["metrics"])}
    mine, theirs = metrics[candidate.label], metrics[baseline.label]
    met = agreement(mine, limits)
    for key, limit in (("mean_gap", "mean_gap_max"), ("blur_residual", "blur_residual_max"),
                       ("lowpass_max", "lowpass_max_max")):
        check("agreement", f"candidate_grille_shadow_{key}", mine[key], f"<= {limits[limit]}", met[key])
    still = [r["observations"]["still_f16_vs_off"]["interior"]["max_delta"] for r in (rb, rc)]
    check("agreement", "candidate_still_interior_max_delta", still[1], f"<= {limits['still_interior_max_delta_max']}",
          still[1] <= limits["still_interior_max_delta_max"])
    base_met = agreement(theirs, limits)
    check("defect", "baseline_reproduces_on_off_mismatch",
          {"agreement_limits_met": base_met, **{k: theirs[k] for k in base_met}},
          "the baseline fails every grille-shadow agreement limit, as the calibration's mismatch controls do",
          not any(base_met.values()))
    better = {k: [mine[k], theirs[k]] for k in ("mean_gap", "blur_residual", "lowpass_max")}
    better["still_interior_max_delta"] = [still[1], still[0]]
    check("defect", "candidate_better_than_baseline", better, "every candidate value below the baseline's",
          all(v[0] < v[1] for v in better.values()))
    observations["grille_shadow_region"] = metrics
    inside = set(idx)
    edge_bins = {run: off_edge_bins(on, off, inside=inside) for run, (on, off) in images.items()}
    observations["pan_off_edge_bins"] = edge_bins
    observations["off_before_after"] = {"band_split": moved,
                                        "clusters_over_8": clusters(boff, coff, 8, rect),
                                        "grille_shadow_region": region_metrics(coff, boff, idx, spec["metrics"])}
    observations["on_before_after_max_step"] = max(
        (step2(bon.rgba, con.rgba, i) for i in range(0, len(bon.rgba), 4)), default=0)
    observations["inherited_temporal_quality"] = {
        run: inherited(reports[run], edge_bins[run]["all"], spec["inherited"]) for run in reports}
    observations["report_still_f16_vs_off"] = {run: reports[run]["observations"]["still_f16_vs_off"]
                                               for run in reports}
    observations["report_pan_f24_vs_off"] = {run: reports[run]["observations"]["pan_f24_vs_off"]
                                             for run in reports}
    return finish(checks, observations)


def calibrate(spec, files):
    """Agreement limits from retained controls, by one rule: each limit is the
    geometric mean of the worst consistent-fit control and the best mismatch
    control, rounded down to two significant figures. `files` maps each control's
    retained paths to local copies. Returns per-control values and the limits."""
    metric_spec, values = spec["metrics"], {}
    for control in spec["calibration"]["controls"]:
        idx = regions.region(control["pose"], spec["region"])
        box = [*regions.bbox(idx)]
        box = [box[0] - spec["calibration"]["synthetic_margin_px"], box[1] - spec["calibration"]["synthetic_margin_px"],
               box[2] + spec["calibration"]["synthetic_margin_px"], box[3] + spec["calibration"]["synthetic_margin_px"]]
        off = Image(files[control["off"]])
        on = Image(files[control["on"]]) if "on" in control else off
        synthetic = control.get("synthetic", {})
        if "blur_on_sigma_px" in synthetic:
            on = blurred_copy(on, synthetic["blur_on_sigma_px"], box)
        m = region_metrics(on, off, idx, metric_spec)
        values[control["id"]] = {"kind": control["kind"], **{k: m[k] for k in (
            "pixels", "mean_gap", "blur_residual", "lowpass_max", "blur_sigma_px", "max_step")}}
    for control in spec["calibration"]["still_controls"]:
        body = json.loads(Path(files[control["report"]]).read_text())
        values[control["id"]] = {"kind": control["kind"],
                                 "still_interior_max_delta": body["observations"][control["key"]]["interior"]["max_delta"]}

    def limit(key):
        good = [v[key] for v in values.values() if v["kind"] == "consistent" and key in v]
        bad = [v[key] for v in values.values() if v["kind"] == "mismatch" and key in v]
        middle = math.sqrt(max(good) * min(bad))
        scale = 10 ** (math.floor(math.log10(middle)) - 1)
        return {"worst_consistent": max(good), "best_mismatch": min(bad),
                "limit": math.floor(middle / scale + 1e-9) * scale}
    derived = {"mean_gap_max": limit("mean_gap"), "blur_residual_max": limit("blur_residual"),
               "lowpass_max_max": limit("lowpass_max"),
               "still_interior_max_delta_max": limit("still_interior_max_delta")}
    for item in derived.values():
        item["limit"] = round(item["limit"], 6)
    return {"values": values, "derived": derived}


def finish(checks, observations):
    exact = all(c["pass"] for c in checks if c["section"] == "exactness")
    failed = [f"{c['section']}: {c['check']}" for c in checks if not c["pass"]]
    return {"verdict": "pass" if checks and not failed else "fail", "exact": exact, "failed": failed,
            "checks": checks, "observations": observations,
            "inherited_note": "inherited temporal-quality rules are observations of the unchanged resolve; "
                              "they never decide this verdict"}
