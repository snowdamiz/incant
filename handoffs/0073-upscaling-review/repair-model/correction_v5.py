#!/usr/bin/env python3
"""0073 implementation-5 correction controls. Standard library, deterministic, no
image simulation, no GPU.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/correction_v5.py \
        --out handoffs/0073-upscaling-review/repair-model/results-5-correction.json

The committed draft model consumer_v5.py (sha-pinned, unchanged) is loaded by path
as the baseline. Each control runs the draft rule and the corrected rule on the
same input:
  1. 3x3 clamp provenance (Astra's clamp_injected_contributor numbers);
  2. per-tap rejection of every bilinear history record, then renormalisation
     (Astra's moving_changed_absent_case record inside a four-record footprint);
  3. camera_static bits: unjittered matrix, camera.view, shading_eye() and the
     internal rect (jitter guard, cluster and cascade inputs);
  4. canonical anchors, positive-weight support, clear and malformed owners on
     every early exit (Astra's canonical_clear_case);
  5. actual future GPU submissions;
  6. flag-keyed admission reuse, charged upload storage, and the reset that
     really releases retained Arcs before a resize retry.
"""
import argparse
import hashlib
import json
import math
import os
import runpy
import struct
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
DRAFT_PATH = os.path.join(HERE, "consumer_v5.py")
DRAFT_SHA = "196cad707f13c50b1a5445deec016f95d02bd2174deeb116efab61c4d13f9cd7"


def load_draft():
    with open(DRAFT_PATH, "rb") as f:
        digest = hashlib.sha256(f.read()).hexdigest()
    if digest != DRAFT_SHA:
        raise SystemExit("draft model changed: " + digest)
    return runpy.run_path(DRAFT_PATH, run_name="consumer_v5_draft")


D = load_draft()
HIGHEST = 100000
PRESENT = OVERFLOW = 1 << 31
ID = (1 << 17) - 1
ABSENT = [0, 0]


# ------------------------------------------------------------- corrected record
def valid(i, highest=HIGHEST):
    return 1 <= i <= highest


def classes(ids):
    mask = 0
    for i in ids:
        mask |= 1 << (i % 31)
    return mask


def owner_id(word):
    """Owner word id | changed << 31. The id is the low 31 bits, so bits 17-30
    can never pass the highest-id check."""
    return word & 0x7FFFFFFF


def record(anchor, owners, mask=0, highest=HIGHEST):
    """Corrected writer. Every owner the stored colour depends on, anchor included,
    must be a valid id; otherwise the pixel writes (0,0), which is no history."""
    full = set(owners) | {anchor}
    if not all(valid(i, highest) for i in full):
        return list(ABSENT)
    if mask or len(full) > 2:
        return [PRESENT | anchor, OVERFLOW | mask | classes(full)]
    other = sorted(full - {anchor})
    return [PRESENT | anchor, other[0] if other else 0]


def read(rec, highest=HIGHEST):
    """Corrected canonical reader; None is no history."""
    x, y = rec
    if not x & PRESENT or x & ~(PRESENT | ID):
        return None
    a = x & ID
    if not valid(a, highest):
        return None
    if y & OVERFLOW:
        mask = y & ~OVERFLOW
        return (a, None, mask) if mask & (1 << (a % 31)) else None
    if y and (not valid(y, highest) or y == a):
        return None
    return a, ({a, y} if y else {a}), 0


def decide_c(rec, S, A, changed, flags, still, K=None, highest=HIGHEST):
    """Corrected single-tap decision (the static tap at P, or one bilinear tap).
    K is the admitted 3x3 clamp support (K contains S whenever the clamp runs)."""
    S = set(S)
    K = set(S) if K is None else set(K)
    if not valid(A, highest) or not all(valid(i, highest) for i in S | K):
        return "unprovable", list(ABSENT)
    C = classes(changed)
    fresh = record(A, S, 0, highest)
    R = read(rec, highest) if rec else None
    if R is None:
        return "no_history", fresh
    a, owners, mask = R
    if owners is not None:
        if any(o in changed and o not in S for o in owners):
            return "drop", fresh
        if still and flags["camera_static"] and not flags["global"] and not ((owners | S) & changed):
            return "persist", record(A, owners | S, 0, highest)
        if a != A:
            return "anchor_mismatch", fresh
        return "reproject", record(A, owners | K, 0, highest)
    if (mask | classes(S)) & C:
        return "drop", fresh
    if still and flags["camera_static"] and not flags["global"]:
        return "persist", record(A, S, mask, highest)
    if a != A:
        return "anchor_mismatch", fresh
    return "reproject", record(A, K, mask, highest)


