#!/usr/bin/env python3
"""0073 resolve_owner.wgsl transcription check. Standard library, deterministic, no
GPU, no image, no compiler.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/shader_v5_check.py \
        --out handoffs/0073-upscaling-review/repair-model/results-shader-v5.json

shader() below transcribes the committed shader statement by statement in scalar
f64 (the shader file is sha-pinned; edit both together). It is compared with the
d90b4aac corrected reference resolve() in correction_v5.py (sha-pinned):
  1. every corrected-mode scenario of the reference controls, replayed in place;
  2. a seeded fuzz of 20000 pixels (texture changed bits consistent with the
     changed table, which the reference models as one set);
plus shader-only conservative paths with explicit expectations, and the
changed-table packing against set membership for every ID.
"""
import argparse
import hashlib
import json
import math
import os
import random
import runpy
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
SHADER_PATH = os.path.join(ROOT, "crates", "incant_render", "src", "upscaling", "resolve_owner.wgsl")
SHADER_SHA = "45a39859021f3ad7968d2a134b9c0aca7e222c3fa0c0100c1f22715c0bc6a8c0"
MODEL_PATH = os.path.join(HERE, "correction_v5.py")
MODEL_SHA = "96464a75ff1ec84b32e400c953e5ab5b5b004f98a4d592a800a3ad5fc93c534f"


