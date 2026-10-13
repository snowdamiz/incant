#!/usr/bin/env python3
"""CPU-only review aids for handoff 0075 (corrected bloom). Standard library only.

Reads retained 0070 helper outputs (never writes into an evidence directory),
verifies every file against its index hash, and derives bloom-only
measurements and review figures. Nothing here renders, touches a GPU or
judges a gate; the 0070 analyzer owns the numeric gates.

  python3 -I -B review.py verify   <outputs/appearance>
  python3 -I -B review.py builds   <old outputs/<label>> <new outputs/<label>>
  python3 -I -B review.py halo     <outputs/case-lifecycle_drs>
  python3 -I -B review.py spread   <outputs/appearance>
  python3 -I -B review.py texture  <outputs/appearance>
  python3 -I -B review.py figures  <evidence dir> <output dir> [<previous evidence dir>]
  python3 -I -B review.py execution <evidence dir>
  python3 -I -B review.py retain   <evidence dir> <output dir> <relative path>...
"""

import gzip
import hashlib
import json
import math
import struct
import sys
import zlib
from pathlib import Path

sys.dont_write_bytecode = True

# The lifecycle_drs fixture (analyze.py RECT/SIZE and project()).
RECT = (13.25, 17.5, 280.5, 160.5)
SIZE = (321, 193)
EMITTER = (0.9, 0.4, -4.0)
FAR_FIELD_PX = 48
NEAR_FIELD_PX = 24


def srgb_to_linear(s):
    s = min(max(s, 0.0), 1.0)
    return s / 12.92 if s <= 0.04045 else ((s + 0.055) / 1.055) ** 2.4


LINEAR = [srgb_to_linear(b / 255) for b in range(256)]


def sha256(data):
    return hashlib.sha256(data).hexdigest()


# ------------------------------------------------------------------ PNG