# ------------------------------------------------- per-pixel resolve (scalar f64)
FAR = 100.0
OPT = {"blend": 0.9, "rel": 0.001, "abs": 0.001, "scale": 1.0, "reactive": 0.0}


def finite(*v):
    return all(math.isfinite(t) and abs(t) <= 3.402823e38 for t in v)


def admitted(c, a):
    return finite(c, a) and c >= 0 and a >= 0 and a <= 1


def clean(c, a):
    if not finite(c, a) or a <= 0:
        return 0.0
    return min(max(c, 0.0), 65504.0)


def compatible(a, b):
    return a > 0 and b > 0 and a <= FAR and b <= FAR and abs(a - b) <= max(OPT["abs"], OPT["rel"] * b)


def tap_weight(f, x, y):
    return (f[0] if x else 1 - f[0]) * (f[1] if y else 1 - f[1])


def mix(a, b, t):
    return a * (1 - t) + b * t                         # WGSL mix


def resolve(sc, mode, highest=HIGHEST):
    """Scalar transcription of resolve.wgsl plus the owner rules. mode 'draft' is
    implementation-5 as written (encode/decode/decide from consumer_v5.py, anchor-only
    tap gate, S-only union); 'corrected' is this correction."""
    corr = mode == "corrected"
    w_, h_ = sc["size"]

    def clampt(t):
        return (min(max(t[0], 0), w_ - 1), min(max(t[1], 0), h_ - 1))

    out = {"color": 0.0, "depth": 0.0, "record": list(ABSENT), "path": "outside", "taps": []}
    if not sc.get("inside", True):
        return out
    q = sc["q"]
    base = (math.floor(q[0]), math.floor(q[1]))
    f = (q[0] - base[0], q[1] - base[1])
    anchor, nearest, same, col, words = clampt(base), FAR + 1.0, True, 0.0, []
    positive = []
    for y in (0, 1):
        for x in (0, 1):
            wt = tap_weight(f, x, y)
            if wt <= 0:
                continue
            t = clampt((base[0] + x, base[1] + y))
            c, a, z, word = sc["src"][t]
            col += clean(c, a) * wt
            positive.append(z)
            if not admitted(c, a) or a < 1 or z <= 0:
                same = False
            if 0 < z < nearest:
                nearest, anchor = z, t
            if z > 0:
                words.append(word)
    out["color"] = col
    anchor_word = sc["src"][anchor][3]
    S_d, A_d = {w & ID for w in words}, anchor_word & ID
    if nearest > FAR:
        out["path"] = "all_clear"
        out["record"] = list(ABSENT) if corr else D["encode"](A_d, S_d)
        return out
    for z in positive:
        same = same and compatible(z, nearest)
    if not same:
        out["path"] = "not_one_surface"
        out["record"] = list(ABSENT) if corr else D["encode"](A_d, S_d)
        return out
    current = clean(col, 1.0)
    A = owner_id(anchor_word)
    S = {owner_id(w) for w in words}
    provable = valid(A, highest) and all(valid(i, highest) for i in S)
    fresh = (record(A, S, 0, highest) if provable else list(ABSENT)) if corr else D["encode"](A_d, S_d)
    out.update(color=current, depth=nearest, record=fresh)
    for gate, path in (("history", "history_disabled"), ("motion", "invalid_motion")):
        if not sc.get(gate, True):
            out["path"] = path
            return out
    if sc.get("old") is None:
        out["path"] = "old_outside"
        return out
    flags, changed, expected = sc["flags"], set(sc["changed"]), sc.get("expected", nearest)
    still = flags["camera_static"] and sc.get("still", False)

    def passes(entry):
        return entry is not None and admitted(entry[0], entry[1]) and entry[1] >= 1 and compatible(entry[2], expected)

    if still:
        taps = [(sc["p"], 1.0)]
        entry = sc["prev"].get(sc["p"])
        if passes(entry):
            if corr:
                branch, rec = decide_c(entry[3], S, A, changed, flags, True, highest=highest) \
                    if provable else ("unprovable", list(ABSENT))
            else:
                branch, rec = D["decide"](entry[3], S_d, A_d, changed, flags, True, highest)
            if branch == "drop":
                out.update(record=rec, path="drop")
                return out
            if branch == "persist":
                out.update(color=clean(mix(current, clean(entry[0], entry[1]), OPT["blend"]), 1.0),
                           record=rec, path="persist")
                return out
    else:
        o = sc["old"]
        ob = (math.floor(o[0]), math.floor(o[1]))
        of = (o[0] - ob[0], o[1] - ob[1])
        taps = [((ob[0] + x, ob[1] + y), tap_weight(of, x, y)) for y in (0, 1) for x in (0, 1)]
    hist, mass, U, Mu = 0.0, 0.0, set(), 0
    for t, wt in taps:
        if not wt > 0 or t not in sc["prev"]:
            continue
        c, a, z, rec = sc["prev"][t]
        verdict = "accept"
        if not passes(sc["prev"][t]):
            verdict = "depth_or_color"
        elif corr:
            R = read(rec, highest)
            if R is None:
                verdict = "no_record"
            elif not provable or R[0] != A:
                verdict = "anchor"
            elif R[1] is not None and any(o in changed and o not in S for o in R[1]):
                verdict = "changed_absent"
            elif R[1] is None and (R[2] | classes(S)) & classes(changed):
                verdict = "class_hit"
        else:
            R = D["decode"](rec, highest)
            if R is None:
                verdict = "no_record"
            elif R[0] != A_d:
                verdict = "anchor"
        out["taps"].append({"tap": list(t), "weight": wt, "verdict": verdict})
        if verdict != "accept":
            continue
        hist += clean(c, a) * wt
        mass += wt
        if R[1] is not None:
            U |= R[1]
        else:
            Mu |= R[2]
    if not mass > 0:
        out["path"] = "current_only"
        return out
    history = clean(hist / mass, 1.0)
    lo = hi = current
    K, K_ok = set(), True
    for dy in (-1, 0, 1):
        for dx in (-1, 0, 1):
            c, a, z, word = sc["src"][clampt((anchor[0] + dx, anchor[1] + dy))]
            if admitted(c, a) and a >= 1 and compatible(z, nearest):
                lo, hi = min(lo, clean(c, a)), max(hi, clean(c, a))
                if valid(owner_id(word), highest):
                    K.add(owner_id(word))
                else:
                    K_ok = False
    reaction = min(max(abs(history - current) / OPT["scale"], 0.0), 1.0)
    weight = OPT["blend"] * (1 - OPT["reactive"] * reaction)
    out["color"] = clean(mix(current, min(max(history, lo), hi), weight), 1.0)
    out["clamp"] = [lo, hi]
    if corr:
        out["record"] = record(A, K | U | S, Mu, highest) if provable and K_ok else list(ABSENT)
    else:
        out["record"] = D["encode"](A_d, S_d | U, Mu)
    out["path"] = "reproject"
    return out


