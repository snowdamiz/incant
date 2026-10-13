"""CPU-only, fail-closed comparison of a candidate fixed-temporal run against the
retained accepted baseline (handoff 0072; first the fixed-sharpness review, now the
fixed-warmup reset-ramp review). Standard library only; never starts a renderer,
compiler or GPU child.

The metrics are those of the committed coverage inspection
(temporal-coverage-inspection/), applied the same way to both runs. The thresholds
come from plan.json "comparison" (fixed-sharpness-design.md proposals plus every
earlier coverage/ghost/reset/overlay constraint). A comparison is only valid when it
is exact: the baseline matches its committed manifest and identities, and both runs
share the temporal-off references, adapter, toolchain and review config. Otherwise
it fails; nothing incompatible is reused silently. Every check records its value,
limit and outcome; the verdict passes only if every check passes.

When the plan names a prior run (comparison.prior: the rejected a143b5c0 sharpness
run) it is a second exact reference: its manifest, files, identity, recorded failures
and temporal-off frames must verify, and every quality check is evaluated on it with
the same code. Its recomputed failures must equal the recorded ones; the side-by-side
values ("prior_effect") and the resolved, persisting and new failures show the effect
of the one change and never act as thresholds. comparison.history_free_frames requires
every history-free frame to be byte-identical to the prior's and the baseline's.
"""

import hashlib
import json
import math
from collections import deque
from pathlib import Path

import pngread

LUT = [(v / 255 / 12.92) if v / 255 <= 0.04045 else ((v / 255 + 0.055) / 1.055) ** 2.4 for v in range(256)]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class Image:
    def __init__(self, path):
        self.path = Path(path)
        self.w, self.h, self.rgba = pngread.read(self.path)
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


BANDS = ((1, 2), (3, 8), (9, 16), (17, 32), (33, 63), (64, 64))


def activity_bins(activity, mask):
    """Pixels of an activity map (peak x4, 255 = 64+ steps) per peak band, edge/interior."""
    counts = {f"{lo}-{hi}": [0, 0] for lo, hi in BANDS}
    d = activity.rgba
    for i in range(activity.w * activity.h):
        v = d[i * 4]
        peak = 64 if v == 255 else v // 4
        for lo, hi in BANDS:
            if lo <= peak <= hi:
                counts[f"{lo}-{hi}"][mask[i]] += 1
                break
    total = {k: v[0] + v[1] for k, v in counts.items()}
    return {"edge_interior": counts, "ge33": total["33-63"] + total["64-64"], "saturated": total["64-64"]}


def bins_from_report(bins):
    """The viewport case records its activity bands in report.json."""
    return {"ge33": bins["33-63"] + bins["64+"], "saturated": bins["64+"]}


def detail(lum, w, box):
    x0, y0, x1, y1 = box
    total = 0.0
    for y in range(y0, y1):
        for x in range(x0, x1):
            m = sum(lum[(y + dy) * w + x + dx] for dy in (-1, 0, 1) for dx in (-1, 0, 1)) / 9
            total += abs(lum[y * w + x] - m)
    return total / ((x1 - x0) * (y1 - y0))


def blurred(lum, w, h, box, sigma):
    """Separable Gaussian of the region plus padding (edge-clamped), as a dense lum list."""
    x0, y0, x1, y1 = box
    r = max(1, int(math.ceil(3 * sigma)))
    k = [math.exp(-(t * t) / (2 * sigma * sigma)) for t in range(-r, r + 1)]
    s = sum(k)
    k = [v / s for v in k]
    pad = r + 2
    X0, Y0, X1, Y1 = max(0, x0 - pad), max(0, y0 - pad), min(w, x1 + pad), min(h, y1 + pad)
    tmp = {}
    for y in range(max(0, Y0 - r), min(h, Y1 + r)):
        for x in range(X0, X1):
            tmp[(x, y)] = sum(k[t + r] * lum[y * w + min(max(x + t, 0), w - 1)] for t in range(-r, r + 1))
    out = list(lum)
    for y in range(Y0, Y1):
        for x in range(X0, X1):
            out[y * w + x] = sum(k[t + r] * tmp[(x, min(max(y + t, max(0, Y0 - r)), min(h, Y1 + r) - 1))]
                                 for t in range(-r, r + 1))
    return out


