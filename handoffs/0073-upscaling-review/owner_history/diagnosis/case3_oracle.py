#!/usr/bin/env python3
"""0073 owner-history case-3 diagnosis oracle. Standard library, deterministic,
CPU only: no GPU, no renderer binary, no build, no image.

    python3 -I -B handoffs/0073-upscaling-review/owner_history/diagnosis/case3_oracle.py \
        --out handoffs/0073-upscaling-review/owner_history/diagnosis/results.json

An f32-exact scalar transcription of the frozen resolve_owner.wgsl (df600750) runs
on all 12 fixtures of the frozen owner_device_tests.rs (07b168aa); both files and
the raw GPU001 stdout are sha-pinned. Basic f32 operations are computed in binary64
then rounded once (innocuous double rounding, 53 >= 2*24+2); fused multiply-add is
rounded exactly from rationals. Variables:
  arithmetic: how the shared u=(p-o)/w division and its multiply-add round;
  table/index: changed-table read and support-vector store mutations;
  f16 colour storage: round-to-nearest-even or toward zero (hypotheses only);
  patched: the proposed anchor-only changed-owner admission (diagnosis patch).
The oracle reproduces what each mechanism would print. It cannot tell which code
path the Metal GPU actually executed.
"""
import argparse
import hashlib
import json
import math
import os
import struct
import sys
from fractions import Fraction as Q

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
FROZEN = "/Users/andreyyurlov/.codex/incant-evidence/temporal-owner-consumer-001/source-final/crates/incant_render/src/upscaling"
PINS = {
    os.path.join(FROZEN, "resolve_owner.wgsl"): "df600750ab5cad117ae0cfa1e76fa635e07c96be4a76d8aa9e8470d9954c64ae",
    os.path.join(FROZEN, "owner_device_tests.rs"): "07b168aa4f9c5db51537beb0f16fbbb80b5cf0e6fae11f29148b40f03068d36e",
    "/Users/andreyyurlov/.codex/incant-evidence/0073-owner-history-gpu-001/wgsl/stdout.txt":
        "bf7e639747a3702f0647a928ab06d3ced68ee501aec89d2306d033620577442c",
    "/Users/andreyyurlov/.codex/incant-evidence/0073-owner-history-gpu-001/wgsl/stderr.txt":
        "21c22e6b7f042bfa94944bec374d90aa218832618ed2a54d3033c09a89b70640",
}
PATCH = os.path.join(HERE, "resolve_owner-anchor-change.patch")
PATCHED_SHA = "0618bd7793f2ef53db7d34bc2020ba515bb722b5a3e5729912cf1f5c5c408777"


def apply_patch(text, patch):
    """Exact-context unified-diff application, each hunk matched once."""
    hunks, cur = [], None
    for line in patch.splitlines(keepends=True):
        if line.startswith("@@"):
            cur = ([], [])
            hunks.append(cur)
        elif cur is not None and line[:1] in (" ", "-", "+"):
            if line[0] in " -":
                cur[0].append(line[1:])
            if line[0] in " +":
                cur[1].append(line[1:])
    for old, new in hunks:
        old, new = "".join(old), "".join(new)
        if text.count(old) != 1:
            raise SystemExit("patch context not unique")
        text = text.replace(old, new)
    return text


def patch_binding():
    with open(os.path.join(FROZEN, "resolve_owner.wgsl")) as fh, open(PATCH) as ph:
        frozen, patch = fh.read(), ph.read()
    patched = apply_patch(frozen, patch)
    digest = hashlib.sha256(patched.encode()).hexdigest()
    return {"patch_sha256": hashlib.sha256(patch.encode()).hexdigest(), "patched_shader_sha256": digest,
            "matches_pinned_patched_sha": digest == PATCHED_SHA,
            "rule_text_present": "return r.b==0u || !changed_id(r.b);" in patched,
            "presence_rule_removed": "gone_b" not in patched and "any(support==" not in patched,
            "support_still_built": "support[u32(y*2+x)]=n;" in patched and "support[u32(i)]" in patched,
            "ascii": all(ord(ch) < 128 for ch in patched)}


def pinned():
    for path, sha in PINS.items():
        with open(path, "rb") as f:
            if hashlib.sha256(f.read()).hexdigest() != sha:
                raise SystemExit("pinned input changed: " + path)