def grid(w, h, texel):
    return {(x, y): texel for y in range(h) for x in range(w)}


CALM = {"camera_static": True, "global": False}
MOVED = {"camera_static": False, "global": False}


def r12(v):
    return round(v, 12)


# --------------------------------------------------------------- 1. clamp chain
def clamp_chain():
    """Astra: current owner 5 colour 0, history 1, clamp-only owner 7 colour 0.5,
    blend 0.9 -> 0.45; next static frame, 7 changed: draft persists 0.405."""
    src = grid(4, 4, (0.0, 1.0, 10.0, 5))
    src[(0, 1)] = (0.5, 1.0, 10.0, 7)                    # in the 3x3, outside the 2x2
    frame_t = {"size": (4, 4), "src": src, "q": (1.0, 1.0), "p": (1, 1), "old": (1.0, 1.0),
               "still": True, "flags": MOVED, "changed": [],
               "prev": {(1, 1): (1.0, 1.0, 10.0, record(5, {5}))}}
    rows = {}
    for mode in ("draft", "corrected"):
        t = resolve(frame_t, mode)
        src2 = dict(src)
        src2[(0, 1)] = (0.8, 1.0, 10.0, 7 | OVERFLOW)    # owner 7 changed
        frame_t1 = {"size": (4, 4), "src": src2, "q": (1.0, 1.0), "p": (1, 1), "old": (1.0, 1.0),
                    "still": True, "flags": CALM, "changed": [7],
                    "prev": {(1, 1): (t["color"], 1.0, t["depth"], t["record"])}}
        t1 = resolve(frame_t1, mode)
        rows[mode] = {"frame_t": {"path": t["path"], "color": r12(t["color"]), "clamp": t["clamp"],
                                  "record": t["record"], "owners": record_owners(t["record"])},
                      "frame_t1": {"path": t1["path"], "color": r12(t1["color"]), "record": t1["record"]}}
    d, c = rows["draft"], rows["corrected"]
    ok = (d["frame_t"]["color"] == c["frame_t"]["color"] == 0.45
          and d["frame_t"]["owners"] == [5] and c["frame_t"]["owners"] == [5, 7]
          and d["frame_t1"]["path"] == "persist" and d["frame_t1"]["color"] == 0.405
          and c["frame_t1"]["path"] == "drop" and c["frame_t1"]["color"] == 0.0
          and c["frame_t1"]["record"] == record(5, {5}))
    return ok, rows