def decode(path):
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path}: not a PNG")
    pos, idat, header = 8, bytearray(), None
    while pos < len(data):
        length, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + length]
        if kind == b"IHDR":
            header = struct.unpack(">IIBBBBB", body)
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    w, h, depth, color, _, _, interlace = header
    if (depth, color, interlace) != (8, 6, 0):
        raise ValueError(f"{path}: expected 8-bit RGBA non-interlaced, got {header}")
    raw, stride = zlib.decompress(bytes(idat)), w * 4
    out, prev = bytearray(), bytearray(stride)
    for y in range(h):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        if kind == 1:
            for i in range(4, stride):
                line[i] = (line[i] + line[i - 4]) & 255
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif kind == 3:
            for i in range(stride):
                left = line[i - 4] if i >= 4 else 0
                line[i] = (line[i] + (left + prev[i]) // 2) & 255
        elif kind == 4:
            for i in range(stride):
                a = line[i - 4] if i >= 4 else 0
                b = prev[i]
                c = prev[i - 4] if i >= 4 else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        elif kind != 0:
            raise ValueError(f"{path}: unknown filter {kind}")
        out += line
        prev = line
    return (w, h), bytes(out)


def encode(path, size, rgba):
    w, h = size
    raw = b"".join(b"\0" + rgba[y * w * 4:(y + 1) * w * 4] for y in range(h))

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


class Image:
    def __init__(self, size, data):
        self.size, self.data = tuple(size), data

    def px(self, x, y):
        i = (y * self.size[0] + x) * 4
        return self.data[i:i + 4]


def load_index(directory):
    return {e["label"]: e for e in json.loads((Path(directory) / "index.json").read_text())["frames"]}


def load_verified(directory, label):
    """One frame, PNG or raw RGBA, verified against every hash its index records."""
    entry = load_index(directory)[label]
    path = Path(directory) / entry["file"]
    if entry["file"].endswith(".png"):
        if sha256(path.read_bytes()) != entry["png_sha256"]:
            raise ValueError(f"{path}: PNG bytes do not match png_sha256")
        size, data = decode(path)
    else:
        size, data = tuple(entry["size"]), path.read_bytes()
    if list(size) != entry["size"] or len(data) != size[0] * size[1] * 4:
        raise ValueError(f"{path}: size does not match its index")
    if sha256(data) != entry["rgba_sha256"]:
        raise ValueError(f"{path}: decoded RGBA does not match rgba_sha256")
    return Image(size, data)


# ------------------------------------------------------------------ measurements

def project(point, rect=RECT):
    """analyze.py project(): the fixture camera, 1 rad vertical field of view."""
    x, y, w, h = rect
    f = 1 / math.tan(0.5)
    ndc = (point[0] / -point[2] * f / (w / h), point[1] / -point[2] * f)
    return (x + (ndc[0] * 0.5 + 0.5) * w, y + (0.5 - ndc[1] * 0.5) * h)


def interior(rect, size, margin):
    x0, y0 = math.ceil(rect[0] + margin), math.ceil(rect[1] + margin)
    x1, y1 = math.floor(rect[0] + rect[2] - margin), math.floor(rect[1] + rect[3] - margin)
    for y in range(max(y0, 0), min(y1, size[1])):
        for x in range(max(x0, 0), min(x1, size[0])):
            yield x, y


def halo_weights(on, off, rect):
    """Positive linear RGB change on non-emitter pixels inside the viewport
    interior: the same pixel set and weight as analyze.py centroid()."""
    backdrop = off.px(int(rect[0] + 3), int(rect[1] + 3))[:3]
    out = []
    for x, y in interior(rect, on.size, 2):
        o = off.px(x, y)
        if o[:3] != backdrop:
            continue
        p = on.px(x, y)
        w = sum(LINEAR[p[c]] - LINEAR[o[c]] for c in range(3))
        if w > 0:
            out.append((x + 0.5, y + 0.5, w))
    return out


def moments(weights, centre):
    total = sum(w for _, _, w in weights)
    if total == 0:
        return None
    cx = sum(x * w for x, _, w in weights) / total
    cy = sum(y * w for _, y, w in weights) / total
    r2 = sum(((x - centre[0]) ** 2 + (y - centre[1]) ** 2) * w for x, y, w in weights) / total
    far = sum(w for x, y, w in weights if math.hypot(x - centre[0], y - centre[1]) > FAR_FIELD_PX) / total
    near = [(x, y, w) for x, y, w in weights if math.hypot(x - centre[0], y - centre[1]) <= NEAR_FIELD_PX]
    near_total = sum(w for _, _, w in near)
    near_c = None if near_total == 0 else (sum(x * w for x, _, w in near) / near_total,
                                           sum(y * w for _, y, w in near) / near_total)
    left = sum(w for x, _, w in weights if x < centre[0]) / total
    up = sum(w for _, y, w in weights if y < centre[1]) / total
    return {"energy": total, "centroid": [cx, cy], "centroid_offset_px": math.hypot(cx - centre[0], cy - centre[1]),
            "rms_radius_px": math.sqrt(r2), "share_beyond_48px": far, "left_right": [left, 1 - left],
            "up_down": [up, 1 - up], "near_field_24px_centroid": near_c,
            "near_field_offset_px": None if near_c is None else math.hypot(near_c[0] - centre[0], near_c[1] - centre[1])}


def command_halo(directory):
    expected = project(EMITTER)
    out = {"expected_centre": expected, "metric": "positive linear RGB change of e_bloom_N over e_off_N on "
           "non-emitter interior pixels (analyze.py centroid pixel set); radii about the expected centre; "
           "emitter_pixels = interior pixels of e_off_N that are not the backdrop",
           "scales": {}}
    for n in (16, 12, 8):
        on, off = load_verified(directory, f"e_bloom_{n}"), load_verified(directory, f"e_off_{n}")
        result = moments(halo_weights(on, off, RECT), expected)
        backdrop = off.px(int(RECT[0] + 3), int(RECT[1] + 3))[:3]
        emitter = sum(1 for x, y in interior(RECT, off.size, 2) if off.px(x, y)[:3] != backdrop)
        result["emitter_pixels"] = emitter
        result["energy_per_emitter_pixel"] = result["energy"] / emitter
        out["scales"][n] = result
    return out


TEXTURE_REGION = (440, 200, 660, 262)  # dark backdrop above the sphere, inside the v1/v3 glow


def command_texture(directory):
    """Nearest-expanded glow would repeat within 2x2 blocks at 8/16: horizontal
    steps of the bloom change between even-x neighbours would then vanish
    while odd-x (block boundary) steps remain. Pairs crossing a geometry edge
    of the off render are skipped."""
    out = {"region": TEXTURE_REGION, "metric": "mean |step| of the summed RGB code change between horizontal "
           "neighbours, split by even or odd x", "pairs": {}}
    for on_label, off_label in (("v1_both", "v1_off"), ("v3_both_12", "v3_off_12"), ("v3_both_8", "v3_off_8")):
        on, off = load_verified(directory, on_label), load_verified(directory, off_label)
        x0, y0, x1, y1 = TEXTURE_REGION
        sums, counts = [0, 0], [0, 0]
        for y in range(y0, y1):
            for x in range(x0, x1 - 1):
                a, b = off.px(x, y)[:3], off.px(x + 1, y)[:3]
                if a != b:
                    continue
                d0 = sum(on.px(x, y)[c] - a[c] for c in range(3))
                d1 = sum(on.px(x + 1, y)[c] - b[c] for c in range(3))
                sums[x % 2] += abs(d0 - d1)
                counts[x % 2] += 1
        out["pairs"][f"{on_label}-{off_label}"] = {"even_x": sums[0] / counts[0], "odd_x": sums[1] / counts[1],
                                                   "pairs_compared": counts}
    return out


def brighter(on, off):
    """Positive linear RGB change of `on` over `off`, per pixel."""
    w, h = on.size
    out = []
    for y in range(h):
        row = y * w * 4
        for x in range(w):
            i = row + x * 4
            d = sum(LINEAR[on.data[i + c]] - LINEAR[off.data[i + c]] for c in range(3))
            if d > 0:
                out.append((x + 0.5, y + 0.5, d))
    return out


def spread(weights):
    total = sum(w for _, _, w in weights)
    cx = sum(x * w for x, _, w in weights) / total
    cy = sum(y * w for _, y, w in weights) / total
    r = math.sqrt(sum(((x - cx) ** 2 + (y - cy) ** 2) * w for x, y, w in weights) / total)
    return {"energy": total, "centroid": [cx, cy], "rms_radius_px": r, "pixels": len(weights)}


APPEARANCE_PAIRS = [("v1_bloom", "v1_off", 16), ("v1_both", "v1_off", 16), ("v1_both", "v1_ssao", 16),
                    ("v3_both_12", "v3_off_12", 12), ("v3_both_8", "v3_off_8", 8)]


def command_spread(directory):
    out = {"metric": "rms radius about the weighted centroid of the positive linear RGB change, whole frame",
           "pairs": {}}
    cache = {}
    for on, off, n in APPEARANCE_PAIRS:
        for label in (on, off):
            if label not in cache:
                cache[label] = load_verified(directory, label)
        out["pairs"][f"{on}-{off}"] = {"scale16": n, **spread(brighter(cache[on], cache[off]))}
    return out


def command_verify(directory):
    out = {}
    for label, entry in sorted(load_index(directory).items()):
        image = load_verified(directory, label)
        out[label] = {"file": entry["file"], "size": list(image.size), "png_sha256": entry.get("png_sha256"),
                      "rgba_sha256": entry["rgba_sha256"], "verified": True}
    return out


def file_matches_index(directory, entry):
    """The stored file's own bytes against its index hash, without decoding."""
    data = (Path(directory) / entry["file"]).read_bytes()
    return sha256(data) == entry["png_sha256" if entry["file"].endswith(".png") else "rgba_sha256"]


def command_builds(old, new):
    a, b = load_index(old), load_index(new)
    out = {}
    for label in sorted(set(a) | set(b)):
        if label not in a or label not in b:
            out[label] = {"present_in": "new" if label in b else "old"}
            continue
        if not (file_matches_index(old, a[label]) and file_matches_index(new, b[label])):
            raise ValueError(f"{label}: a stored file does not match its index hash")
        same = a[label]["rgba_sha256"] == b[label]["rgba_sha256"]
        entry = {"rgba_identical": same}
        if not same:
            x, y = load_verified(old, label), load_verified(new, label)
            entry["differing_pixels"] = sum(1 for i in range(0, len(x.data), 4) if x.data[i:i + 4] != y.data[i:i + 4])
        out[label] = entry
    return out


# ------------------------------------------------------------------ figures

def crop(image, rect, scale):
    x0, y0, cw, ch = rect
    out = bytearray()
    for y in range(ch * scale):
        sy = min(max(y0 + y // scale, 0), image.size[1] - 1)
        for x in range(cw * scale):
            sx = min(max(x0 + x // scale, 0), image.size[0] - 1)
            out += image.px(sx, sy)
    return Image((cw * scale, ch * scale), bytes(out))


def gain_map(on, off, gain):
    """Positive change only, mean linear RGB x gain, encoded as an sRGB gray ramp."""
    out = bytearray()
    for i in range(0, len(on.data), 4):
        d = sum(LINEAR[on.data[i + c]] - LINEAR[off.data[i + c]] for c in range(3)) / 3
        v = 0 if d <= 0 else min(255, round(255 * (min(1.0, d * gain) ** (1 / 2.2))))
        out += bytes((v, v, v, 255))
    return Image(on.size, bytes(out))


def mark(image, centre, radius, colour=(255, 64, 64)):
    """Draw a thin cross at `centre` (review annotation, not rendered content)."""
    data = bytearray(image.data)
    cx, cy = int(centre[0]), int(centre[1])
    for d in range(-radius, radius + 1):
        for x, y in ((cx + d, cy), (cx, cy + d)):
            if 0 <= x < image.size[0] and 0 <= y < image.size[1]:
                i = (y * image.size[0] + x) * 4
                data[i:i + 4] = bytes((*colour, 255))
    return Image(image.size, bytes(data))


def ring(image, centre, radius, colour=(64, 160, 255)):
    data = bytearray(image.data)
    steps = max(64, int(2 * math.pi * radius * 2))
    for k in range(steps):
        t = 2 * math.pi * k / steps
        x, y = int(centre[0] + radius * math.cos(t)), int(centre[1] + radius * math.sin(t))
        if 0 <= x < image.size[0] and 0 <= y < image.size[1]:
            i = (y * image.size[0] + x) * 4
            data[i:i + 4] = bytes((*colour, 255))
    return Image(image.size, bytes(data))


def hstack(images, gap=6, fill=(40, 40, 40)):
    h = max(i.size[1] for i in images)
    w = sum(i.size[0] for i in images) + gap * (len(images) - 1)
    out = bytearray()
    for y in range(h):
        for k, image in enumerate(images):
            if k:
                out += bytes((*fill, 255)) * gap
            if y < image.size[1]:
                out += image.data[y * image.size[0] * 4:(y + 1) * image.size[0] * 4]
            else:
                out += bytes((*fill, 255)) * image.size[0]
    return Image((w, h), bytes(out))


def vstack(images, gap=6, fill=(40, 40, 40)):
    w = max(i.size[0] for i in images)
    out = bytearray()
    for k, image in enumerate(images):
        if k:
            out += bytes((*fill, 255)) * (w * gap)
        for y in range(image.size[1]):
            out += image.data[y * image.size[0] * 4:(y + 1) * image.size[0] * 4]
            out += bytes((*fill, 255)) * (w - image.size[0])
    return Image((w, len(out) // (4 * w)), bytes(out))


def command_figures(evidence, target, previous=None):
    """Review figures from one evidence directory; with `previous` (build003)
    also before/after strips of the frames the correction changed."""
    evidence, target = Path(evidence), Path(target)
    target.mkdir(parents=True, exist_ok=True)
    appearance, lifecycle = evidence / "outputs" / "appearance", evidence / "outputs" / "case-lifecycle_drs"
    made = {}

    def save(name, image, op, sources):
        path = target / name
        if path.exists():
            raise FileExistsError(f"{path} exists; review figures are never replaced")
        encode(path, image.size, image.data)
        made[name] = {"op": op, "size": list(image.size), "sha256": sha256(path.read_bytes()),
                      "sources": [{"dir": str(d), "label": s, "rgba_sha256": load_index(d)[s]["rgba_sha256"]}
                                  for d, s in sources]}

    def halo_panel(directory, n):
        on, off = load_verified(directory, f"e_bloom_{n}"), load_verified(directory, f"e_off_{n}")
        g = ring(mark(gain_map(on, off, 24), expected, 6), expected, FAR_FIELD_PX)
        return crop(g, (0, 0, SIZE[0], SIZE[1]), 2)

    # 1. The centroid gate's own frames: bloom-only change at 16/12/8, x2 nearest,
    #    with the projected emitter centre (red cross) and the 48 px far-field ring.
    expected = project(EMITTER)
    save("drs_halo_lifecycle_16_12_8.png", hstack([halo_panel(lifecycle, n) for n in (16, 12, 8)]),
         "positive linear change e_bloom_N - e_off_N, mean RGB x24, gamma 2.2 gray; x2 nearest; red cross = "
         "projected emitter centre, blue ring = 48 px radius; panels 16/16, 12/16, 8/16",
         [(lifecycle, f"e_{k}_{n}") for n in (16, 12, 8) for k in ("bloom", "off")])
    if previous is not None:
        old = Path(previous) / "outputs" / "case-lifecycle_drs"
        rows = [hstack([halo_panel(old, n), halo_panel(lifecycle, n)]) for n in (12, 8)]
        save("drs_halo_lifecycle_before_after_12_8.png", vstack(rows),
             "as drs_halo_lifecycle_16_12_8.png; columns build003 (before) and build004 (after); rows 12/16, 8/16",
             [(d, f"e_{k}_{n}") for n in (12, 8) for d in (old, lifecycle) for k in ("bloom", "off")])

    # 2. The same frames at true contrast, cropped around the emitter, x3.
    cx, cy = int(expected[0]), int(expected[1])
    rect = (cx - 60, cy - 50, 120, 100)
    save("drs_emitter_true_contrast_16_12_8.png",
         hstack([mark(crop(load_verified(lifecycle, f"e_bloom_{n}"), rect, 3),
                      ((expected[0] - rect[0]) * 3, (expected[1] - rect[1]) * 3), 4) for n in (16, 12, 8)]),
         f"e_bloom_N at true contrast, crop {list(rect)}, x3 nearest, red cross = projected emitter centre; "
         "panels 16/16, 12/16, 8/16", [(lifecycle, f"e_bloom_{n}") for n in (16, 12, 8)])

    # 3. Appearance at 100/75/50 %: positive change of the composed renders over
    #    their own off render at one gain (the like-for-like `spread` pairs).
    images = {}

    def image(directory, label):
        key = (str(directory), label)
        if key not in images:
            images[key] = load_verified(directory, label)
        return images[key]

    pairs = [("v1_both", "v1_off"), ("v3_both_12", "v3_off_12"), ("v3_both_8", "v3_off_8")]
    save("appearance_bloom_gain_100_75_50.png",
         vstack([gain_map(image(appearance, on), image(appearance, off), 8) for on, off in pairs]),
         "positive linear change, mean RGB x8, gamma 2.2 gray; rows v1_both-v1_off (100 %), "
         "v3_both_12-v3_off_12 (75 %), v3_both_8-v3_off_8 (50 %)",
         [(appearance, label) for pair in pairs for label in pair])
    if previous is not None:
        old = Path(previous) / "outputs" / "appearance"
        region = (150, 110, 420, 280)
        rows = []
        for on, off in pairs[1:]:
            before = crop(gain_map(image(old, on), image(old, off), 8), region, 1)
            after = crop(gain_map(image(appearance, on), image(appearance, off), 8), region, 1)
            rows.append(hstack([before, after]))
        reference = crop(gain_map(image(appearance, "v1_both"), image(appearance, "v1_off"), 8), region, 1)
        save("appearance_bloom_gain_before_after.png", vstack([hstack([reference, reference]), *rows]),
             f"gain maps as appearance_bloom_gain_100_75_50.png, crop {list(region)}; columns build003 (before) "
             "and build004 (after); rows 100 % (v1, identical in both builds), 75 %, 50 %",
             [(d, label) for d in (old, appearance) for pair in pairs for label in pair])

    # 4. True-contrast crops at 100/75/50 % in a 2x2 grid: off | 100 % / 75 % | 50 %.
    for name, rect, scale in (("emissive_panel_box", (230, 160, 270, 150), 2), ("sphere_crate", (450, 240, 280, 170), 2)):
        tiles = [crop(image(appearance, l), rect, scale) for l in ("v1_off", "v1_both", "v3_both_12", "v3_both_8")]
        save(f"crop_{name}_grid.png", vstack([hstack(tiles[:2]), hstack(tiles[2:])]),
             f"true contrast crop {list(rect)} x{scale} nearest; grid v1_off | v1_both (100 %) / "
             "v3_both_12 (75 %) | v3_both_8 (50 %)",
             [(appearance, l) for l in ("v1_off", "v1_both", "v3_both_12", "v3_both_8")])
    return made


# ------------------------------------------------------------------ execution record and retained copies

DEVICE_LABELS = ("list-lib", "suite-lib", "case-ssao_core", "case-ssao_materials", "case-ssao_geometry",
                 "case-lifecycle_drs", "case-bloom_numeric", "appearance")


def command_execution(evidence):
    """Per-child facts from the runner's own ledger, in launch order."""
    evidence = Path(evidence)
    records = [json.loads(line) for line in (evidence / "ledger.jsonl").read_text().splitlines() if line.strip()]
    children = {}
    for r in records:
        if r.get("label") in DEVICE_LABELS:
            entry = children.setdefault(r["label"], {})
            if r["type"] in ("reserve", "spawned", "complete"):
                entry[r["type"]] = r
    out = []
    for label in DEVICE_LABELS:
        entry = children.get(label, {})
        reserve, spawned, done = entry.get("reserve", {}), entry.get("spawned", {}), entry.get("complete", {})
        outcome = done.get("outcome") or {}
        post = done.get("postprocess") or {}
        out.append({
            "label": label, "kind": reserve.get("kind"), "argv": reserve.get("argv"),
            "env_keys": (reserve.get("env") or {}).get("keys"), "binary": reserve.get("binary"),
            "timeout_s": reserve.get("timeout_s"), "existing_frames": reserve.get("existing_frames", 0),
            "new_frames": reserve.get("new_frames", 0), "pngs": reserve.get("pngs", 0),
            "pid": spawned.get("pid"), "identity": spawned.get("identity"),
            "started": outcome.get("started"), "ended": outcome.get("ended"), "exit": outcome.get("exit"),
            "signal": outcome.get("signal"), "timed_out": outcome.get("timed_out"),
            "group_absent": outcome.get("group_absent"), "survivors": outcome.get("survivors"),
            "binary_unchanged": done.get("binary_unchanged"), "success": done.get("success"),
            "failure_reasons": done.get("failure_reasons"),
            "stdout_sha256": done.get("stdout_sha256"), "stderr_sha256": done.get("stderr_sha256"),
            "verdict": {k: post.get(k) for k in ("valid", "problems", "frames", "declared", "index", "receipts",
                                                 "listed", "expected", "passed", "failed", "summary")
                        if k in post}})
    totals = {"children": len([c for c in out if c["success"] is not None]),
              "existing_frames": sum(c["existing_frames"] for c in out),
              "new_frames": sum(c["new_frames"] for c in out), "pngs": sum(c["pngs"] for c in out),
              "succeeded": sum(1 for c in out if c["success"] is True)}
    return {"ledger_sha256": sha256((evidence / "ledger.jsonl").read_bytes()), "totals": totals, "children": out}


def whitespace_clean(data):
    """True for UTF-8 text that `git diff --check` accepts: no trailing blanks,
    space before tab, carriage return or blank line at the end. Anything else
    (raw RGBA included, which may lack NUL bytes) is not text."""
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return False
    if "\0" in text:
        return False
    lines = text.split("\n")
    trailing = any(line.rstrip(" \t") != line or " \t" in line for line in lines)
    return not trailing and "\r" not in text and not text.endswith("\n\n")


def command_retain(evidence, target, *relative):
    """Exact copies of named evidence files. Binary files and clean text are
    copied as is; text that whitespace checks would reject is kept byte-exact
    inside a deterministic gzip. Every copy is re-read and hash-verified."""
    evidence, target = Path(evidence), Path(target)
    out = {}
    for rel in relative:
        source = evidence / rel
        data = source.read_bytes()
        is_png = data[:8] == b"\x89PNG\r\n\x1a\n"
        keep_raw = is_png or whitespace_clean(data)
        dest = target / (rel if keep_raw else rel + ".gz")
        if dest.exists():
            raise FileExistsError(f"{dest} exists; retained copies are never replaced")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(data if keep_raw else gzip.compress(data, compresslevel=9, mtime=0))
        back = dest.read_bytes()
        restored = back if keep_raw else gzip.decompress(back)
        if sha256(restored) != sha256(data) or sha256(source.read_bytes()) != sha256(data):
            raise ValueError(f"{rel}: retained copy does not reproduce the original bytes")
        if keep_raw:
            why = {}
        else:
            try:
                data.decode("utf-8")
                reason = "text that git diff --check rejects"
            except UnicodeDecodeError:
                reason = "binary without NUL bytes, which git text=auto would treat as text"
            why = {"gzip_sha256": sha256(back), "why": reason}
        out[rel] = {"original": str(source), "sha256": sha256(data), "bytes": len(data),
                    "retained_as": str(dest.relative_to(target)), **why}
    return out


def main(argv):
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    command, rest = argv[0], argv[1:]
    if command == "execution" and len(rest) == 1:
        result = command_execution(rest[0])
    elif command == "retain" and len(rest) >= 3:
        result = command_retain(*rest)
    elif command == "verify" and len(rest) == 1:
        result = command_verify(rest[0])
    elif command == "builds" and len(rest) == 2:
        result = command_builds(*rest)
    elif command == "halo" and len(rest) == 1:
        result = command_halo(rest[0])
    elif command == "spread" and len(rest) == 1:
        result = command_spread(rest[0])
    elif command == "texture" and len(rest) == 1:
        result = command_texture(rest[0])
    elif command == "figures" and len(rest) in (2, 3):
        result = command_figures(*rest)
    else:
        print(__doc__, file=sys.stderr)
        return 2
    print(json.dumps(result, indent=1, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