def sigma_equivalent(on, off, box):
    """Gaussian sigma (px) at which the reference retains the candidate's detail."""
    target = detail(on.lum, on.w, box)
    if target >= detail(off.lum, off.w, box):
        return 0.0
    lo, hi = 0.05, 2.0
    if detail(blurred(off.lum, off.w, off.h, box, hi), off.w, box) > target:
        return hi
    for _ in range(10):
        mid = (lo + hi) / 2
        if detail(blurred(off.lum, off.w, off.h, box, mid), off.w, box) > target:
            lo = mid
        else:
            hi = mid
    return round((lo + hi) / 2, 3)


def channel_extreme_3x3(img, channel, pick):
    """Per-pixel 3x3 maximum or minimum (edge-clamped) of one channel, separably."""
    w, h, d = img.w, img.h, img.rgba
    plane = d[channel::4]
    row = [0] * (w * h)
    for y in range(h):
        b = y * w
        for x in range(w):
            row[b + x] = pick(plane[b + max(x - 1, 0)], plane[b + x], plane[b + min(x + 1, w - 1)])
    out = [0] * (w * h)
    for y in range(h):
        up, down = max(y - 1, 0) * w, min(y + 1, h - 1) * w
        b = y * w
        for x in range(w):
            out[b + x] = pick(row[up + x], row[b + x], row[down + x])
    return out


def halo(on, off, steps):
    """Pixels where any channel exceeds the reference's local 3x3 maximum by more than
    `steps` (halo), and, as an observation, falls below its 3x3 minimum by more."""
    above = bytearray(on.w * on.h)
    below = bytearray(on.w * on.h)
    for c in range(3):
        hi = channel_extreme_3x3(off, c, max)
        lo = channel_extreme_3x3(off, c, min)
        d = on.rgba
        for i in range(on.w * on.h):
            v = d[i * 4 + c]
            if v > hi[i] + steps:
                above[i] = 1
            if v < lo[i] - steps:
                below[i] = 1
    over, under = sum(above), sum(below)
    return {"above_local_max": over, "below_local_min": under}


def off_edge_bins(on, off, threshold=12):
    """On-minus-off differences at pixels at least 3 px (8-connected) from any edge of
    either image (neighbour step above `threshold`): counts at 3-8, 9-16, >16 steps."""
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
    bins = [0, 0, 0]
    worst = 0
    for i in range(w * h):
        if dist[i] >= 3:
            v = step2(on.rgba, off.rgba, i * 4)
            worst = max(worst, v)
            if 3 <= v <= 8:
                bins[0] += 1
            elif 9 <= v <= 16:
                bins[1] += 1
            elif v > 16:
                bins[2] += 1
    return {"3-8": bins[0], "9-16": bins[1], ">16": bins[2], "max": worst}


def step2(a, b, p):
    return max(abs(a[p] - b[p]), abs(a[p + 1] - b[p + 1]), abs(a[p + 2] - b[p + 2]))


def max_step(a, b):
    return max((step2(a.rgba, b.rgba, i) for i in range(0, len(a.rgba), 4)), default=0)


def cube_bands(on, off):
    """Moving-cube orangeness (R-B) beside its trailing (left) edge and above its top
    edge (perpendicular to the motion, a no-motion control), relative to temporal-off."""
    w = off.w

    def rb(d, x, y):
        return d[(y * w + x) * 4] - d[(y * w + x) * 4 + 2]
    left, top = [], []
    for y in range(190, 240):
        xs = [x for x in range(400, 540) if rb(off.rgba, x, y) > 60]
        if xs and xs[-1] - xs[0] > 30:
            left.append((xs[0], y))
    for x in range(445, 490):
        ys = [y for y in range(150, 230) if rb(off.rgba, x, y) > 60]
        if ys:
            top.append((x, ys[0]))
    if not left or not top:
        return None

    def band(points, dx, dy):
        return sum(rb(on.rgba, x + dx, y + dy) - rb(off.rgba, x + dx, y + dy) for x, y in points) / len(points)
    return {"trailing_1px": round(band(left, -1, 0), 2), "top_1px": round(band(top, 0, -1), 2),
            "samples": [len(left), len(top)]}