def record_owners(rec):
    r = read(rec)
    if r is None:
        return None
    return sorted(r[1]) if r[1] is not None else {"anchor": r[0], "mask": r[2]}


# ------------------------------------------------------------- 2. every record
def every_record():
    """Moving pixel, four positive-weight history taps. Tap (1,0) holds Astra's
    record [2147483653, 7]; owner 7 changed and is absent from the current support."""
    src = grid(4, 4, (0.5, 1.0, 10.0, 5))
    src[(1, 1)] = (0.25, 1.0, 10.0, 5)
    src[(0, 0)] = (0.0, 1.0, 10.0, 5)
    src[(2, 2)] = (1.0, 1.0, 10.0, 5)                    # wide clamp range: no clamp
    astra = [2147483653, 7]
    prev = {(0, 0): (0.4, 1.0, 10.0, record(5, {5})),
            (1, 0): (0.9, 1.0, 10.0, astra),
            (0, 1): (0.6, 1.0, 10.0, record(5, {5, 9})),
            (1, 1): (0.5, 1.0, 10.0, record(5, {5}))}
    sc = {"size": (4, 4), "src": src, "q": (1.0, 1.0), "p": (1, 1), "old": (0.25, 0.5),
          "still": False, "flags": CALM, "changed": [7], "prev": prev}
    rows = {m: resolve(sc, m) for m in ("draft", "corrected")}
    scalar = D["decide"](astra, [5], 5, {7}, CALM, False)
    d, c = rows["draft"], rows["corrected"]
    out = {m: {"path": r["path"], "color": r12(r["color"]), "record": r["record"],
               "owners": record_owners(r["record"]), "taps": r["taps"]} for m, r in rows.items()}
    out["astra_scalar_decide"] = [scalar[0], scalar[1]]
    out["expected"] = {"draft_history": 0.55, "corrected_history": 0.5,
                       "corrected_renormalised_mass": 0.875}
    ok = (scalar == ("drop", [2147483653, 0])
          and r12(d["color"]) == r12(mix(0.25, 0.55, 0.9)) and r12(c["color"]) == r12(mix(0.25, 0.5, 0.9))
          and [t["verdict"] for t in c["taps"]] == ["accept", "changed_absent", "accept", "accept"]
          and c["record"] == record(5, {5, 9})
          and read(d["record"])[1] is None and d["record"][1] & (1 << 7))
    return ok, out


# ---------------------------------------- 3. camera_static: matrix, view and eye
def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def fbits(x):
    return struct.unpack("<I", struct.pack("<f", x))[0]


def fnext(x):
    return struct.unpack("<f", struct.pack("<I", fbits(x) + 1))[0]


def axis_camera(eye_x, w, h, r, near, eye_z=8.0):
    """CameraView looking down -Z with +Y up: glam look_to_rh gives s=(1,0,0) and
    u=(0,1,0) exactly, so view translation is (-eye.x, -eye.y, -eye.z). The only
    eye.x-dependent unjittered matrix entry is round32(w * -eye.x): every other
    term in that row is an exact zero product, with or without FMA."""
    view_t = [f32(-eye_x), 0.0, f32(-eye_z), 1.0]
    vp_t = [f32(w * view_t[0]), 0.0, f32(f32(r * view_t[2]) + f32(r * f32(near))), f32(-view_t[2])]
    return {"vp_translation_bits": [fbits(v) for v in vp_t],
            "view_translation_bits": [fbits(v) for v in view_t],
            "shading_eye_bits": [fbits(f32(eye_x)), 0, fbits(f32(eye_z)), fbits(1.0)]}