def pinned(path, sha):
    with open(path, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
    if digest != sha:
        raise SystemExit("pinned file changed: {} {}".format(path, digest))


pinned(SHADER_PATH, SHADER_SHA)
pinned(MODEL_PATH, MODEL_SHA)
G = runpy.run_path(MODEL_PATH, run_name="correction_v5_reference")["resolve"].__globals__
REFERENCE = G["resolve"]
clean, admitted, compatible, tap_weight, mix = (G[k] for k in ("clean", "admitted", "compatible", "tap_weight", "mix"))
FAR, OPT, read, classes = G["FAR"], G["OPT"], G["read"], G["classes"]
HIGH_BIT, ID_BITS, MAX_OWNER, HIGHEST = 0x80000000, 0x1FFFF, 100000, 100000


def pack(changed):
    """Host layout: u32 word k holds IDs 32k..32k+31; vec4 k>>2, component k&3."""
    words = [[0, 0, 0, 0] for _ in range(782)]
    for n in changed:
        k = n >> 5
        words[k >> 2][k & 3] |= 1 << (n & 31)
    return words


# ----------------------------------------------- transcription of the shader
def shader(sc, header_z=HIGHEST, header_c=None):
    changed = set(sc["changed"])
    words = pack(n for n in changed if 1 <= n <= MAX_OWNER)
    header_x = classes(changed) if header_c is None else header_c
    flags = (1 if sc["flags"]["camera_static"] else 0) | (2 if sc["flags"]["global"] else 0)

    def valid_id(n):
        return n >= 1 and n <= min(header_z, MAX_OWNER)

    def changed_id(n):
        return ((words[n >> 7][(n >> 5) & 3] >> (n & 31)) & 1) != 0

    def class_of(n):
        return 1 << (n % 31)

    def owner_set(a):
        return {"a": a, "b": 0, "m": class_of(a), "over": False, "ok": valid_id(a)}

    def with_id(s, n):
        o = dict(s)
        if not valid_id(n):
            o["ok"] = False
            return o
        o["m"] |= class_of(n)
        if n != o["a"] and n != o["b"]:
            if o["b"] == 0:
                o["b"] = n
            else:
                o["over"] = True
        return o

    def encode(s):
        if not s["ok"]:
            return [0, 0]
        return [HIGH_BIT | s["a"], (HIGH_BIT | s["m"]) if s["over"] else s["b"]]

    def stored(v):
        s = {"a": v[0] & ID_BITS, "b": 0, "m": 0, "over": False, "ok": False}
        if (v[0] & HIGH_BIT) == 0 or (v[0] & ~(HIGH_BIT | ID_BITS) & 0xFFFFFFFF) != 0 or not valid_id(s["a"]):
            return s
        if (v[1] & HIGH_BIT) != 0:
            s["m"] = v[1] & ~HIGH_BIT & 0xFFFFFFFF
            s["over"] = True
            s["ok"] = (s["m"] & class_of(s["a"])) != 0
        else:
            s["b"] = v[1]
            s["ok"] = v[1] == 0 or (v[1] != s["a"] and valid_id(v[1]))
        return s

    def with_record(s, r):
        o = with_id(s, r["a"])
        if r["over"]:
            o["m"] |= r["m"]
            o["over"] = True
        elif r["b"] != 0:
            o = with_id(o, r["b"])
        return o

    def record_unchanged(r, support_classes, changed_classes):
        if r["over"]:
            return ((r["m"] | support_classes) & changed_classes) == 0
        return not changed_id(r["a"]) and (r["b"] == 0 or not changed_id(r["b"]))

    def record_admissible(r, support, support_classes, changed_classes):
        if r["over"]:
            return ((r["m"] | support_classes) & changed_classes) == 0
        gone_a = changed_id(r["a"]) and r["a"] not in support
        gone_b = r["b"] != 0 and changed_id(r["b"]) and r["b"] not in support
        return not gone_a and not gone_b

    w_, h_ = sc["size"]

    def clampt(t):
        return (min(max(t[0], 0), w_ - 1), min(max(t[1], 0), h_ - 1))

    out = {"color": 0.0, "depth": 0.0, "record": [0, 0], "path": "outside"}
    if not sc.get("inside", True):
        return out
    q = sc["q"]
    base = (math.floor(q[0]), math.floor(q[1]))
    f = (q[0] - base[0], q[1] - base[1])
    anchor, nearest, same_surface, col = clampt(base), FAR + 1.0, True, 0.0
    support, support_classes, support_ok, support_changed = [0, 0, 0, 0], 0, True, False
    changed_classes = header_x
    for y in (0, 1):
        for x in (0, 1):
            weight = tap_weight(f, x, y)
            if weight <= 0.0:
                continue
            tap = clampt((base[0] + x, base[1] + y))
            c, a, z, word = sc["src"][tap]
            col += clean(c, a) * weight
            if not admitted(c, a) or a < 1.0 or z <= 0.0:
                same_surface = False
            if z > 0.0 and z < nearest:
                nearest, anchor = z, tap
            if z > 0.0:
                n = word & ~HIGH_BIT & 0xFFFFFFFF
                support[y * 2 + x] = n
                if not valid_id(n):
                    support_ok = False
                else:
                    support_classes |= class_of(n)
                    if (word & HIGH_BIT) != 0 or changed_id(n):
                        support_changed = True
                        changed_classes |= class_of(n)
    out["color"] = col
    if nearest > FAR:
        out["path"] = "all_clear"
        return out
    for y in (0, 1):
        for x in (0, 1):
            if tap_weight(f, x, y) <= 0.0:
                continue
            same_surface = same_surface and compatible(sc["src"][clampt((base[0] + x, base[1] + y))][2], nearest)
    if not same_surface:
        out["path"] = "not_one_surface"
        return out
    current = clean(col, 1.0)
    out.update(color=current, depth=nearest)
    anchor_id = sc["src"][anchor][3] & ~HIGH_BIT & 0xFFFFFFFF
    fresh = owner_set(anchor_id)
    for i in range(4):
        if support[i] != 0:
            fresh = with_id(fresh, support[i])
    fresh["ok"] = fresh["ok"] and support_ok
    provable = fresh["ok"]
    out["record"] = encode(fresh)
    if not sc.get("history", True):
        out["path"] = "history_disabled"
        return out
    if not sc.get("motion", True):
        out["path"] = "invalid_motion"
        return out
    if sc.get("old") is None:
        out["path"] = "old_outside"
        return out
    expected = sc.get("expected", nearest)
    still = (flags & 1) != 0 and sc.get("still", False)
    if still:
        oldbase, oldf = tuple(sc["p"]), (0.0, 0.0)
    else:
        o = sc["old"]
        oldbase = (math.floor(o[0]), math.floor(o[1]))
        oldf = (o[0] - oldbase[0], o[1] - oldbase[1])
    prev = sc["prev"]
    if still and (flags & 2) == 0 and provable and not support_changed and oldbase in prev:
        c, a, z, rec = prev[oldbase]
        r = stored(rec)
        if admitted(c, a) and a >= 1.0 and compatible(z, expected) and r["ok"] \
                and record_unchanged(r, support_classes, changed_classes):
            out.update(color=clean(mix(current, clean(c, a), OPT["blend"]), 1.0),
                       record=encode(with_record(fresh, r)), path="persist")
            return out
    history, mass, joined = 0.0, 0.0, fresh
    for y in (0, 1):
        for x in (0, 1):
            weight = tap_weight(oldf, x, y)
            if not weight > 0.0:
                continue
            tap = (oldbase[0] + x, oldbase[1] + y)
            if tap not in prev:
                continue
            c, a, z, rec = prev[tap]
            if not admitted(c, a) or a < 1.0 or not compatible(z, expected):
                continue
            r = stored(rec)
            if not provable or not r["ok"] or r["a"] != anchor_id \
                    or not record_admissible(r, support, support_classes, changed_classes):
                continue
            history += clean(c, a) * weight
            mass += weight
            joined = with_record(joined, r)
    if not mass > 0.0:
        out["path"] = "current_only"
        return out
    history = clean(history / mass, 1.0)
    minimum = maximum = current
    for y in (-1, 0, 1):
        for x in (-1, 0, 1):
            c, a, z, word = sc["src"][clampt((anchor[0] + x, anchor[1] + y))]
            if admitted(c, a) and a >= 1.0 and compatible(z, nearest):
                minimum, maximum = min(minimum, clean(c, a)), max(maximum, clean(c, a))
                joined = with_id(joined, word & ~HIGH_BIT & 0xFFFFFFFF)
    reaction = min(max(abs(history - current) / OPT["scale"], 0.0), 1.0)
    weight = OPT["blend"] * (1.0 - OPT["reactive"] * reaction)
    out.update(color=clean(mix(current, min(max(history, minimum), maximum), weight), 1.0),
               record=encode(joined), path="reproject")
    return out


def same(ref, mine):
    return abs(ref["color"] - mine["color"]) <= 1e-12 and ref["depth"] == mine["depth"] \
        and ref["record"] == mine["record"]


def canonical(rec):
    return rec == [0, 0] or read(rec) is not None


# ------------------------------------------------ 1. reference scenarios replayed
def replay():
    seen = []

    def wrapped(sc, mode, highest=HIGHEST):
        ref = REFERENCE(sc, mode, highest)
        if mode == "corrected":
            mine = shader(sc)
            seen.append({"reference_path": ref["path"], "shader_path": mine["path"], "match": same(ref, mine),
                         "record": mine["record"], "canonical": canonical(mine["record"])})
        return ref

    G["resolve"] = wrapped
    try:
        oks = {name: bool(G[name]()[0]) for name in ("clamp_chain", "every_record", "canonical_and_malformed")}
    finally:
        G["resolve"] = REFERENCE
    ok = all(oks.values()) and len(seen) > 0 and all(s["match"] and s["canonical"] for s in seen)
    return ok, {"reference_controls_ok": oks, "pixels": len(seen), "results": seen}


# ------------------------------------------------------------------- 2. fuzz
POOL = [5, 7, 9, 11, 36, 40, 0, HIGHEST + 1, 5 | (1 << 20)]
POOL_WEIGHTS = [8, 5, 3, 2, 2, 1, 1, 1, 1]
VALID = [5, 7, 9, 11, 36, 40]


def fuzz_scenario(rng):
    changed = set(rng.sample(VALID, rng.randrange(0, 3)))

    def word():
        n = rng.choices(POOL, POOL_WEIGHTS)[0]
        bit = (n in changed) if 1 <= n <= HIGHEST else rng.random() < 0.5
        return n | (HIGH_BIT if bit else 0)

    def texel():
        return (rng.choice([0.0, 0.25, 0.5, 0.75, 1.0, 1.5]), rng.choices([1.0, 0.5, 0.0], [40, 1, 1])[0],
                rng.choices([10.0, 10.004, 12.0, 0.0], [40, 6, 1, 1])[0], word())

    def record():
        kind = rng.randrange(12)
        if kind < 8:
            a = rng.choice(VALID[:5])
            owners = set(rng.sample(VALID, rng.randrange(0, 4)))
            mask = classes(rng.sample(VALID, 2)) if rng.random() < 0.1 else 0
            return G["record"](a, owners, mask)
        return [[0, 0], [HIGH_BIT, 7], [HIGH_BIT | 5, HIGH_BIT | classes({9, 11})],
                [HIGH_BIT | 5 | (1 << 20), 0]][kind - 8]

    def previous():
        return (rng.choice([0.0, 0.3, 0.6, 0.9, 1.2]), rng.choices([1.0, 0.5], [12, 1])[0],
                rng.choices([10.0, 10.004, 0.0, 12.0], [10, 3, 1, 1])[0], record())

    p = (rng.randrange(1, 3), rng.randrange(1, 3))
    moving = rng.random() < 0.5
    old = (p[0] + rng.choice([-0.5, -0.25, 0.0, 0.25, 0.5]), p[1] + rng.choice([-0.5, -0.25, 0.0, 0.25, 0.5])) \
        if moving else (float(p[0]), float(p[1]))
    return {"size": (4, 4), "src": {(x, y): texel() for y in range(4) for x in range(4)},
            "prev": {(x, y): previous() for y in range(4) for x in range(4)},
            "q": (rng.choice([0.5, 0.75, 1.0, 1.25, 1.5, 2.0]), rng.choice([0.5, 0.75, 1.0, 1.25, 1.5, 2.0])),
            "p": p, "old": None if rng.random() < 0.03 else old, "still": not moving,
            "flags": {"camera_static": rng.random() < 0.7, "global": rng.random() < 0.2},
            "changed": sorted(changed), "history": rng.random() < 0.97, "motion": rng.random() < 0.97,
            "inside": rng.random() >= 0.01}


def fuzz(count=20000, seed=73):
    rng = random.Random(seed)
    paths, mismatches, noncanonical = {}, [], 0
    for i in range(count):
        sc = fuzz_scenario(rng)
        ref, mine = REFERENCE(sc, "corrected"), shader(sc)
        paths[mine["path"]] = paths.get(mine["path"], 0) + 1
        # The reference names its static drop "drop"; the shader reaches the same
        # current-only output by rejecting the single static tap.
        if not same(ref, mine) or {"drop": "current_only"}.get(ref["path"], ref["path"]) != mine["path"]:
            if len(mismatches) < 5:
                mismatches.append({"index": i, "reference": [ref["path"], ref["color"], ref["record"]],
                                   "shader": [mine["path"], mine["color"], mine["record"]]})
        if not canonical(mine["record"]):
            noncanonical += 1
    ok = not mismatches and noncanonical == 0 and all(paths.get(k, 0) > 0 for k in (
        "outside", "all_clear", "not_one_surface", "history_disabled", "invalid_motion", "old_outside",
        "current_only", "persist", "reproject"))
    return ok, {"pixels": count, "seed": seed, "paths": dict(sorted(paths.items())),
                "mismatches": mismatches, "noncanonical_records": noncanonical}


# --------------------------------------------------- 3. shader-only conservatism
def conservative():
    rows, ok = {}, True
    base = {(x, y): (0.25, 1.0, 10.0, 5) for y in range(4) for x in range(4)}
    base[(0, 0)] = (0.0, 1.0, 10.0, 5)
    base[(2, 2)] = (1.0, 1.0, 10.0, 5)

    def sc(**k):
        d = {"size": (4, 4), "src": base, "q": (1.0, 1.0), "p": (1, 1), "old": (1.0, 1.0), "still": True,
             "flags": {"camera_static": True, "global": False}, "changed": [],
             "prev": {(1, 1): (0.9, 1.0, 10.0, G["record"](5, {5}))}}
        d.update(k)
        return d

    calm = shader(sc())
    rows["unchanged static pixel persists"] = calm
    ok = ok and calm["path"] == "persist" and abs(calm["color"] - mix(0.25, 0.9, 0.9)) <= 1e-12
    texture_only = dict(base)
    texture_only[(2, 2)] = (0.25, 1.0, 10.0, 5)                  # clamp max 0.25 now binds
    still_calm = shader(sc(src=dict(texture_only)))
    texture_only[(1, 1)] = (0.25, 1.0, 10.0, 5 | HIGH_BIT)       # changed bit, table clear
    t = shader(sc(src=texture_only))
    rows["same pixel without the bit persists unclamped"] = still_calm
    rows["texture changed bit alone blocks persistence"] = t
    ok = ok and still_calm["path"] == "persist" and abs(still_calm["color"] - mix(0.25, 0.9, 0.9)) <= 1e-12 \
        and t["path"] == "reproject" and t["record"] == G["record"](5, {5}) and abs(t["color"] - 0.25) <= 1e-12
    beyond = dict(base)
    beyond[(1, 1)] = (0.25, 1.0, 10.0, HIGHEST + 1)
    hb = shader(sc(src=beyond, history=False), header_z=100500)
    rows["header highest above MAX_OWNER still bounds IDs"] = hb
    ok = ok and hb["record"] == [0, 0]
    other = sc(prev={(1, 1): (0.9, 1.0, 10.0, G["record"](9, {9}))})
    persist = shader(other)
    rows["persistence without anchor match (complete proof)"] = persist
    ok = ok and persist["path"] == "persist" and persist["record"] == G["record"](5, {5, 9})
    other["flags"] = {"camera_static": True, "global": True}
    gated = shader(other)
    rows["global change: ordinary tap needs anchor match"] = gated
    ok = ok and gated["path"] == "current_only" and gated["record"] == G["record"](5, {5})
    return ok, rows


def table_layout():
    rng = random.Random(7)
    changed = set(rng.sample(range(1, MAX_OWNER + 1), 300)) | {1, 31, 32, 127, 128, MAX_OWNER}
    words = pack(changed)
    lookup = all((((words[n >> 7][(n >> 5) & 3] >> (n & 31)) & 1) != 0) == (n in changed)
                 for n in range(1, MAX_OWNER + 1))
    return lookup and (MAX_OWNER >> 7) == 781, {"ids_checked": MAX_OWNER, "changed": len(changed),
                                                "max_vec4_index": MAX_OWNER >> 7}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    checks, details = [], {}
    for name, fn in (("reference scenarios replayed", replay), ("seeded fuzz against reference", fuzz),
                     ("shader-only conservative paths", conservative), ("changed-table layout", table_layout)):
        ok, detail = fn()
        checks.append({"check": name, "ok": bool(ok)})
        details[name] = detail
    text = json.dumps({"model": "0073 resolve_owner.wgsl transcription check", "python": sys.version.split()[0],
                       "shader_sha256": SHADER_SHA, "reference_model_sha256": MODEL_SHA,
                       "checks": checks, "details": details}, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 0 if all(c["ok"] for c in checks) else 1


if __name__ == "__main__":
    sys.exit(main())