def registration_linear(on, off):
    """Global iterative Lucas-Kanade translation on linear-light luminance (x255) over
    strong-gradient reference pixels; positive = 'on' content right/down of reference."""
    w, h = off.w, off.h
    a = [v * 255 for v in on.lum]
    b = [v * 255 for v in off.lum]

    def sample(x, y):
        x = min(max(x, 0.0), w - 1.001)
        y = min(max(y, 0.0), h - 1.001)
        x0, y0 = int(x), int(y)
        fx, fy = x - x0, y - y0
        i = y0 * w + x0
        return b[i] * (1 - fx) * (1 - fy) + b[i + 1] * fx * (1 - fy) + b[i + w] * (1 - fx) * fy + b[i + w + 1] * fx * fy
    pts = [(x, y) for y in range(2, h - 2) for x in range(2, w - 2)
           if ((b[y * w + x + 1] - b[y * w + x - 1]) / 2) ** 2 + ((b[(y + 1) * w + x] - b[(y - 1) * w + x]) / 2) ** 2 > 100.0]
    s = [0.0, 0.0]
    for _ in range(6):
        g11 = g12 = g22 = e1 = e2 = 0.0
        for x, y in pts:
            r = sample(x - s[0], y - s[1])
            gx = (sample(x + 1 - s[0], y - s[1]) - sample(x - 1 - s[0], y - s[1])) / 2
            gy = (sample(x - s[0], y + 1 - s[1]) - sample(x - s[0], y - 1 - s[1])) / 2
            t = a[y * w + x] - r
            g11 += gx * gx
            g12 += gx * gy
            g22 += gy * gy
            e1 += gx * t
            e2 += gy * t
        det = g11 * g22 - g12 * g12
        if abs(det) < 1e-9:
            break
        dx = -(g22 * e1 - g12 * e2) / det
        dy = -(g11 * e2 - g12 * e1) / det
        s[0] += dx
        s[1] += dy
        if abs(dx) + abs(dy) < 1e-4:
            break
    return {"dx": round(s[0], 3), "dy": round(s[1], 3), "points": len(pts)}


# ------------------------------------------------------------------ runs

class Run:
    """One run's reports, PNGs and build records, located by name."""

    def __init__(self, label, reports, pngs, build, manifest=None, evidence=None):
        self.label, self.reports, self.pngs, self.build, self.manifest = label, reports, pngs, build, manifest
        self.evidence = Path(evidence) if evidence is not None else None
        self._images = {}

    def report(self, case):
        path = self.reports.get(case)
        return json.loads(Path(path).read_text()) if path and Path(path).is_file() else None

    def image(self, name):
        if name not in self._images:
            path = self.pngs.get(name)
            self._images[name] = Image(path) if path and Path(path).is_file() else None
        return self._images[name]

    def record(self, name):
        path = self.build.get(name)
        return json.loads(Path(path).read_text()) if path and Path(path).is_file() else None


def run_from_captures(label, evidence, cases):
    """A live run: captures/<case>/report.json and PNGs beside it; build/*.json."""
    evidence = Path(evidence)
    reports = {c: evidence / "captures" / c / "report.json" for c in cases}
    pngs = {p.name: p for c in cases for p in sorted((evidence / "captures" / c).glob("*.png"))}
    build = {p.name: p for p in sorted((evidence / "build").glob("*.json"))}
    return Run(label, reports, pngs, build, evidence=evidence)


def run_from_published(label, evidence, screenshots, cases):
    """A published run: captures/<case>/report.json (or reports/<case>.json), PNGs in
    the screenshots directory, build/*.json and the publication manifest."""
    evidence, screenshots = Path(evidence), Path(screenshots)
    reports = {}
    for c in cases:
        for candidate in (evidence / "captures" / c / "report.json", evidence / "reports" / f"{c}.json"):
            if candidate.is_file():
                reports[c] = candidate
                break
    pngs = {p.name: p for p in sorted(screenshots.glob("*.png"))}
    build = {p.name: p for p in sorted((evidence / "build").glob("*.json"))}
    manifest = evidence / "manifest.json" if (evidence / "manifest.json").is_file() else None
    return Run(label, reports, pngs, build, manifest, evidence)


