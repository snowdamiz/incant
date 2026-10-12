"""Replay the 1x UNORM panel's blend trace under candidate backend arithmetic.

Input: the trace written by tools/diag-trace.patch (diag_0059_unorm_trace):
background codes, every blend in draw order (pixel, colour, coverage interval)
and the Rust Exact, Llvmpipe and Portable canvases. The replay recomputes the
float-only reference of checkpoint 12 and the corrected one with identical f64
operations, checks them against the Rust dump, then reports for each candidate
arithmetic the statistics the GPU test prints. Checkpoint 12 printed, for both
llvmpipe (Linux) and WARP (Windows): outside 245 by 2, vs exact worst 2,
>1: 5, changed 4248.

Usage: python3 -I -B unorm_fingerprint.py <unorm-1x-trace.txt>
"""

import math
import struct
import sys

ENCODE = 0.67
CONVERT = 0.6
PRODUCT = 128 / 255
CI = {"outside": 245, "excess": 2, "worst": 2, "over": 5, "changed": 4248}


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


def load(path):
    bg, blends, canvases = [], [], {"exact": [], "llvmpipe": [], "portable": []}
    for line in open(path):
        f = line.split()
        if f[0] == "bg":
            bg.append(tuple(int(v) for v in f[2:5]))
        elif f[0] == "blend":
            x, y, r, g, b, a = (int(v) for v in f[1:7])
            blends.append((x, y, (r, g, b, a), (float(f[7]), float(f[8]))))
        elif f[0] in canvases:
            v = [float(t) for t in f[2:8]]
            canvases[f[0]].append(((v[0], v[1]), (v[2], v[3]), (v[4], v[5])))
        elif f[0] == "size":
            size = (int(f[1]), int(f[2]))
    return size, bg, blends, canvases


def float_interval(code, alpha, low, high, write):
    """Ui reference float blend (Canvas::over, UNORM), same f64 operations."""
    src = code / 255.0
    dest = (low / 255.0, high / 255.0)
    vals = [src * a + d * (1.0 - a) for a in alpha for d in dest]
    lo, hi = min(vals), max(vals)
    return (max(math.ceil(lo * 255.0 - write), 0.0), min(math.floor(hi * 255.0 + write), 255.0))


def fixed_interval(code, alpha, low, high):
    """fixed_point() in ui_draw_tests.rs, same f64 operations."""
    src = code / 255.0

    def codes(lh):
        return (max(math.ceil(lh[0] * 255.0 - CONVERT), 0.0), min(math.floor(lh[1] * 255.0 + CONVERT), 255.0))

    s = codes((src * alpha[0], src * alpha[1]))
    a = codes(alpha)
    product = (low * (255.0 - a[1]) / 255.0, high * (255.0 - a[0]) / 255.0)
    return (
        min(s[0] + max(math.ceil(product[0] - PRODUCT), 0.0), 255.0),
        min(s[1] + math.floor(product[1] + PRODUCT), 255.0),
    )


def reference(size, bg, blends, corrected):
    px = [[(float(c), float(c)) for c in p] for p in bg]
    for x, y, color, cov in blends:
        alpha = tuple(color[3] / 255.0 * c for c in cov)
        if alpha == (0.0, 0.0):
            continue
        p = px[y * size[0] + x]
        for c in range(3):
            low, high = p[c]
            interval = float_interval(color[c], alpha, low, high, ENCODE)
            if corrected:
                fixed = fixed_interval(color[c], alpha, low, high)
                interval = (min(interval[0], fixed[0]), max(interval[1], fixed[1]))
            p[c] = interval
    return px


# Candidate arithmetic: (shader colour code, opacity code, coverage, destination code) -> stored code.
def shader(color, opacity, coverage):
    """The UI shader's f32 premultiplied output; UNORM8 attributes scale by 1/255."""
    unit = lambda code: f32(code * f32(1 / 255))
    alpha = f32(unit(opacity) * f32(coverage))
    return f32(unit(color) * alpha), alpha


def fixed(convert, product):
    def blend(color, opacity, coverage, d):
        source, alpha = shader(color, opacity, coverage)
        s, a = convert(source), convert(alpha)
        return min(s + product(d, 255 - a), 255)

    return blend


def nearest_even(x):
    return int(round(f32(min(x, 1.0) * 255.0)))  # Python rounds half to even


def half_up(x):  # D3D11.3 3.2.3.6: c * 255 + 0.5, drop the fraction
    return int(math.floor(f32(min(x, 1.0) * 255.0) + 0.5))


def truncate(x):
    return int(math.floor(f32(min(x, 1.0) * 255.0)))


blinn = lambda d, k: (d * k + ((d * k) >> 8) + 128) >> 8  # lp_build_mul_norm
exact_product = lambda d, k: (2 * d * k + 255) // 510  # round(t / 255); no ties
truncated_product = lambda d, k: d * k // 255
alpha_plus_one = lambda d, k: (d * (k + 1)) >> 8
geometric = lambda d, k: (d * k + ((d * k) >> 8)) >> 8


def float_blend(store):
    def blend(color, opacity, coverage, d):
        source, alpha = shader(color, opacity, coverage)
        return store(source * 255.0 + d * (1.0 - alpha))

    return blend


CANDIDATES = [
    ("llvmpipe: cvtps2dq, Blinn product", fixed(nearest_even, blinn)),
    ("8-bit: nearest-even, exact product", fixed(nearest_even, exact_product)),
    ("8-bit: D3D half-up, exact product", fixed(half_up, exact_product)),
    ("8-bit: truncated conversions and product", fixed(truncate, truncated_product)),
    ("8-bit: nearest-even, a*(b+1)>>8 product", fixed(nearest_even, alpha_plus_one)),
    ("8-bit: nearest-even, (t+(t>>8))>>8 product", fixed(nearest_even, geometric)),
    ("float blend, nearest store", float_blend(lambda v: min(max(round(v), 0), 255))),
    ("float blend, truncated store", float_blend(lambda v: min(max(math.floor(v), 0), 255))),
]


