#!/usr/bin/env python3
"""0073 implementation-5 contract model. Arithmetic and provenance only, standard
library, deterministic. No image simulation.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/consumer_v5.py \
        --out handoffs/0073-upscaling-review/repair-model/results-5.json

Part A replicates producer 613ee59a motion/attachment.rs admit() (exact reuse of
equal extents, per-component old+new sub-caps, unchanged 671,105,024 B ceiling),
checks it against the producer's own unit-test values, then adds the proposed
owner-history meta pair (Rg32Uint x 2 = 16 B per output pixel).

Part B is the executable reference for the consumer's per-pixel provenance
decision and Rg32Uint record packing in the existing 2x2 portable resolve.
"""
import argparse
import json
import sys

sys.dont_write_bytecode = True

P = 4 * 1024 * 1024
INSTANCE = 64 * 1024 * 1024
FIXED = 16384
CAP = 128 * P + 2 * INSTANCE + FIXED                  # MAX_UPSCALE_TRANSITION_BYTES
CAPS = {"motion": 24 * P, "owner": 4 * P, "output": 32 * P, "overlay": 8 * P, "meta": 16 * P}
STRIDE = {"motion": 24, "owner": 4, "output": 32, "overlay": 8, "meta": 16}


class Refused(Exception):
    pass


def pixels(size, limit=8192):
    if 0 in size or any(d > min(limit, 8192) for d in size) or size[0] * size[1] > P:
        raise Refused("extent")
    return size[0] * size[1]


def component(old, new, name):
    """producer component(): old+candidate sub-cap; equal extents count once."""
    candidate = pixels(new) * STRIDE[name]
    old_bytes = pixels(old) * STRIDE[name] if old else 0
    if old_bytes > CAPS[name] or candidate > CAPS[name] or old_bytes + candidate > 2 * CAPS[name]:
        raise Refused(name)
    return old_bytes + (0 if old == new else candidate)


def admit(source, old_motion, output=None, old_output=None, old_overlay=None, meta=False):
    total = 2 * INSTANCE + FIXED
    total += component(old_motion, source, "motion") + component(old_motion, source, "owner")
    if output is not None:
        total += component(old_output, output, "output") + component(old_overlay, source, "overlay")
        if meta:  # proposed: the meta pair shares Output's extent and lifetime
            total += component(old_output, output, "meta")
    elif old_output is not None or old_overlay is not None:
        raise Refused("output")
    if total > CAP:
        raise Refused("ceiling")
    return total


def attempt(*args, **kw):
    try:
        return admit(*args, **kw)
    except Refused as error:
        return "refused:" + str(error)


# ------------------------------------------------------------------ Part B
NONE, OVERFLOW = 0, 1 << 31
PRESENT = 1 << 31
ID_BITS = (1 << 17) - 1


def classes(ids):
    mask = 0
    for i in ids:
        mask |= 1 << (i % 31)
    return mask


def encode(anchor, owners, mask=0):
    """x: bit31 present, bits0-16 anchor owner (0 = clear); y: second id, or
    bit31 set with the 31-class mask of every recorded owner."""
    full = (set(owners) | {anchor}) - {0}
    if mask or len(full) > 2:
        return [PRESENT | anchor, OVERFLOW | mask | classes(full)]
    other = sorted(full - {anchor})
    return [PRESENT | anchor, other[0] if other else NONE]


def decode(record, highest):
    """Canonical read; anything else is no history (None)."""
    x, y = record
    if not x & PRESENT or x & ~(PRESENT | ID_BITS) or (x & ID_BITS) > highest:
        return None
    anchor = x & ID_BITS
    if y & OVERFLOW:
        mask = y & ~OVERFLOW
        return (anchor, None, mask) if mask else None
    if y > highest or (y and y == anchor) or (y and not anchor):
        return None
    owners = {i for i in (anchor, y) if i}
    return anchor, owners, 0


def decide(record, support, anchor, changed, flags, static, highest=100000):
    """support: ids of 2x2 taps with positive weight and depth > 0 (current frame);
    changed: set of this frame's changed ids; flags: camera_static, global."""
    S = set(support)
    C = classes(changed)
    stored = decode(record, highest) if record else None
    fresh = encode(anchor, S)
    if stored is None:
        return "reproject_no_record", fresh
    _, owners, mask = stored
    if owners is not None:
        moved = {o for o in owners if o in changed}
        if any(o not in S for o in moved):
            return "drop", fresh
        union = owners | S
        if static and flags["camera_static"] and not flags["global"] and not moved and not (S & changed):
            return "persist", encode(anchor, union)
        return "reproject", encode(anchor, union)
    if (mask | classes(S)) & C:
        return "drop", fresh
    if static and flags["camera_static"] and not flags["global"]:
        return "persist", encode(anchor, S, mask | classes(S))
    return "reproject", encode(anchor, S, mask | classes(S))