def camera_identity():
    near, far = 0.1, 100.0
    h = f32(f32(math.cos(f32(math.radians(30.0)))) / f32(math.sin(f32(math.radians(30.0)))))
    w = f32(h / f32(16.0 / 9.0))
    r = f32(f32(far) / f32(f32(near) - f32(far)))
    a = f32(1.5)
    while f32(w * a) != f32(w * fnext(a)):
        a = fnext(a)
    one, two = axis_camera(a, w, h, r, near), axis_camera(fnext(a), w, h, r, near)
    same_vp = one["vp_translation_bits"] == two["vp_translation_bits"]
    draft_static = same_vp
    corrected_static = same_vp and one["view_translation_bits"] == two["view_translation_bits"] \
        and one["shading_eye_bits"] == two["shading_eye_bits"]
    # Every multiplier w in (0.5, 1) collides somewhere: on a in [1/w, 2) the
    # products stay in [1, 2) and there are 1/w times more inputs than outputs.
    scan = {}
    for m in (0.55, 0.75, 0.9, 0.99):
        wm = f32(m)
        b = f32(1.0 / wm)
        if wm * b < 1.0:
            b = fnext(b)
        steps = 0
        while f32(wm * b) != f32(wm * fnext(b)):
            b, steps = fnext(b), steps + 1
        scan[str(m)] = {"eye_x": b, "next_eye_x": fnext(b), "steps": steps}
    # DRS at a fixed output keeps has_previous (the upscaled Extent ignores the
    # internal rect). A proportional step keeps aspect, matrix, view and eye bits,
    # but jitter::guard = 1/rect size changes, and cascade::fit scales receivers by
    # 1 + guard (and the cluster grid reads the rect), so shading inputs differ.
    drs = {}
    for size in (2048, 1920):
        s = f32(float(size))
        drs[str(size)] = {"aspect_bits": fbits(f32(s / s)), "guard_bits": fbits(f32(1.0 / s))}
    drs_eye_only_static = drs["2048"]["aspect_bits"] == drs["1920"]["aspect_bits"]
    drs_corrected_static = drs_eye_only_static and drs["2048"]["guard_bits"] == drs["1920"]["guard_bits"]
    ok = (same_vp and draft_static and not corrected_static and fbits(a) != fbits(fnext(a))
          and drs_eye_only_static and not drs_corrected_static)
    return ok, {"multiplier_w_bits": fbits(w), "eye_x": [a, fnext(a)], "eye_x_bits": [fbits(a), fbits(fnext(a))],
                "unjittered_vp_equal": same_vp, "view_equal": one["view_translation_bits"] == two["view_translation_bits"],
                "shading_eye_equal": one["shading_eye_bits"] == two["shading_eye_bits"],
                "draft_camera_static": draft_static, "corrected_camera_static": corrected_static,
                "collision_scan": scan,
                "drs_2048_to_1920": {"rects": drs, "matrix_view_eye_only_static": drs_eye_only_static,
                                     "corrected_static_with_rect": drs_corrected_static}}