# ------------------------------------------------------------------ f32 / f16
def f(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def f32_exact(q):
    """Round a rational to the nearest f32 (ties to even)."""
    if q == 0:
        return 0.0
    sign, q = (-1.0 if q < 0 else 1.0), abs(q)
    e = q.numerator.bit_length() - q.denominator.bit_length()
    if Q(2) ** e > q:
        e -= 1
    scale = Q(2) ** (max(e, -126) - 23)
    m = q / scale
    whole = m.numerator // m.denominator
    rest = m - whole
    if rest > Q(1, 2) or (rest == Q(1, 2) and whole % 2):
        whole += 1
    return sign * float(whole * scale)


def fma(a, b, c):
    return f32_exact(Q(a) * Q(b) + Q(c))


def step(x, k):
    bits = struct.unpack("<I", struct.pack("<f", x))[0]
    return struct.unpack("<f", struct.pack("<I", bits + k))[0]


def f16_rn(x):
    return struct.unpack("<e", struct.pack("<e", x))[0]


def f16_rz(x):
    if x == 0 or not math.isfinite(x):
        return x
    a = abs(x)
    e = math.frexp(a)[1] - 1
    spacing = 2.0 ** (max(e, -14) - 10)
    return math.copysign(math.floor(a / spacing) * spacing, x)


def rust_f32(x):
    """Rust {:?} for f32: shortest round-trip digits."""
    if x == int(x) and abs(x) < 1e16:
        return "%d.0" % int(x) if x != 0 else ("-0.0" if math.copysign(1, x) < 0 else "0.0")
    for digits in range(1, 10):
        s = "%.*g" % (digits, x)
        if f(float(s)) == x:
            if "e" in s:
                mant, exp = s.split("e")
                return "%se%d" % (mant, int(exp))
            return s
    return repr(x)


# ------------------------------------------------------------------- fixtures
N, PRESENT, CENTER = 16, 1 << 31, (8, 8)


def classes(ids):
    mask = 0
    for i in ids:
        mask |= 1 << (i % 31)
    return mask


def fixture(case):
    """Transcription of fixture() in owner_device_tests.rs (f32 literals and ops)."""
    ortho = case in (3, 5)
    z = f(50.05) if ortho else f(1.0 / f(f(0.5 / f(0.1)) + f(0.5 / 100.0)))
    grid = lambda v: {(x, y): v for y in range(N) for x in range(N)}
    c = {"case": case, "output": (1.0, 1.0, 14.0, 14.0), "source": (1.0, 1.0, 14.0, 14.0),
         "near": f(0.1), "far": 100.0,
         "ortho": ortho, "history": 1.0, "options": (f(0.9), f(0.001), f(0.001), 1.0), "reactive": 0.0,
         "current": grid((0.25, 1.0)), "motion": grid((0.0, 0.0, 0.5, 1.0)), "old": grid((0.75, 1.0)),
         "old_depth": grid(z), "owners": grid(5), "records": grid([PRESENT | 5, 0]),
         "header": [0, 1, 100000, 0], "table": {}, "depth": 0.5, "probes": [], "baseline": case in (7, 8), "z": z}
    color, record = (0.25, 1.0), [PRESENT | 5, 0]
    if case == 1:
        c["history"] = 0.0
    elif case == 2:
        c["records"] = grid([PRESENT | 7, 5])
        color, record = (f(f(0.25 * f(0.1)) + f(0.75 * f(0.9))), 1.0), [PRESENT | 5, 7]
    elif case == 3:
        c["current"], c["old"], c["records"] = grid((0.0, 1.0)), grid((f(0.45), 1.0)), grid([PRESENT | 5, 7])
        c["owners"][(9, 8)], c["current"][(9, 8)] = 7, (0.5, 1.0)
        c["header"][0], c["table"][0] = classes([7]), 1 << 7
        color = (0.0, 1.0)
    elif case in (4, 5):
        mask = classes([5, 7, 9])
        c["records"] = grid([PRESENT | 5, PRESENT | mask])
        if case == 4:
            c["header"][0], c["table"][0] = classes([7]), 1 << 7
        else:
            color, record = (f(f(0.25 * f(0.1)) + f(0.75 * f(0.9))), 1.0), [PRESENT | 5, PRESENT | mask]
    elif case == 6:
        c["header"][1] = 0
        c["records"], c["old_depth"] = grid([PRESENT | 9, 0]), grid(f(z + f(0.00025)))
    elif case in (7, 8):
        c["header"][1] = 3 if case == 7 else 0
        for y in range(N):
            for x in range(N):
                c["current"][(x, y)] = (0.25 if (x + y) % 2 == 0 else 64.0, 1.0)
                c["old"][(x, y)] = (0.25 if (x + y) % 2 == 0 else 32.0, 1.0)
        delta = (f(3.0 / 1024.0), 0.0) if case == 7 else (0.25, 0.5)
        c["motion"] = grid((f(delta[0] / 14.0), f(delta[1] / 14.0), 0.5, 1.0))
    elif case == 9:
        c["header"][1] = 0
        c["current"], c["old"] = grid((0.0, 1.0)), grid((1.0, 1.0))
        c["current"][(7, 8)], c["owners"][(7, 8)] = (0.5, 1.0), 7
        color, record = (f(0.45), 1.0), [PRESENT | 5, 7]
    elif case == 10:
        c["header"][1], c["header"][0], c["table"][0] = 0, classes([7]), 1 << 7
        c["motion"] = grid((f(0.25 / 14.0), f(0.5 / 14.0), 0.5, 1.0))
        c["old"] = grid((0.5, 1.0))
        c["old"][(9, 8)], c["records"][(9, 8)], c["records"][(8, 9)] = (f(0.9), 1.0), [PRESENT | 5, 7], [PRESENT | 5, 9]
        c["current"][(7, 8)], c["current"][(8, 7)] = (0.0, 1.0), (1.0, 1.0)
        color, record = (f(f(0.25 * f(0.1)) + f(0.5 * f(0.9))), 1.0), [PRESENT | 5, 9]
    elif case == 11:
        for p, word in (((3, 3), 0), ((11, 3), 100001), ((6, 3), 5 | (1 << 20))):
            c["owners"][p] = word
            c["probes"].append((p, (0.25, 1.0), z, [0, 0]))
        c["current"][(3, 11)] = (0.125, 0.5)
        c["probes"].append(((3, 11), (0.125, 0.5), 0.0, [0, 0]))
        c["records"][(11, 11)] = [PRESENT, 7]
        c["probes"].append(((11, 11), (0.25, 1.0), z, [PRESENT | 5, 0]))
        c["records"][(8, 11)] = [PRESENT | 5 | (1 << 20), 0]
        c["probes"].append(((8, 11), (0.25, 1.0), z, [PRESENT | 5, 0]))
        color = (f(f(0.25 * f(0.1)) + f(0.75 * f(0.9))), 1.0)
    elif case == 12:
        c["depth"], c["current"] = 1.0, grid((0.0, 0.0))
    if not c["baseline"] and case != 12:
        c["probes"].append((CENTER, color, z, record))
    return c


# ------------------------------------------------- f32 shader transcription
ARITH = {  # (division of the shared u, multiply-add)
    "ieee": ("ieee", "sep"), "ieee+fma": ("ieee", "fma"), "recip": ("recip", "sep"),
    "recip+fma": ("recip", "fma"), "quot+1": (1, "sep"), "quot+1+fma": (1, "fma"),
    "quot+2": (2, "sep"), "quot+2+fma": (2, "fma"), "quot-1": (-1, "sep"), "quot-1+fma": (-1, "fma"),
}


def divide(a, b, kind):
    if kind == "ieee":
        return f(a / b)
    if kind == "recip":
        return f(a * f(1.0 / b))
    return step(f(a / b), kind)


def madd(o, u, w, kind):
    return fma(u, w, o) if kind == "fma" else f(o + f(u * w))


def mix(x, y, a, form):
    if form == "metal_fma":
        return fma(f(y - x), a, x)
    if form == "wgsl":
        return f(f(x * f(1.0 - a)) + f(y * a))
    return f(x + f(f(y - x) * a))


def resolve(c, arith="ieee", table="correct", index="correct", patched=False, form="metal_sep", drop_rule=False):
    """Returns {(x, y): (color, alpha, depth, record)} for the whole 16x16 target."""
    div_kind, madd_kind = ARITH[arith]
    o, near, far = c["output"], c["near"], c["far"]
    lo, hi = math.ceil(o[0] - 0.5), math.ceil(o[0] + o[2] - 0.5) - 1          # 1, 14
    header, words = c["header"], c["table"]
    highest = min(header[2], 100000)

    def valid(n):
        return 1 <= n <= highest

    def word_at(k):
        return words.get(k, 0)

    def changed(n):
        if table == "zero_table":
            return False
        k = n >> 5
        if table == "component_reversed":
            k = ((n >> 7) << 2) | (3 - ((n >> 5) & 3))
        elif table == "next_vec4":
            k = (((n >> 7) + 1) << 2) | ((n >> 5) & 3)
        elif table == "header_aliased":                                   # table read from offset 0
            return ((header[k] if k < 4 else word_at(k - 4)) >> (n & 31)) & 1 != 0
        bit = 31 - (n & 31) if table == "bit_reversed" else n & 31
        return (word_at(k) >> bit) & 1 != 0

    cls = lambda n: 1 << (n % 31)

    def lin(d):
        if not (0.0 <= d < 1.0):
            return 0.0
        if c["ortho"]:
            return f(f(f(1.0 - d) * near) + f(d * far))
        return f(1.0 / f(f(f(1.0 - d) / near) + f(d / far)))

    def compatible(a, b):
        opt = c["options"]
        return a > 0 and b > 0 and a <= far and b <= far and abs(f(a - b)) <= max(opt[2], f(opt[1] * b))

    def admitted(v, a):
        return all(math.isfinite(t) and abs(t) <= 3.402823e38 for t in (v, a)) and v >= 0 and a >= 0 and a <= 1

    def clean(v, a):
        if not all(math.isfinite(t) and abs(t) <= 3.402823e38 for t in (v, a)) or a <= 0:
            return 0.0, 0.0
        return min(max(v, 0.0), 65504.0), min(max(a, 0.0), 1.0)

    def tap_weight(fr, x, y):
        return f((fr[0] if x else f(1.0 - fr[0])) * (fr[1] if y else f(1.0 - fr[1])))

    def owner_set(a):
        return {"a": a, "b": 0, "m": cls(a), "over": False, "ok": valid(a)}

    def with_id(s, n):
        s = dict(s)
        if not valid(n):
            s["ok"] = False
            return s
        s["m"] |= cls(n)
        if n != s["a"] and n != s["b"]:
            if s["b"] == 0:
                s["b"] = n
            else:
                s["over"] = True
        return s

    def encode(s):
        return [PRESENT | s["a"], (PRESENT | s["m"]) if s["over"] else s["b"]] if s["ok"] else [0, 0]

    def stored(v):
        a = v[0] & 0x1FFFF
        s = {"a": a, "b": 0, "m": 0, "over": False, "ok": False}
        if not v[0] & PRESENT or v[0] & ~(PRESENT | 0x1FFFF) & 0xFFFFFFFF or not valid(a):
            return s
        if v[1] & PRESENT:
            s.update(m=v[1] & 0x7FFFFFFF, over=True, ok=bool(v[1] & cls(a)))
        else:
            s.update(b=v[1], ok=v[1] == 0 or (v[1] != a and valid(v[1])))
        return s

    def with_record(s, r):
        s = with_id(s, r["a"])
        if r["over"]:
            s = dict(s, m=s["m"] | r["m"], over=True)
        elif r["b"]:
            s = with_id(s, r["b"])
        return s

    def unchanged(r, sc, cc):
        if r["over"]:
            return ((r["m"] | sc) & cc) == 0
        return not changed(r["a"]) and (r["b"] == 0 or not changed(r["b"]))

    def admissible(r, support, sc, cc):
        if drop_rule:
            return True
        if r["over"]:
            return ((r["m"] | sc) & cc) == 0
        if patched:                                   # only the anchor (r.a) may have changed
            return r["b"] == 0 or not changed(r["b"])
        gone_a = changed(r["a"]) and r["a"] not in support
        gone_b = r["b"] != 0 and changed(r["b"]) and r["b"] not in support
        return not gone_a and not gone_b

    clampt = lambda t: (min(max(t[0], lo), hi), min(max(t[1], lo), hi))
    zl = lin(c["depth"])
    out, weights = {}, {}
    for ty in range(N):
        for tx in range(N):
            p = (tx + 0.5, ty + 0.5)
            if not (p[0] >= o[0] and p[1] >= o[1] and p[0] < f(o[0] + o[2]) and p[1] < f(o[1] + o[3])):
                out[(tx, ty)] = (0.0, 0.0, 0.0, [0, 0])
                continue
            u = (divide(f(p[0] - o[0]), o[2], div_kind), divide(f(p[1] - o[1]), o[3], div_kind))
            s_ = c["source"]
            q = (f(madd(s_[0], u[0], s_[2], madd_kind) - 0.5), f(madd(s_[1], u[1], s_[3], madd_kind) - 0.5))
            base = (math.floor(q[0]), math.floor(q[1]))
            fr = (f(q[0] - base[0]), f(q[1] - base[1]))
            anchor, nearest, same, col, cola = clampt(base), f(far + 1.0), True, 0.0, 0.0
            support, sc, sok, schanged, cc = [0, 0, 0, 0], 0, True, False, header[0]
            taps = []
            for y in (0, 1):
                for x in (0, 1):
                    w = tap_weight(fr, x, y)
                    if w <= 0:
                        continue
                    t = clampt((base[0] + x, base[1] + y))
                    v, a = c["current"][t]
                    cv, ca = clean(v, a)
                    col, cola = f(col + f(cv * w)), f(cola + f(ca * w))
                    taps.append((t, w))
                    if not admitted(v, a) or a < 1 or zl <= 0:
                        same = False
                    if 0 < zl < nearest:
                        nearest, anchor = zl, t
                    if zl > 0:
                        word = c["owners"][t]
                        n = word & 0x7FFFFFFF
                        slot = {"correct": y * 2 + x, "slot0": 0, "reversed": 3 - (y * 2 + x)}.get(index)
                        if slot is not None:
                            support[slot] = n
                        if not valid(n):
                            sok = False
                        else:
                            sc |= cls(n)
                            if word & PRESENT or changed(n):
                                schanged, cc = True, cc | cls(n)
            weights[(tx, ty)] = taps
            if nearest > far:
                out[(tx, ty)] = (col, cola, 0.0, [0, 0])
                continue
            for t, w in taps:
                same = same and compatible(zl, nearest)
            if not same:
                out[(tx, ty)] = (col, cola, 0.0, [0, 0])
                continue
            cur = clean(col, 1.0)[0]
            anchor_id = c["owners"][anchor] & 0x7FFFFFFF
            fresh = owner_set(anchor_id)
            for i in range(4):
                if support[i]:
                    fresh = with_id(fresh, support[i])
            fresh["ok"] = fresh["ok"] and sok
            provable = fresh["ok"]
            res = [cur, 1.0, nearest, encode(fresh)]
            m = c["motion"][anchor]
            if c["history"] < 0.5 or m[3] < 0.5:
                out[(tx, ty)] = tuple(res)
                continue
            expected = lin(m[2])
            old = tuple(madd(o[i], f(u[i] + m[i]), o[i + 2], madd_kind) for i in (0, 1))
            if not (old[0] >= o[0] and old[1] >= o[1] and old[0] < f(o[0] + o[2]) and old[1] < f(o[1] + o[3])):
                out[(tx, ty)] = tuple(res)
                continue
            flags, own = header[1], (tx, ty)
            near_p = abs(f(old[0] - p[0])) < 1.0 / 256.0 and abs(f(old[1] - p[1])) < 1.0 / 256.0
            if flags & 1 and not flags & 2 and near_p and provable and not schanged and lo <= tx <= hi and lo <= ty <= hi:
                v, a = c["old"][own]
                r = stored(c["records"][own])
                if admitted(v, a) and a >= 1 and compatible(c["old_depth"][own], expected) and r["ok"] \
                        and unchanged(r, sc, cc):
                    res[0] = clean(mix(cur, clean(v, a)[0], c["options"][0], form), 1.0)[0]
                    res[3] = encode(with_record(fresh, r))
                    out[(tx, ty)] = tuple(res)
                    continue
            oq = (f(old[0] - 0.5), f(old[1] - 0.5))
            ob = (math.floor(oq[0]), math.floor(oq[1]))
            of = (f(oq[0] - ob[0]), f(oq[1] - ob[1]))
            hist, mass, joined = 0.0, 0.0, fresh
            for y in (0, 1):
                for x in (0, 1):
                    w = tap_weight(of, x, y)
                    t = (ob[0] + x, ob[1] + y)
                    if not w > 0 or not (lo <= t[0] <= hi and lo <= t[1] <= hi):
                        continue
                    v, a = c["old"][t]
                    if not admitted(v, a) or a < 1 or not compatible(c["old_depth"][t], expected):
                        continue
                    r = stored(c["records"][t])
                    if not provable or not r["ok"] or r["a"] != anchor_id or not admissible(r, support, sc, cc):
                        continue
                    hist, mass, joined = f(hist + f(clean(v, a)[0] * w)), f(mass + w), with_record(joined, r)
            if not mass > 0:
                out[(tx, ty)] = tuple(res)
                continue
            h = clean(f(hist / mass), 1.0)[0]
            lo_c = hi_c = cur
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    t = clampt((anchor[0] + dx, anchor[1] + dy))
                    v, a = c["current"][t]
                    if admitted(v, a) and a >= 1 and compatible(zl, nearest):
                        lo_c, hi_c = min(lo_c, clean(v, a)[0]), max(hi_c, clean(v, a)[0])
                        joined = with_id(joined, c["owners"][t] & 0x7FFFFFFF)
            reaction = min(max(f(abs(f(h - cur)) / c["options"][3]), 0.0), 1.0)
            weight = f(c["options"][0] * f(1.0 - f(c["reactive"] * reaction)))
            res[0] = clean(mix(cur, min(max(h, lo_c), hi_c), weight, form), 1.0)[0]
            res[3] = encode(joined)
            out[(tx, ty)] = tuple(res)
    return out, weights


# --------------------------------------------------------- the test's checks
def canonical(rec):
    x, y = rec
    if x == 0:
        return y == 0
    a = x & 0x1FFFF
    if not x & PRESENT or x & ~(PRESENT | 0x1FFFF) & 0xFFFFFFFF or not 1 <= a <= 100000:
        return False
    return bool(y & (1 << (a % 31))) if y & PRESENT else y == 0 or (1 <= y <= 100000 and y != a)


def run_test(case, out, store, coupled=False):
    """The probe and texel assertions of the GPU test on f16-stored colours.
    coupled: the proposed provenance-coupled CENTER record for case 3."""
    c = fixture(case)
    sto = f16_rn if store == "rn" else f16_rz
    read = lambda t: (sto(out[t][0]), sto(out[t][1]), out[t][2], out[t][3])
    for (x, y), _ in sorted(out.items()):
        v, a, d, r = read((x, y))
        if not (math.isfinite(v) and v >= 0 and 0 <= a <= 1 and 0 <= d <= 100 and canonical(r)):
            return {"pass": False, "failure": "texel (%d,%d)" % (x, y)}
        if (x in (0, N - 1) or y in (0, N - 1) or case == 12) and (v, a, d, r) != (0.0, 0.0, 0.0, [0, 0]):
            return {"pass": False, "failure": "clear/outside (%d,%d)" % (x, y)}
    probes = c["probes"]
    for pixel, color, depth, record in probes:
        v, a, d, r = read(pixel)
        if coupled and case == 3 and pixel == CENTER and v > 0:
            record = [PRESENT | 5, 7]
        if abs(v - color[0]) > 0.001 or abs(a - color[1]) > 0.001:
            return {"pass": False, "failure": "probe %s color" % (list(pixel),), "actual": [v, a]}
        if abs(d - depth) > 0.0001:
            return {"pass": False, "failure": "probe %s depth" % (list(pixel),), "actual": d}
        if r != record:
            return {"pass": False, "failure": "probe %s record" % (list(pixel),), "actual": r}
    return {"pass": True}


def marker(case, out, store):
    sto = f16_rn if store == "rn" else f16_rz
    v, a, d, r = out[CENTER]
    rgb = rust_f32(sto(v))
    return "OWNER_HISTORY case=%d center=([%s, %s, %s, %s], %s, [%d, %d]) probes=%d baseline=%s" % (
        case, rgb, rgb, rgb, rust_f32(sto(a)), rust_f32(d), r[0], r[1], len(fixture(case)["probes"]),
        "true" if fixture(case)["baseline"] else "false")


# ----------------------------------------------------------------- analyses
def raw_markers():
    with open("/Users/andreyyurlov/.codex/incant-evidence/0073-owner-history-gpu-001/wgsl/stdout.txt") as fh:
        lines = [x.split(" ... ")[-1] for x in fh.read().splitlines()]
    return [x for x in lines if x.startswith("OWNER_HISTORY case=")]


def calibration():
    """Which arithmetic/storage hypotheses reproduce the printed GPU bytes exactly?"""
    raw = raw_markers()
    panic = "[0.40478516, 0.40478516, 0.40478516, 1.0]"
    rows = {}
    for arith in ARITH:
        for table in ("correct", "zero_table"):
            for store in ("rn", "rz"):
                for form in ("metal_sep", "metal_fma", "wgsl"):
                    o1, o2, o3 = (resolve(fixture(k), arith, table, form=form)[0] for k in (1, 2, 3))
                    ok12 = [marker(1, o1, store), marker(2, o2, store)] == raw
                    sto = f16_rn if store == "rn" else f16_rz
                    v3 = sto(o3[CENTER][0])
                    got = "[%s, %s, %s, %s]" % (rust_f32(v3), rust_f32(v3), rust_f32(v3), rust_f32(sto(o3[CENTER][1])))
                    rows["%s|%s|%s|%s" % (arith, table, store, form)] = {
                        "cases_1_2_markers_exact": ok12, "case3_center": got, "case3_reproduced": got == panic,
                        "case3_record_unprinted": o3[CENTER][3]}
    reproducing = sorted(k for k, v in rows.items() if v["cases_1_2_markers_exact"] and v["case3_reproduced"])
    return rows, reproducing, raw


def arithmetic():
    rows = {}
    for arith in ARITH:
        div_kind, madd_kind = ARITH[arith]
        nonzero = []
        for tx in range(1, 15):
            u = divide(f(tx + 0.5 - 1.0), 14.0, div_kind)
            qq = f(madd(1.0, u, 14.0, madd_kind) - 0.5)
            fr = f(qq - math.floor(qq))
            if fr != 0:
                nonzero.append([tx, math.floor(qq), fr])
        out, weights = resolve(fixture(3), arith)
        w98 = [w for t, w in weights[CENTER] if t == (9, 8)]
        rows[arith] = {"texels_with_nonzero_fraction": nonzero,
                       "center_tap_9_8_weight": w98[0] if w98 else 0.0,
                       "center_color_f32": out[CENTER][0], "center_record": out[CENTER][3]}
    return rows


def mechanisms():
    """CENTER, far [3,3] and owner-7 texel [9,8] for each mechanism, current and patched."""
    rows = {}
    cases = [("exact", "ieee", "correct", "correct"), ("H1 arithmetic", "recip+fma", "correct", "correct")]
    cases += [("T " + t, "ieee", t, "correct") for t in ("zero_table", "component_reversed", "next_vec4",
                                                           "bit_reversed", "header_aliased")]
    cases += [("I " + i, a, "correct", i) for i in ("dropped", "slot0", "reversed") for a in ("ieee", "recip+fma")]
    for name, arith, table, index in cases:
        row = {}
        for patched in (False, True):
            out = resolve(fixture(3), arith, table, index, patched)[0]
            row["patched" if patched else "current"] = {
                str(list(t)): [f16_rz(out[t][0]), out[t][3]] for t in (CENTER, (3, 3), (9, 8))}
        key = name if name.startswith(("exact", "H1", "T ")) else "%s (%s)" % (name, arith)
        rows[key] = row
    return rows


def suite(arith, table="correct", patched=False, coupled=False):
    res = {}
    for case in range(1, 13):
        out = resolve(fixture(case), arith, table, patched=patched)[0]
        res[case] = run_test(case, out, "rz", coupled)
    return res


def controls():
    ok, rows = True, {}
    # The patch changes no output of any of the 12 fixtures under exact arithmetic.
    same_exact = all(resolve(fixture(k), "ieee")[0] == resolve(fixture(k), "ieee", patched=True)[0]
                     for k in range(1, 13))
    rows["patch_identical_on_all_12_fixtures_exact_arithmetic"] = same_exact
    # Oracle validation: the current shader passes the unchanged test exactly.
    rows["current_exact_suite"] = suite("ieee")
    # The original case-3 expectation stays a failed counterexample under H1.
    rows["H1_current_suite"] = suite("recip+fma")
    rows["H1_patched_original_test"] = suite("recip+fma", patched=True)
    rows["H1_patched_coupled_record"] = suite("recip+fma", patched=True, coupled=True)
    # A changed-table misread is not repaired by the patch: discrimination.
    rows["T_zero_table_current"] = suite("ieee", "zero_table")
    rows["T_zero_table_patched_coupled"] = suite("ieee", "zero_table", patched=True, coupled=True)
    # Sensitivity: deleting the changed-owner rule fails case 3 even with exact arithmetic.
    mutant = resolve(fixture(3), "ieee", drop_rule=True)[0]
    rows["mutant_without_changed_rule_center"] = f16_rz(mutant[CENTER][0])

    def variant(**k):
        c = fixture(3)
        c.update(k)
        return c

    def center(c, patched):
        out = resolve(c, "ieee", patched=patched)[0]
        return [f16_rz(out[CENTER][0]), out[CENTER][3]]

    # Large support: a half-texel source offset gives owner 7 weight 0.5 at CENTER.
    large = variant(source=(1.5, 1.0, 14.0, 14.0))
    rows["large_support_changed_7"] = {"current": center(large, False), "patched": center(large, True)}
    # Moving anchor: owner 5 changed, records {5}; both rules keep reprojection.
    moving = variant(records={t: [PRESENT | 5, 0] for t in fixture(3)["records"]})
    moving["header"] = [classes([5]), 1, 100000, 0]
    moving["table"] = {0: 1 << 5}
    rows["changed_anchor_reprojects"] = {"current": center(moving, False), "patched": center(moving, True)}
    # Unchanged second owner: nothing changed, camera not static; both admit.
    calm = variant()
    calm["header"], calm["table"] = [0, 0, 100000, 0], {}
    rows["unchanged_second_owner"] = {"current": center(calm, False), "patched": center(calm, True)}
    lc, lp = rows["large_support_changed_7"]["current"], rows["large_support_changed_7"]["patched"]
    mc, mp = rows["changed_anchor_reprojects"]["current"], rows["changed_anchor_reprojects"]["patched"]
    cc_, cp = rows["unchanged_second_owner"]["current"], rows["unchanged_second_owner"]["patched"]
    exact, h1c = rows["current_exact_suite"], rows["H1_current_suite"]
    h1p, h1k = rows["H1_patched_original_test"], rows["H1_patched_coupled_record"]
    checks = {
        "patch_identical_exact": same_exact,
        "oracle_passes_unchanged_test_exact": all(v["pass"] for v in exact.values()),
        "H1_current_fails_only_case3_center_color": not h1c[3]["pass"]
            and h1c[3]["failure"] == "probe [8, 8] color" and all(v["pass"] for k, v in h1c.items() if k != 3),
        "H1_patched_original_fails_only_case3_center_record": not h1p[3]["pass"]
            and h1p[3]["failure"] == "probe [8, 8] record" and all(v["pass"] for k, v in h1p.items() if k != 3),
        "H1_patched_coupled_all_pass": all(v["pass"] for v in h1k.values()),
        "T_not_repaired_by_patch": not rows["T_zero_table_patched_coupled"][3]["pass"],
        "mutant_detected": rows["mutant_without_changed_rule_center"] > 0.4,
        "large_support_current_carries_stale": lc[0] > 0.4 and lp[0] < 0.26 and lp[1] == [PRESENT | 5, 7],
        "changed_anchor_kept_by_both": mc == mp and mc[0] > 0.4,
        "unchanged_second_owner_same": cc_ == cp,
    }
    rows["checks"] = checks
    return all(checks.values()), rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    pinned()
    cal, reproducing, raw = calibration()
    arith = arithmetic()
    mech = mechanisms()
    ok, ctl = controls()
    binding = patch_binding()
    checks = {
        "patch_text_is_the_oracle_rule": all(v for k, v in binding.items() if not k.endswith("sha256")),
        "raw_markers_read": raw[:1] == [x for x in raw if x.startswith("OWNER_HISTORY case=1 ")],
        "exact_arithmetic_center_zero": arith["ieee"]["center_tap_9_8_weight"] == 0.0
            and arith["ieee"]["center_color_f32"] == 0.0,
        "some_hypothesis_reproduces_all_printed_values": bool(reproducing),
        "controls": ok,
    }
    text = json.dumps({"model": "0073 owner-history case-3 diagnosis oracle", "python": sys.version.split()[0],
                       "pins": PINS, "checks": checks, "reproducing_hypotheses": reproducing,
                       "raw_case_markers": raw, "calibration": cal, "arithmetic": arith,
                       "mechanisms": mech, "controls": ctl, "patch": binding}, indent=1, sort_keys=True,
                      default=str)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(json.dumps({"checks": checks, "reproducing_hypotheses": reproducing}, indent=1))
    return 0 if all(checks.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