def provenance_cases():
    calm = {"camera_static": True, "global": False}
    rows = []

    def case(name, expect_branch, record, support, anchor, changed, flags=calm, static=True):
        branch, new = decide(record, support, anchor, set(changed), flags, static)
        rows.append({"case": name, "branch": branch, "expected": expect_branch,
                     "ok": branch == expect_branch, "record_in": record, "record_out": new})

    case("unchanged static pixel persists", "persist", encode(5, {5, 9}), [5, 9], 5, [])
    case("thin owner missing this phase still persists", "persist", encode(5, {5, 9}), [5], 5, [])
    case("changed recorded owner absent from support drops", "drop", encode(5, {5, 7}), [5], 5, [7])
    case("changed recorded owner still present reprojects", "reproject", encode(5, {5, 7}), [5, 7], 5, [7])
    case("changed newcomer in support blocks persistence", "reproject", encode(5, {5}), [5, 3], 5, [3])
    case("camera change blocks persistence", "reproject", encode(5, {5}), [5], 5, [],
         {"camera_static": False, "global": False})
    case("light or caster change blocks persistence", "reproject", encode(5, {5}), [5], 5, [],
         {"camera_static": True, "global": True})
    case("moving pixel reprojects", "reproject", encode(5, {5}), [5], 5, [], static=False)
    case("overflow without class hit persists", "persist", encode(5, {5, 9, 11}), [5, 9], 5, [43])
    case("overflow collision with a non-contributor drops (cost)", "drop", encode(5, {5, 9, 11}), [5, 9], 5, [40])
    case("overflow with class hit drops", "drop", encode(5, {5, 9, 11}), [5, 9], 5, [42])
    case("unrelated changed owner does not disturb exact record", "persist", encode(5, {5, 9}), [5, 9], 5, [42])
    case("no record reprojects without history", "reproject_no_record", None, [5], 5, [])
    case("malformed record is no history", "reproject_no_record", [PRESENT | 5 | (1 << 20), 0], [5], 5, [])
    case("id above highest is no history", "reproject_no_record", [PRESENT | 5, 100001], [5], 5, [])
    return rows


def report():
    full, wide, reduced = (2048, 2048), (2048, 1920), (1920, 1920)
    checks = []

    def check(name, ok, **values):
        checks.append({"check": name, "ok": bool(ok), **values})

    # Producer unit-test vectors (attachment.rs), reproduced exactly.
    steady = attempt(full, full, full, full, full)
    check("producer steady owner+upscale total", steady == 68 * P + 2 * INSTANCE + FIXED, total=steady)
    check("producer 2048^2 -> 2048x1920 resize refused",
          str(attempt(wide, full, wide, full, full)).startswith("refused"),
          result=attempt(wide, full, wide, full, full))
    check("producer 1920^2 internal change admitted", isinstance(attempt(reduced, full, full, full, full), int),
          total=attempt(reduced, full, full, full, full))
    check("cap unchanged", CAP == 671105024, cap=CAP)
    # Proposed meta pair.
    rows = {}
    for label, args in {
        "steady 2048^2 16/16": (full, full, full, full, full),
        "DRS 16->15 at 2048^2": ((1920, 1920), full, full, full, full),
        "DRS 15->16 at 2048^2": (full, (1920, 1920), full, full, (1920, 1920)),
        "resize 2048^2 -> 2048x1920 16/16": (wide, full, wide, full, full),
        "resize 1920x1080 -> 2560x1440 16/16": ((2560, 1440), (1920, 1080), (2560, 1440), (1920, 1080), (1920, 1080)),
        "resize 2560x1440 -> 2560x1600 8/16": ((1280, 800), (1280, 720), (2560, 1600), (2560, 1440), (1280, 720)),
        "steady 2560x1600 8/16": ((1280, 800), (1280, 800), (2560, 1600), (2560, 1600), (1280, 800)),
    }.items():
        rows[label] = {"without_meta": attempt(*args), "with_meta": attempt(*args, meta=True)}
    check("steady maximum with meta fits", isinstance(rows["steady 2048^2 16/16"]["with_meta"], int))
    check("DRS at maximum output with meta fits", isinstance(rows["DRS 16->15 at 2048^2"]["with_meta"], int)
          and isinstance(rows["DRS 15->16 at 2048^2"]["with_meta"], int))
    budget = CAP - 2 * INSTANCE - FIXED
    thresholds = {f"{n}/16": budget // (32 + 16 + 36 * (n / 16) ** 2) for n in (16, 12, 8, 4)}
    cases = provenance_cases()
    check("provenance reference cases", all(c["ok"] for c in cases), cases=len(cases))
    return {"cap": CAP, "strides": STRIDE, "sub_caps": CAPS, "checks": checks, "admission": rows,
            "resize_sum_threshold_px_approx": thresholds, "provenance": cases}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    res = report()
    text = json.dumps({"model": "0073 implementation-5 consumer contract", "python": sys.version.split()[0],
                       **res}, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 0 if all(c["ok"] for c in res["checks"]) else 1


if __name__ == "__main__":
    sys.exit(main())