# ------------------------------------------- 4. canonical, positive weight, malformed
def canonical_and_malformed():
    rows, ok = [], True

    def row(name, draft, corrected, good):
        nonlocal ok
        ok = ok and bool(good)
        rows.append({"case": name, "draft": draft, "corrected": corrected, "ok": bool(good)})

    enc = D["encode"](0, {7})
    row("encode(0,{7}) (Astra canonical_clear_case)", {"written": enc, "read": D["decode"](enc, HIGHEST)},
        {"written": record(0, {7})}, enc == [2147483648, 7] and record(0, {7}) == ABSENT)
    clear = D["encode"](0, set())
    row("present-clear record [P|0,0]", {"written_at_clear_exits": clear, "read": list(D["decode"](clear, HIGHEST)[:1])},
        {"read": read(clear)}, D["decode"](clear, HIGHEST) is not None and read(clear) is None)
    bad_mask = [PRESENT | 5, OVERFLOW | classes({9, 11})]
    row("overflow mask without the anchor class", {"read_ok": D["decode"](bad_mask, HIGHEST) is not None},
        {"read_ok": read(bad_mask) is not None}, D["decode"](bad_mask, HIGHEST) is not None and read(bad_mask) is None)
    row("draft decide() cases unchanged under corrected decision", None, None, draft_cases_hold())

    # Every early exit of resolve(), corrected writer.
    base = grid(4, 4, (0.25, 1.0, 10.0, 5))
    sc = lambda **k: dict({"size": (4, 4), "src": base, "q": (1.25, 1.25), "p": (1, 1), "old": (1.0, 1.0),
                           "still": True, "flags": CALM, "changed": [], "prev": {}}, **k)
    clear_src = grid(4, 4, (0.0, 0.0, 0.0, 0))
    edge_src = dict(base)
    edge_src[(2, 1)] = (0.25, 0.5, 10.0, 9)              # alpha < 1: not one surface
    untracked = grid(4, 4, (0.25, 1.0, 9.004, 7))        # one surface with owner 7
    untracked[(1, 1)] = (0.25, 1.0, 9.0, 0)              # nearest tap, owner 0, depth > 0
    beyond = dict(base)
    beyond[(2, 2)] = (0.25, 1.0, 10.0, HIGHEST + 1)
    high_bits = dict(base)
    high_bits[(2, 2)] = (0.25, 1.0, 10.0, 5 | (1 << 20))
    exits = {
        "outside": (sc(inside=False), "outside"),
        "all clear": (sc(src=clear_src), "all_clear"),
        "not one surface": (sc(src=edge_src), "not_one_surface"),
        "history disabled": (sc(history=False), "history_disabled"),
        "invalid motion": (sc(motion=False), "invalid_motion"),
        "old outside": (sc(old=None), "old_outside"),
        "no admitted tap": (sc(), "current_only"),
        "untracked anchor (owner 0 at depth)": (sc(history=False, src=untracked), "history_disabled"),
        "support id above highest": (sc(history=False, src=beyond), "history_disabled"),
        "owner word bits 17-30 set": (sc(history=False, src=high_bits), "history_disabled"),
    }
    exit_rows = {}
    for name, (case, path) in exits.items():
        d, c = resolve(case, "draft"), resolve(case, "corrected")
        canonical = c["record"] == ABSENT or read(c["record"]) is not None
        expect_absent = name in ("outside", "all clear", "not one surface") or "owner" in name \
            or "highest" in name or "17-30" in name
        good = c["path"] == path and canonical and ((c["record"] == ABSENT) == expect_absent)
        if expect_absent is False:
            good = good and c["record"] == record(5, {5})
        if c["depth"] == 0.0:
            good = good and c["record"] == ABSENT
        exit_rows[name] = {"path": c["path"], "depth": c["depth"], "draft_record": d["record"],
                           "draft_record_readable": read(d["record"]) is not None if d["record"] != ABSENT else None,
                           "corrected_record": c["record"], "ok": good}
        ok = ok and good
    malformed_draft = exit_rows["untracked anchor (owner 0 at depth)"]["draft_record"]
    ok = ok and malformed_draft == [2147483648, 7]

    # A malformed previous record at the only history tap is rejected, not carried.
    prev = {(1, 1): (0.9, 1.0, 10.0, [2147483648, 7])}
    held = resolve(sc(prev=prev, flags=MOVED), "corrected")
    good = held["path"] == "current_only" and held["taps"][0]["verdict"] == "no_record" \
        and held["record"] == record(5, {5})
    ok = ok and good
    exit_rows["malformed previous record [P|0,7]"] = {"path": held["path"], "taps": held["taps"],
                                                      "corrected_record": held["record"], "ok": good}

    # Positive-weight selection: a changed recorded owner present only in a
    # zero-weight tap is absent. f=(0,0): only tap (1,1) has weight.
    zw = dict(base)
    zw[(2, 1)] = (0.25, 1.0, 10.0, 7)
    case = sc(src=zw, q=(1.0, 1.0), changed=[7], prev={(1, 1): (0.9, 1.0, 10.0, record(5, {5, 7}))})
    c = resolve(case, "corrected")
    wrong = decide_c(record(5, {5, 7}), {5, 7}, 5, {7}, CALM, True)
    good = c["path"] == "drop" and c["record"] == record(5, {5}) and wrong[0] == "reproject"
    ok = ok and good
    exit_rows["zero-weight tap cannot establish owner 7"] = {
        "path": c["path"], "record": c["record"], "if_zero_weight_counted": wrong[0], "ok": good}
    return ok, {"records": rows, "exits": exit_rows}


def draft_cases_hold():
    """The 15 draft decide() cases: same branch (no_history for reproject_no_record)
    and same record when K = S and every id is valid."""
    for case in D["provenance_cases"]():
        rec = case["record_in"]
        name = case["case"]
        support = {"unchanged static pixel persists": [5, 9], "thin owner missing this phase still persists": [5],
                   "changed recorded owner absent from support drops": [5],
                   "changed recorded owner still present reprojects": [5, 7],
                   "changed newcomer in support blocks persistence": [5, 3]}.get(name, [5, 9] if "overflow" in name or "unrelated" in name else [5])
        changed = {"changed recorded owner absent from support drops": [7],
                   "changed recorded owner still present reprojects": [7],
                   "changed newcomer in support blocks persistence": [3],
                   "overflow without class hit persists": [43],
                   "overflow collision with a non-contributor drops (cost)": [40],
                   "overflow with class hit drops": [42],
                   "unrelated changed owner does not disturb exact record": [42]}.get(name, [])
        flags = {"camera change blocks persistence": MOVED,
                 "light or caster change blocks persistence": {"camera_static": True, "global": True}}.get(name, CALM)
        still = name != "moving pixel reprojects"
        branch, out = decide_c(rec, support, 5, set(changed), flags, still)
        draft_branch, draft_out = D["decide"](rec, support, 5, set(changed), flags, still)
        expected = {"reproject_no_record": "no_history"}.get(draft_branch, draft_branch)
        if branch != expected or out != draft_out or draft_branch != case["branch"] or draft_out != case["record_out"]:
            return False
    return True