def render(size, bg, blends, blend):
    px = [list(p) for p in bg]
    for x, y, color, cov in blends:
        if color[3] / 255.0 * cov[0] == 0.0:  # discarded
            continue
        p = px[y * size[0] + x]
        for c in range(3):
            p[c] = blend(color[c], color[3], cov[0], p[c])
    return px


def stats(image, bg, old, exact):
    outside = excess = worst = over = changed = 0
    for i, p in enumerate(image):
        changed += p != list(bg[i])
        for c in range(3):
            lo, hi = old[i][c]
            d = max(lo - p[c], p[c] - hi)
            if d > 0:
                outside += 1
                excess = max(excess, d)
            e = abs(p[c] - exact[i][c])
            worst = max(worst, e)
            over += e > 1
    return {"outside": outside, "excess": int(excess), "worst": worst, "over": over, "changed": changed}


def admitted(image, ref):
    return sum(1 for i, p in enumerate(image) for c in range(3) if not ref[i][c][0] <= p[c] <= ref[i][c][1])


def main(path):
    size, bg, blends, canvases = load(path)
    old = reference(size, bg, blends, corrected=False)
    new = reference(size, bg, blends, corrected=True)
    assert new == [list(p) for p in canvases["portable"]], "corrected reference differs from Rust"
    exact = [[int(p[c][0]) for c in range(3)] for p in canvases["exact"]]
    rust_llvmpipe = [[int(p[c][0]) for c in range(3)] for p in canvases["llvmpipe"]]
    width = lambda ref: max(h - l for p in ref for l, h in p)
    wide = lambda ref: sum(1 for p in ref for l, h in p if h - l > 2)
    changed = sum(1 for a, b in zip(old, new) for c in range(3) if a[c] != b[c])
    print(f"trace: {len(blends)} blends over {size[0]}x{size[1]}; Rust corrected reference reproduced exactly")
    print(f"float-only reference (checkpoint 12): widest {width(old):g} ({wide(old)} channels > 2)")
    print(f"corrected reference: widest {width(new):g} ({wide(new)} channels > 2); {changed} of {3 * len(bg)} channels gain codes")
    print(f"CI llvmpipe and WARP printed: {CI}")
    for name, blend in CANDIDATES:
        image = render(size, bg, blends, blend)
        s = stats(image, bg, old, exact)
        match = "MATCHES CI" if s == CI else "differs"
        print(f"- {name}: {s} {match}; corrected reference outside {admitted(image, new)}")
        if name.startswith("llvmpipe"):
            assert image == rust_llvmpipe, "Python llvmpipe replay differs from the Rust emulation"
            llvmpipe = image
    # Samples: every channel the float-only reference refused by 2 codes, and
    # the per-blend arithmetic that produced it.
    print("\nsamples refused by the float-only reference by 2 codes (llvmpipe replay):")
    layers = {}
    for x, y, color, cov in blends:
        layers.setdefault(y * size[0] + x, []).append((color, cov))
    shown = 0
    for i, p in enumerate(llvmpipe):
        for c in range(3):
            lo, hi = old[i][c]
            if max(lo - p[c], p[c] - hi) < 2:
                continue
            x, y = i % size[0], i // size[0]
            print(f"  pixel ({x},{y}) channel {'RGB'[c]}: background {bg[i][c]}, exact {exact[i][c]}, "
                  f"llvmpipe {p[c]}, float-only [{lo:g},{hi:g}], corrected [{new[i][c][0]:g},{new[i][c][1]:g}]")
            d = bg[i][c]
            ed = float(d)
            for color, cov in layers[i]:
                source, alpha = shader(color[c], color[3], cov[0])
                s, a = nearest_even(source), nearest_even(alpha)
                k = 255 - a
                out = min(s + blinn(d, k), 255)
                ed = color[c] / 255.0 * (color[3] / 255.0 * cov[0]) * 255.0 + ed * (1.0 - color[3] / 255.0 * cov[0])
                print(f"    over colour {color[c]} opacity {color[3]} coverage {cov[0]:.6f}: "
                      f"S {source * 255:.4f} -> {s}, A {alpha * 255:.4f} -> {a}, "
                      f"d {d}: P {d * k / 255:.4f} -> {blinn(d, k)}; stored {out} (exact float chain {ed:.4f})")
                d = out
            shown += 1
    print(f"  ({shown} channels)")
    # Per-blend distance of the llvmpipe replay from the float blend of the
    # same destination code, against the stated bound.
    worst = 0.0
    px = [list(p) for p in bg]
    for x, y, color, cov in blends:
        if color[3] / 255.0 * cov[0] == 0.0:
            continue
        p = px[y * size[0] + x]
        for c in range(3):
            a = color[3] / 255.0 * cov[0]
            e = color[c] * a + p[c] * (1.0 - a)
            p[c] = CANDIDATES[0][1](color[c], color[3], cov[0], p[c])
            worst = max(worst, abs(p[c] - e))
    print(f"\nllvmpipe replay, worst single-blend distance from the float blend: {worst:.4f} codes "
          f"(bound {CONVERT} + {CONVERT}*d/255 + {PRODUCT:.4f} <= {2 * CONVERT + PRODUCT:.4f})")


if __name__ == "__main__":
    main(sys.argv[1])