def report_frames(run, cases, history_free):
    """{case/frame: rgba_sha256} for temporal-off frames (temporal null) or for
    history-free temporal frames (has_previous false, not bypassed)."""
    frames = {}
    for case in cases:
        for frame in (run.report(case) or {}).get("frames", []):
            if "temporal" not in frame:
                continue
            t = frame["temporal"]
            fresh = isinstance(t, dict) and t.get("has_previous") is False and not t.get("bypassed")
            if (fresh if history_free else t is None):
                frames[f"{case}/{frame.get('name')}"] = frame.get("rgba_sha256")
    return frames


def manifest_entries(manifest):
    entries = {}
    if manifest is not None:
        body = json.loads(Path(manifest).read_text())
        entries.update({e["path"]: e["sha256"] for e in body.get("files", [])})
        entries.update({e["name"]: e["sha256"] for e in body.get("screenshots", [])})
    return entries


def png_report_hash(run, cases, name):
    found = None
    for case in cases:
        for png in (run.report(case) or {}).get("pngs", []):
            if png.get("name") == name:
                found = png.get("rgba_sha256")
    return found


# ------------------------------------------------------------------ comparison

QUALITY = ("sharpness", "stability", "ghosts", "exclusion", "reset", "registration", "viewport")


def compare(candidate, baseline, plan, sections=None, prior=None):
    spec = plan["comparison"]
    checks = []
    context = {}

    def check(section, name, value, limit, ok):
        checks.append({"section": section, "check": name, "value": value, "limit": limit, "pass": bool(ok)})
        return bool(ok)

    want = set(sections or ("exactness", "sharpness", "stability", "ghosts", "exclusion", "reset",
                            "registration", "viewport"))
    base_cases = spec["baseline"]["cases"]

    # -------------------------------------------------- exactness
    if "exactness" in want:
        b = spec["baseline"]
        man = baseline.manifest
        check("exactness", "baseline_manifest_sha256", sha(man) if man else None, b["manifest_sha256"],
              man is not None and sha(man) == b["manifest_sha256"])
        entries = {}
        if man is not None:
            body = json.loads(Path(man).read_text())
            entries.update({e["path"]: e["sha256"] for e in body.get("files", [])})
            entries.update({e["name"]: e["sha256"] for e in body.get("screenshots", [])})
        used = [(f"captures/{c}/report.json", baseline.reports.get(c)) for c in base_cases]
        used += [(f"build/{n}", baseline.build.get(n)) for n in ("source.json", "review.json", "toolchain.json")]
        used += [(n, baseline.pngs.get(n)) for n in b["pngs"]]
        mismatched = [rel for rel, path in used
                      if not path or not Path(path).is_file() or entries.get(rel) != sha(path)]
        check("exactness", "baseline_files_match_manifest", mismatched, [], not mismatched)
        source = baseline.record("source.json") or {}
        review = baseline.record("review.json") or {}
        frozen = (review.get("frozen") or {}).get("temporal_visual_review", {})
        check("exactness", "baseline_identity",
              [source.get("source_commit"), (review.get("tools") or {}).get("visual_tree"), frozen.get("sha256")],
              [b["source_commit"], b["visual_tree"], b["review_binary_sha256"]],
              [source.get("source_commit"), (review.get("tools") or {}).get("visual_tree"), frozen.get("sha256")]
              == [b["source_commit"], b["visual_tree"], b["review_binary_sha256"]])
        csource = candidate.record("source.json") or {}
        check("exactness", "candidate_source_commit", csource.get("source_commit"), spec["candidate_commit"],
              csource.get("source_commit") == spec["candidate_commit"])
        same_off = {}
        for name in b["identical_references"]:
            values = []
            for run in (candidate, baseline):
                found = None
                for c in base_cases:
                    rep = run.report(c) or {}
                    for png in rep.get("pngs", []):
                        if png.get("name") == name:
                            found = png.get("rgba_sha256")
                values.append(found)
            same_off[name] = values[0] is not None and values[0] == values[1]
        check("exactness", "temporal_off_references_identical", same_off, "all true", all(same_off.values()))

        def adapter(run):
            rep = run.report("probe") or {}
            a = (rep.get("renderer") or {}).get("adapter") or {}
            return [a.get("name"), a.get("backend"), a.get("device_type")]
        check("exactness", "same_adapter", [adapter(candidate), adapter(baseline)], "equal",
              adapter(candidate) == adapter(baseline) and adapter(candidate)[0] is not None)

        def toolchain(run):
            t = run.record("toolchain.json") or {}
            cmds = t.get("commands", {})
            first = {k: (cmds.get(k, {}).get("stdout") or "").strip().splitlines()[:2]
                     for k in ("rustc", "cargo", "sw_vers", "hw_model")}
            return first
        check("exactness", "same_toolchain_and_host", [toolchain(candidate), toolchain(baseline)], "equal",
              toolchain(candidate) == toolchain(baseline) and bool(toolchain(candidate)["rustc"]))
        def config_ok(run):
            """Every report's review_config equals the plan's at f32 precision."""
            want_config = plan["review_config"]
            for c in base_cases:
                got = (run.report(c) or {}).get("review_config") or {}
                if set(got) != set(want_config) or any(
                        abs(float(got[k]) - float(want_config[k])) > 1e-6 * max(1.0, abs(float(want_config[k])))
                        for k in want_config):
                    return False
            return True
        configs = {run.label: config_ok(run) for run in (candidate, baseline)}
        check("exactness", "same_review_config", configs, "every report equals plan review_config (f32)",
              all(configs.values()))

        ps = spec.get("prior")
        if ps and prior is not None:
            man = prior.manifest
            check("exactness", "prior_manifest_sha256", sha(man) if man else None, ps["manifest_sha256"],
                  man is not None and sha(man) == ps["manifest_sha256"])
            entries = manifest_entries(man)
            record = prior.evidence / ps["compare_record"] if prior.evidence is not None else None
            used = [(f"captures/{c}/report.json", prior.reports.get(c)) for c in ps["cases"]]
            used += [(f"build/{n}", prior.build.get(n)) for n in ("source.json", "review.json", "toolchain.json")]
            used += [(ps["compare_record"], record)]
            used += [(n, prior.pngs.get(n)) for n in ps["pngs"]]
            mismatched = [rel for rel, path in used
                          if not path or not Path(path).is_file() or entries.get(rel) != sha(path)]
            check("exactness", "prior_files_match_manifest", mismatched, [], not mismatched)
            psource = prior.record("source.json") or {}
            preview = prior.record("review.json") or {}
            pfrozen = (preview.get("frozen") or {}).get("temporal_visual_review", {})
            identity = [psource.get("source_commit"), (preview.get("tools") or {}).get("visual_tree"),
                        pfrozen.get("sha256")]
            wanted = [ps["source_commit"], ps["visual_tree"], ps["review_binary_sha256"]]
            check("exactness", "prior_identity", identity, wanted, identity == wanted)
            recorded = (json.loads(record.read_text()).get("failed")
                        if record is not None and record.is_file() else None)
            check("exactness", "prior_recorded_failures", recorded, ps["recorded_quality_failures"],
                  recorded == ps["recorded_quality_failures"])
            order = plan["review"]["order"]
            mine, theirs = report_frames(candidate, order, False), report_frames(prior, ps["cases"], False)
            pngs = {n: png_report_hash(candidate, order, n) is not None
                    and png_report_hash(candidate, order, n) == png_report_hash(prior, ps["cases"], n)
                    for n in ps["identical_references"]}
            check("exactness", "temporal_off_identical_to_prior",
                  {"pngs": pngs, "off_frames": len(mine), "off_frames_identical": bool(mine) and mine == theirs},
                  "all temporal-off PNGs and frames identical", all(pngs.values()) and bool(mine) and mine == theirs)
            check("exactness", "prior_same_adapter", [adapter(candidate), adapter(prior)], "equal",
                  adapter(candidate) == adapter(prior) and adapter(prior)[0] is not None)
            check("exactness", "prior_same_toolchain_and_host", [toolchain(candidate), toolchain(prior)], "equal",
                  toolchain(candidate) == toolchain(prior) and bool(toolchain(prior)["rustc"]))
            check("exactness", "prior_same_review_config", config_ok(prior),
                  "every prior report equals plan review_config (f32)", config_ok(prior))

    def pair(run, on_name, off_name):
        return run.image(on_name), run.image(off_name)

    # -------------------------------------------------- sharpness
    if "sharpness" in want:
        s = spec["sharpness"]
        on, off = pair(candidate, "stationary-on-f16.png", "stationary-off.png")
        bon, boff = pair(baseline, "stationary-on-f16.png", "stationary-off.png")
        if on is None or off is None:
            check("sharpness", "stationary_images_present", None, "present", False)
        else:
            for region, box in s["regions"].items():
                ratio = detail(on.lum, on.w, box) / detail(off.lum, off.w, box)
                sigma = sigma_equivalent(on, off, box)
                if bon is not None and boff is not None:
                    context[f"baseline_{region}"] = {
                        "detail_ratio": round(detail(bon.lum, bon.w, box) / detail(boff.lum, boff.w, box), 3),
                        "sigma_px": sigma_equivalent(bon, boff, box)}
                if region == s["detail_region"]:
                    check("sharpness", f"{region}_detail_ratio", round(ratio, 3), f">= {s['detail_min']}",
                          ratio >= s["detail_min"])
                check("sharpness", f"{region}_sigma_px", sigma, f"<= {s['sigma_max_px']}", sigma <= s["sigma_max_px"])

    # -------------------------------------------------- stability
    if "stability" in want:
        t = spec["stability"]
        rep = candidate.report("stationary")
        act, off = candidate.image("stationary-activity.png"), candidate.image("stationary-off.png")
        if rep is None or act is None or off is None:
            check("stability", "stationary_outputs_present", None, "present", False)
        else:
            bins = activity_bins(act, edge_mask(off))
            check("stability", "stationary_activity_ge33", bins["ge33"], f"<= {t['activity_ge33_max']}",
                  bins["ge33"] <= t["activity_ge33_max"])
            check("stability", "stationary_activity_saturated", bins["saturated"], f"<= {t['saturated_max']}",
                  bins["saturated"] <= t["saturated_max"])
            late = rep["observations"]["consecutive"][7:]
            emean = max(c["delta"]["edge"]["mean_delta"] for c in late)
            emax = max(c["delta"]["edge"]["max_delta"] for c in late)
            ichg = max(c["delta"]["interior"]["changed_over_threshold"] for c in late)
            check("stability", "frames_9_16_edge_mean", round(emean, 3), f"<= {t['edge_mean_max']}", emean <= t["edge_mean_max"])
            check("stability", "frames_9_16_edge_max", emax, f"<= {t['edge_max_max']}", emax <= t["edge_max_max"])
            check("stability", "frames_9_16_interior_changed", ichg, f"<= {t['interior_changed_max']}",
                  ichg <= t["interior_changed_max"])
            same = rep["observations"]["same_phase_f08_vs_f16"]["interior"]["changed_over_threshold"]
            check("stability", "same_phase_f08_f16_interior_changed", same, f"<= {t['same_phase_interior_max']}",
                  same <= t["same_phase_interior_max"])
        for on_name, off_name in t["halo_pairs"]:
            on, off = pair(candidate, on_name, off_name)
            if on is None or off is None:
                check("stability", f"halo_{on_name}", None, "present", False)
                continue
            h = halo(on, off, t["halo_steps"])
            check("stability", f"halo_{on_name}", h, f"above_local_max <= {t['halo_pixels_max']}",
                  h["above_local_max"] <= t["halo_pixels_max"])
            bon, boff = pair(baseline, on_name, off_name)
            if bon is not None and boff is not None:
                context[f"baseline_halo_{on_name}"] = halo(bon, boff, t["halo_steps"])

    # -------------------------------------------------- ghosts
    if "ghosts" in want:
        g = spec["ghosts"]
        for on_name, off_name in g["pairs"]:
            on, off = pair(candidate, on_name, off_name)
            bon, boff = pair(baseline, on_name, off_name)
            if None in (on, off, bon, boff):
                check("ghosts", f"off_edge_{on_name}", None, "present in both runs", False)
                continue
            mine, theirs = off_edge_bins(on, off), off_edge_bins(bon, boff)
            check("ghosts", f"off_edge_{on_name}", mine, {"baseline": theirs, "rule": "each bin <= baseline"},
                  all(mine[k] <= theirs[k] for k in ("3-8", "9-16", ">16")))
        on, off = pair(candidate, "motion-object-on-f8.png", "motion-object-off-f8.png")
        bands = cube_bands(on, off) if on is not None and off is not None else None
        bon, boff = pair(baseline, "motion-object-on-f8.png", "motion-object-off-f8.png")
        base_bands = cube_bands(bon, boff) if bon is not None and boff is not None else None
        if bands is None:
            check("ghosts", "trailing_band_vs_top_control", None, "measurable", False)
        else:
            limit = g["trailing_over_top_max"] * bands["top_1px"] if bands["top_1px"] > 0 else 0.0
            check("ghosts", "trailing_band_vs_top_control", bands, f"trailing_1px <= {g['trailing_over_top_max']} x top_1px",
                  bands["trailing_1px"] <= limit)
        if g.get("trailing_no_worse_than_baseline"):
            check("ghosts", "trailing_band_no_worse_than_baseline",
                  [bands and bands["trailing_1px"], base_bands and base_bands["trailing_1px"]],
                  "candidate trailing_1px <= baseline trailing_1px",
                  bands is not None and base_bands is not None and bands["trailing_1px"] <= base_bands["trailing_1px"])

    # -------------------------------------------------- exclusion
    if "exclusion" in want:
        rep = candidate.report("excluded")
        act, off = candidate.image("excluded-activity.png"), candidate.image("excluded-off.png")
        ui = ((rep or {}).get("observations") or {}).get("ui_pixels_identical_to_off")
        check("exclusion", "ui_identical_all_phases", ui, "8 x true", isinstance(ui, list) and len(ui) == 8 and all(ui))
        if act is None or off is None:
            check("exclusion", "excluded_outputs_present", None, "present", False)
        else:
            bins = activity_bins(act, edge_mask(off))
            check("exclusion", "overlay_activity_ge33", bins["ge33"], f"<= {spec['exclusion']['activity_ge33_max']}",
                  bins["ge33"] <= spec["exclusion"]["activity_ge33_max"])

    # -------------------------------------------------- reset
    if "reset" in want:
        for name in spec["reset"]["identical_to_baseline"]:
            a, b = candidate.image(name), baseline.image(name)
            value = max_step(a, b) if a is not None and b is not None and (a.w, a.h) == (b.w, b.h) else None
            check("reset", f"{name}_max_step_vs_baseline", value, f"<= {spec['reset']['max_step']}",
                  value is not None and value <= spec["reset"]["max_step"])
        hf = spec.get("history_free_frames")
        if hf:
            order = plan["review"]["order"]
            mine = report_frames(candidate, order, True)
            refs = {label: run for label, run in (("prior", prior), ("baseline", baseline))
                    if label in hf["compare_to"] and run is not None}
            theirs = {label: report_frames(run, order, True) for label, run in refs.items()}
            rows = {key: {label: (frames[key] == digest) if key in frames else None
                          for label, frames in theirs.items()} for key, digest in mine.items()}
            ok = (bool(mine) and all(v is not False for row in rows.values() for v in row.values())
                  and ("prior" not in theirs or set(theirs["prior"]) == set(mine)))
            check("reset", "history_free_frames_identical",
                  {"frames": len(mine), "compared": {label: sum(1 for row in rows.values() if row[label] is not None)
                                                     for label in theirs},
                   "differing": sorted(k for k, row in rows.items() if False in row.values()),
                   "missing_from_prior": sorted(set(theirs.get("prior", mine)) ^ set(mine))},
                  "every history-free frame byte-identical to " + " and ".join(theirs), ok)

    # -------------------------------------------------- registration
    if "registration" in want:
        r = spec["registration"]
        for on_name, off_name in r["pairs"]:
            on, off = pair(candidate, on_name, off_name)
            if on is None or off is None:
                check("registration", on_name, None, "present", False)
                continue
            reg = registration_linear(on, off)
            check("registration", on_name, reg, f"|dx|,|dy| <= {r['max_px']}",
                  abs(reg["dx"]) <= r["max_px"] and abs(reg["dy"]) <= r["max_px"])
            bon, boff = pair(baseline, on_name, off_name)
            if bon is not None and boff is not None:
                context[f"baseline_registration_{on_name}"] = registration_linear(bon, boff)

    # -------------------------------------------------- viewport (candidate only)
    if "viewport" in want:
        v = spec["viewport"]
        rep = candidate.report("viewport")
        if rep is None:
            check("viewport", "viewport_report_present", None, "present", False)
        else:
            obs = rep["observations"]
            bins = bins_from_report(obs["still_activity_frames_9_16"])
            check("viewport", "still_activity_ge33", bins["ge33"], f"<= {v['activity_ge33_max']}",
                  bins["ge33"] <= v["activity_ge33_max"])
            check("viewport", "still_activity_saturated", bins["saturated"], f"<= {v['saturated_max']}",
                  bins["saturated"] <= v["saturated_max"])
            late = obs["still_consecutive"][7:]
            emean = max(c["delta"]["edge"]["mean_delta"] for c in late)
            emax = max(c["delta"]["edge"]["max_delta"] for c in late)
            ichg = max(c["delta"]["interior"]["changed_over_threshold"] for c in late)
            check("viewport", "still_frames_9_16_edge_mean", round(emean, 3), f"<= {v['edge_mean_max']}", emean <= v["edge_mean_max"])
            check("viewport", "still_frames_9_16_edge_max", emax, f"<= {v['edge_max_max']}", emax <= v["edge_max_max"])
            check("viewport", "still_frames_9_16_interior_changed", ichg, f"<= {v['interior_changed_max']}",
                  ichg <= v["interior_changed_max"])
            same = obs["still_same_phase_f08_vs_f16"]["interior"]["changed_over_threshold"]
            check("viewport", "still_same_phase_interior_changed", same, f"<= {v['same_phase_interior_max']}",
                  same <= v["same_phase_interior_max"])
            for key in ("still_f16_border_vs_off", "pan_f24_border_vs_off"):
                split = obs[key]
                band, inner = split["border_band_3px"]["max_delta"], split["inner"]["max_delta"]
                check("viewport", f"{key}_band_max_not_above_inner", [band, inner], "band <= inner", band <= inner)
            check("viewport", "outside_viewport_max_delta", obs["outside_viewport_max_delta"], "== 0",
                  obs["outside_viewport_max_delta"] == 0)
            on, off = pair(candidate, "viewport-on-f24.png", "viewport-off-f24.png")
            if on is None or off is None:
                check("viewport", "pan_images_present", None, "present", False)
            else:
                bins = off_edge_bins(on, off)
                check("viewport", "pan_off_edge_ge9", bins, "9-16 == 0 and >16 == 0",
                      bins["9-16"] == 0 and bins[">16"] == 0)

    ps = spec.get("prior")
    quality = [s for s in QUALITY if s in want]
    if ps and prior is not None and quality:
        # The same code on the prior run, over the same quality sections; prior=None
        # stops the recursion. Its failures must equal the recorded ones there.
        earlier = compare(prior, baseline, plan, sections=quality)
        recomputed = earlier["failed"]
        expected = [f for f in ps["recorded_quality_failures"] if f.split(":")[0] in quality]
        if "exactness" in want:
            check("exactness", "prior_recomputed_failures_match_record", recomputed, expected,
                  recomputed == expected)
        mine = {f"{c['section']}: {c['check']}": c for c in checks if c["section"] != "exactness"}
        theirs = {f"{c['section']}: {c['check']}": c for c in earlier["checks"]}
        context["prior_effect"] = {key: {"prior": theirs[key]["value"] if key in theirs else None,
                                         "candidate": c["value"],
                                         "prior_pass": theirs[key]["pass"] if key in theirs else None,
                                         "candidate_pass": c["pass"]} for key, c in mine.items()}
        now_failed = [key for key, c in mine.items() if not c["pass"]]
        context["failure_history"] = {
            "baseline_known_failures": ps.get("baseline_known_failures", []),
            "prior_recorded": ps["recorded_quality_failures"], "prior_recomputed": recomputed,
            "resolved": [k for k in ps["recorded_quality_failures"] if k in mine and mine[k]["pass"]],
            "persisting": [k for k in ps["recorded_quality_failures"] if k in now_failed],
            "new": [k for k in now_failed if k not in ps["recorded_quality_failures"]]}
    exact = all(c["pass"] for c in checks if c["section"] == "exactness")
    failed = [f"{c['section']}: {c['check']}" for c in checks if not c["pass"]]
    return {"verdict": "pass" if checks and not failed else "fail", "exact": exact,
            "failed": failed, "checks": checks, "baseline_context": context,
            "sections": sorted(want)}