# ---------------------------------------------------------------- 5. submissions
GPU_CASES = [
    # name, owner renders, baseline Upscale renders, explicit queue submits
    ("fresh record: history disabled; outside texels (0,0)", 1, 0, 1),
    ("static persistence, history outside the clamp range", 1, 0, 1),
    ("changed absent owner drops: record {5,7}, 7 changed (Astra t+1), plus 7 only in a zero-weight tap", 1, 0, 1),
    ("overflow class hit drops", 1, 0, 1),
    ("overflow without a hit persists", 1, 0, 1),
    ("tolerance-close tap with a different anchor owner rejected", 1, 0, 1),
    ("global set: colour/depth equal the existing Upscale pipeline", 1, 1, 1),
    ("camera not static: colour/depth equal the existing Upscale pipeline", 1, 1, 1),
    ("clamp-only owner recorded: colour 0.45, record {5,7} (Astra t)", 1, 0, 1),
    ("four history records: per-tap rejection and renormalisation", 1, 0, 1),
    ("untracked anchor, id above highest, alpha<1 edge, malformed previous record", 1, 0, 1),
    ("all clear (depth cleared to 1.0): every record (0,0)", 1, 0, 1),
]


def submissions():
    owner = sum(c[1] for c in GPU_CASES)
    baseline = sum(c[2] for c in GPU_CASES)
    submits = sum(c[3] for c in GPU_CASES)
    draft_cases, draft_baselines = 8, 2
    out = {
        "draft_marker": "OWNER_HISTORY renders=8 readback_submissions=8 max_extent=32x32 no_images=true",
        "draft_claimed_submissions": 16,
        "draft_actual_renders": draft_cases + draft_baselines,
        "draft_actual_if_unbatched": 2 * (draft_cases + draft_baselines),
        "corrected": {"cases": len(GPU_CASES), "owner_renders": owner, "baseline_renders": baseline,
                      "queue_submits": submits, "readback_maps": len(GPU_CASES),
                      "marker": ("OWNER_HISTORY cases={} owner_renders={} baseline_renders={} queue_submits={} "
                                 "readback_maps={} max_extent=32x32 no_images=true").format(
                                     len(GPU_CASES), owner, baseline, submits, len(GPU_CASES))},
        "cases": [{"case": c[0], "owner_renders": c[1], "baseline_renders": c[2], "queue_submits": c[3]}
                  for c in GPU_CASES],
    }
    ok = (out["draft_actual_renders"] == 10 and out["draft_actual_if_unbatched"] == 20
          and owner == 12 and baseline == 2 and submits == 12)
    return ok, out


# ------------------------------------------- 6. admission, upload storage, reset
P = 4 * 1024 * 1024
INSTANCE = 64 * 1024 * 1024
FIXED = 16384
CAP = 128 * P + 2 * INSTANCE + FIXED
SUB = {"motion": 24 * P, "owner": 4 * P, "output": 32 * P, "overlay": 8 * P, "record": 16 * P}
UNIFORM = 16 + 782 * 16                                  # OwnerChanges header + words
LIGHTS = (16 + 4096) * 64                                # MAX_DIRECTIONAL + MAX_LOCAL, 64 B each


class Refused(Exception):
    pass


def px(size):
    if 0 in size or max(size) > 8192 or size[0] * size[1] > P:
        raise Refused("extent")
    return size[0] * size[1]


def comp(old, new, stride, name):
    """old/new: (extent, flag) or None. Exact Arc reuse needs extent AND flag."""
    candidate = px(new[0]) * stride if new else 0
    old_bytes = px(old[0]) * stride if old else 0
    if old_bytes > SUB[name] or candidate > SUB[name] or old_bytes + candidate > 2 * SUB[name]:
        raise Refused(name)
    return old_bytes + (0 if old == new else candidate)


def admit_c(source, old_motion, output, old_output, old_overlay, attach=True, rec=True):
    total = 2 * INSTANCE + FIXED
    total += comp(old_motion, (source, attach), 24, "motion")
    total += comp(old_motion if old_motion and old_motion[1] else None, (source, True) if attach else None, 4, "owner")
    total += comp(old_output, (output, rec), 32, "output")
    total += comp(old_output if old_output and old_output[1] else None, (output, True) if rec else None, 16, "record")
    total += comp((old_overlay, True) if old_overlay else None, (source, True), 8, "overlay")
    if total > CAP:
        raise Refused("ceiling")
    return total


def attempt_c(*a, **k):
    try:
        return admit_c(*a, **k)
    except Refused as e:
        return "refused:" + str(e)


def admission_and_reset():
    full, wide = (2048, 2048), (2048, 1920)
    same = {}
    rows = {"steady 2048^2 16/16": (full, full, full, full, full),
            "DRS 16->15 at 2048^2": ((1920, 1920), full, full, full, full),
            "DRS 15->16 at 2048^2": (full, (1920, 1920), full, full, (1920, 1920)),
            "resize 2048^2 -> 2048x1920 16/16": (wide, full, wide, full, full),
            "resize 1920x1080 -> 2560x1440 16/16": ((2560, 1440), (1920, 1080), (2560, 1440), (1920, 1080), (1920, 1080)),
            "resize 2560x1440 -> 2560x1600 8/16": ((1280, 800), (1280, 720), (2560, 1600), (2560, 1440), (1280, 720)),
            "steady 2560x1600 8/16": ((1280, 800), (1280, 800), (2560, 1600), (2560, 1600), (1280, 800))}
    for label, (src, om, out, oo, ov) in rows.items():
        draft = D["attempt"](src, om, out, oo, ov, meta=True)
        corr = attempt_c(src, (om, True), out, (oo, True), ov)
        same[label] = {"draft_with_record": draft, "corrected_flags_equal": corr, "equal": draft == corr}
    # Same extent, different flags: the draft rule counts one allocation where two live.
    mismatch = {
        "output record flag differs at 2048^2": {
            "draft_extent_only": D["attempt"](full, full, full, full, full, meta=True),
            "corrected": attempt_c(full, (full, True), full, (full, False), full)},
        "motion owner flag differs at 2048^2": {
            "draft_extent_only": D["attempt"](full, full, full, full, full, meta=True),
            "corrected": attempt_c(full, (full, False), full, (full, True), full)},
    }
    for m in mismatch.values():
        m["undercount"] = m["corrected"] - m["draft_extent_only"]
    # Charged upload and retained storage, inside the 64 MiB motion instance sub-cap
    # (the aggregate already counts 2 x 64 MiB there, so the ceiling is unchanged).
    charges = {"owner_changes_uniform_gpu": UNIFORM, "owner_changes_upload_cpu": UNIFORM,
               "accepted_lights_cpu_max": LIGHTS}
    charges["total"] = sum(charges.values())
    words = (HIGHEST + 1 + 31) // 32
    # Reset: refused resize, then cut, then enable_temporal_upscaling.
    state = {"motion": (full, True), "output": (full, True), "overlay": full}
    log = []

    def submit(source, output):
        r = attempt_c(source, state["motion"], output, state["output"], state["overlay"])
        if isinstance(r, int):
            state.update(motion=(source, True), output=(output, True), overlay=source)
        log.append({"submit": [list(source), list(output)], "result": r,
                    "retained": {k: (list(v[0]) if isinstance(v, tuple) and isinstance(v[0], tuple) else v)
                                 for k, v in state.items()}})
        return r

    first = submit(wide, wide)
    log.append({"op": "invalidate_motion_history (cut): MotionState::invalidate only; Targets, Output, Overlay kept"})
    after_cut = submit(wide, wide)
    state.update(motion=None, output=None, overlay=None)
    log.append({"op": "enable_temporal_upscaling(same config): new State (no Output/Overlay), new MotionState "
                      "(targets None), attachment flag preserved"})
    after_reset = submit(wide, wide)
    fresh_expected = 84 * wide[0] * wide[1] + 2 * INSTANCE + FIXED
    ok = (all(r["equal"] for r in same.values())
          and mismatch["output record flag differs at 2048^2"]["undercount"] == 32 * P
          and mismatch["motion owner flag differs at 2048^2"]["undercount"] == 24 * P
          and CAP == 671105024 and words <= 782 * 4 and charges["total"] < INSTANCE
          and first == after_cut == "refused:ceiling" and after_reset == fresh_expected)
    return ok, {"cap": CAP, "rows_flags_equal": same, "flag_mismatch": mismatch,
                "instance_subcap_charges": charges, "provenance_words_needed": words,
                "uniform_words": 782 * 4, "reset_sequence": log}


def report():
    checks, details = [], {}
    for name, fn in (("clamp provenance", clamp_chain), ("every reprojected record", every_record),
                     ("camera_static identity", camera_identity),
                     ("canonical anchors, positive weight, clear and malformed exits", canonical_and_malformed),
                     ("future GPU submissions", submissions),
                     ("admission flags, upload charges, reset", admission_and_reset)):
        ok, detail = fn()
        checks.append({"check": name, "ok": bool(ok)})
        details[name] = detail
    return checks, details


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    checks, details = report()
    text = json.dumps({"model": "0073 implementation-5 correction controls", "python": sys.version.split()[0],
                       "draft_model_sha256": DRAFT_SHA, "checks": checks, "details": details}, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 0 if all(c["ok"] for c in checks) else 1


if __name__ == "__main__":
    sys.exit(main())
