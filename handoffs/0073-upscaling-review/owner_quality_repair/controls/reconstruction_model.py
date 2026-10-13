#!/usr/bin/env python3
"""0073 owner_quality_repair: independent CPU control of the upscaling resolve.

Standard library only, deterministic, no evidence or renderer access:

    python3 -I -B handoffs/0073-upscaling-review/owner_quality_repair/controls/reconstruction_model.py \
        --out handoffs/0073-upscaling-review/owner_quality_repair/controls/reconstruction_model.json

A small periodic 2-D scalar mirror of fd5b86d0 upscaling/{common,resolve,
resolve_owner}.wgsl and of the repair in ../src.
- Each internal texel point-samples the scene at its true jittered position
  (temporal/jitter.rs SAMPLES; retained.rs ndc).
- Textures use a box prefilter of one internal pixel, approximating trilinear
  LOD from internal derivatives. FULL uses one output pixel; DRS is unjittered
  nearest expansion.
- Static history reprojects to the pixel itself. Pans move the content by
  `velocity` output px per frame, as the fixture's camera pan does.

Variants:
- current_up: resolve.wgsl. 2x2 bilinear current, bilinear history, 3x3
  internal box clamp, reactive term, history fraction h_r = h(1 - s*r).
- current_own: resolve_owner.wgsl. Static frames take the unclipped persistence
  branch mix(current, own history, h); otherwise as current_up.
- repair_up: ../src resolve.wgsl.
  - R1: a 4x4 Catmull-Rom current reconstruction clamped to its central 2x2.
  - R2: 4x4 Catmull-Rom history resampling clamped to its central 2x2.
  - R3: history fraction min(h_r, W_r / (W_r + 1)), with W the host frame mass
    (0 after a cut, +1 per frame up to h / (1 - h), times (old/new width)^2
    after an internal-scale increase).
- repair_own: ../src resolve_owner.wgsl. As repair_up, plus static persistence
  at min(h, W / (W + 1)), extended to mixed and clear pixels with an owner
  record (R4, thin lines only).
- oracle_mass: reference only. The design-only confidence oracle
  (kernel_oracle.rs Gaussian evidence; confidence_oracle.rs mass capped at
  h / (1 - h) * min(ratio^2, 4); fraction W / (W + m)). It can exceed
  history_weight and is not proposed.

Units are linear scene values (texture 0.05..0.85, line coverage 0..1).
Contrast is the standard deviation over all output pixels; activity is the
mean per-pixel range over the last eight frames. Nothing here is a rendered
image or a GPU result.
"""
import argparse
import json
import math

SAMPLES = [(-0.375, -0.125), (0.125, 0.375), (0.375, -0.375), (-0.125, 0.125),
           (0.375, 0.125), (-0.125, -0.375), (-0.375, 0.375), (0.125, -0.125)]
# Jitter-lattice control only: eight distinct x and y offsets (odd sixteenths),
# zero mean, one point per row and column of an 8x8 grid.
LATIN = [((2 * i + 1) / 16 - 0.5, (2 * j + 1) / 16 - 0.5)
         for i, j in [(0, 5), (4, 1), (2, 7), (6, 3), (1, 0), (5, 4), (3, 2), (7, 6)]]
H, THRESHOLD, STRENGTH = 0.9, 4.0, 0.5            # review policy REVIEW
N = 32                                              # output pixels per side (periodic)
LO, HI = 0.05, 0.85
CELLS = {"cell_5_33px": 16 / 3, "cell_4px": 4.0, "cell_2_29px": 16 / 7}
OFFSET = (0.37, 0.21)                               # keeps texels off the internal grid


class Grid:
    def __init__(self, n16, jitter=SAMPLES):
        assert N * n16 % 16 == 0
        self.n16, self.ni, self.ratio, self.jitter = n16, N * n16 // 16, 16 / n16, jitter
        self.rho = 0.35 + 0.325 * max(self.ratio - 2.0, 0.0)          # kernel_oracle.rs
        self.norm = self.ratio ** 2 / (2 * math.pi * self.rho ** 2)

    def position(self, t, j):
        """Output-pixel coordinate of internal texel t's sample (kernel_oracle position)."""
        return (t + 0.5 - j) * self.ratio


def square_integral(x, cell):
    r = x % (2 * cell)
    return r if r <= cell else 2 * cell - r


def square_box(x, cell, width):
    return (square_integral(x + width / 2, cell) - square_integral(x - width / 2, cell)) / width


def checker(cell, shade=1.0, shift=0.0):
    mid, amp = (HI + LO) / 2, (HI - LO) / 2
    return lambda x, y, width: shade * (mid + amp * square_box(x - shift + OFFSET[0], cell, width)
                                         * square_box(y + OFFSET[1], cell, width))


def internal_image(grid, phase, scene):
    jx, jy = grid.jitter[phase % 8]
    return [[scene(grid.position(tx, jx) % N, grid.position(ty, jy) % N, grid.ratio)
             for tx in range(grid.ni)] for ty in range(grid.ni)]


def catmull_rom(f):
    return [-0.5 * f ** 3 + f * f - 0.5 * f, 1.5 * f ** 3 - 2.5 * f * f + 1,
            -1.5 * f ** 3 + 2 * f * f + 0.5 * f, 0.5 * f ** 3 - 0.5 * f * f]


def bilinear_weights(fx, fy):
    return [(1 - fx) * (1 - fy), fx * (1 - fy), (1 - fx) * fy, fx * fy]


def reconstruct(img, size, qx, qy, kind):
    """Value at continuous texel coordinate (qx, qy) = position - 0.5 (common.wgsl)."""
    bx, by = math.floor(qx), math.floor(qy)
    fx, fy = qx - bx, qy - by
    near = [img[(by + j) % size][(bx + i) % size] for j in (0, 1) for i in (0, 1)]
    if kind == "bilinear":
        return sum(w * v for w, v in zip(bilinear_weights(fx, fy), near))
    wx, wy = catmull_rom(fx), catmull_rom(fy)
    v = sum(wy[j] * wx[i] * img[(by - 1 + j) % size][(bx - 1 + i) % size] for j in range(4) for i in range(4))
    return min(max(v, min(near)), max(near))                   # anti-ringing clamp (R1, R2)


def gaussian_evidence(img, grid, phase, px, py):
    jx, jy = grid.jitter[phase % 8]
    sx, sy = (px + 0.5) / grid.ratio + jx, (py + 0.5) / grid.ratio + jy
    cx, cy = math.floor(sx), math.floor(sy)
    s = m = 0.0
    for j in (-1, 0, 1):
        for i in (-1, 0, 1):
            dx = px + 0.5 - grid.position(cx + i, jx)
            dy = py + 0.5 - grid.position(cy + j, jy)
            w = grid.norm * math.exp(-(dx * dx + dy * dy) / (2 * grid.rho ** 2))
            s += w * img[(cy + j) % grid.ni][(cx + i) % grid.ni]
            m += w
    return s, m


def frame_mass(mass, has_previous, prev_width, width, cap):
    """Proposed owner.rs State::frame_mass, repeated here independently."""
    if not has_previous:
        return 0.0
    w = min(mass, cap)
    if prev_width is not None and width > prev_width:
        w *= (prev_width / width) ** 2
    return w


def run(variant, scene_at, schedule, velocity=0.0, caster_change=(), cut=(), local_reset=None):
    """Static-camera frames (or a constant pan) through one variant; returns outputs."""
    out, frames, phase, prev_width, mass = None, [], 0, None, 0.0
    cap = H / (1 - H)
    for k, n16 in enumerate(schedule, start=1):
        grid = Grid(n16)
        has_previous = out is not None and k not in cut
        phase = phase + 1 if has_previous else 0
        oracle = variant == "oracle_mass"
        mass_cap = cap * min(grid.ratio ** 2, 4.0) if oracle else cap
        w_frame = frame_mass(mass, has_previous, prev_width, grid.ni, mass_cap)
        camera_static = has_previous and prev_width == grid.ni and velocity == 0.0 and k not in caster_change
        img = internal_image(grid, phase, scene_at(k))
        jx, jy = grid.jitter[phase % 8]
        new = [[0.0] * N for _ in range(N)]
        repair = variant.startswith("repair")
        kind = "catmull_rom" if repair else "bilinear"
        for py in range(N):
            for px in range(N):
                qx, qy = (px + 0.5) / grid.ratio + jx - 0.5, (py + 0.5) / grid.ratio + jy - 0.5
                current = reconstruct(img, grid.ni, qx, qy, kind)
                if oracle:
                    s, m = gaussian_evidence(img, grid, phase, px, py)
                    current = s / m
                reset_here = local_reset is not None and k == local_reset[0] and local_reset[1](px, py)
                if not has_previous or reset_here:
                    new[py][px] = current
                    continue
                if variant in ("current_own", "repair_own") and camera_static:
                    fraction = H if variant == "current_own" else min(H, w_frame / (w_frame + 1))
                    new[py][px] = current + (out[py][px] - current) * fraction
                    continue
                history = reconstruct(out, N, px + 0.5 - velocity - 0.5, py, kind)
                ax, ay = math.floor(qx) % grid.ni, math.floor(qy) % grid.ni
                box = [img[(ay + j) % grid.ni][(ax + i) % grid.ni] for j in (-1, 0, 1) for i in (-1, 0, 1)]
                lo, hi = min(box + [current]), max(box + [current])
                reaction = min(abs(history - current) / THRESHOLD, 1.0)
                h_r = H * (1 - STRENGTH * reaction)
                w_r = w_frame * (1 - STRENGTH * reaction)
                clamped = min(max(history, lo), hi)
                if oracle:
                    fraction = w_r / (w_r + m)
                elif repair:
                    fraction = min(h_r, w_r / (w_r + 1))
                else:
                    fraction = h_r
                new[py][px] = current + (clamped - current) * fraction
        out, prev_width = new, grid.ni
        mass = min(w_frame + (m_mean(grid) if oracle else 1.0), mass_cap)
        frames.append(out)
    return frames


def m_mean(grid):
    return 1.0           # the kernel's cycle-mean evidence mass (kernel_tests cycle_mass)


def contrast(img):
    v = [x for row in img for x in row]
    mean = sum(v) / len(v)
    return math.sqrt(sum((x - mean) ** 2 for x in v) / len(v))


def activity(frames):
    return sum(max(f[y][x] for f in frames) - min(f[y][x] for f in frames) for y in range(N) for x in range(N)) / N ** 2


def mean_abs(a, b):
    return sum(abs(a[y][x] - b[y][x]) for y in range(N) for x in range(N)) / N ** 2


def full_image(scene):
    return [[scene(x + 0.5, y + 0.5, 1.0) for x in range(N)] for y in range(N)]


def drs_image(scene, n16=8):
    g = Grid(n16)
    img = [[scene(g.position(tx, 0.0), g.position(ty, 0.0), g.ratio) for tx in range(g.ni)] for ty in range(g.ni)]
    return [[img[int((y + 0.5) / g.ratio)][int((x + 0.5) / g.ratio)] for x in range(N)] for y in range(N)]


VARIANTS = ["current_up", "current_own", "repair_up", "repair_own", "oracle_mass"]


def r(v, n=4):
    return round(v, n)


def steady_detail():
    rows = {}
    for name, cell in CELLS.items():
        scene = checker(cell)
        full = full_image(scene)
        drs = drs_image(scene)
        row = {"full": {"contrast": r(contrast(full))},
               "drs50": {"contrast": r(contrast(drs)), "error_vs_full": r(mean_abs(drs, full))}}
        for v in VARIANTS:
            f = run(v, lambda k: scene, [8] * 24)
            row[v] = {"contrast_frame24": r(contrast(f[23])), "error_vs_full_frame24": r(mean_abs(f[23], full)),
                      "last_cycle_activity": r(activity(f[16:24]))}
        rows[name] = row
    return rows


def pans():
    rows = {}
    for name, cell in CELLS.items():
        for velocity in (0.37, 2.73):
            row = {}
            for v in ("current_up", "repair_up"):
                f = run(v, lambda k: checker(cell, shift=velocity * (k - 1)), [8] * 24, velocity=velocity)
                last = list(range(16, 24))
                c = sum(contrast(f[k]) for k in last) / 8
                e = sum(mean_abs(f[k], full_image(checker(cell, shift=velocity * k))) for k in last) / 8
                row[v] = {"contrast_last_cycle": r(c), "error_vs_full_last_cycle": r(e)}
            rows[f"{name}_pan_{velocity}px_per_frame"] = row
    return rows


def transitions():
    schedule = [8] * 10 + [12] * 3 + [4] * 3 + [16] * 4 + [8] * 3
    rows = {}
    for name in ("cell_5_33px", "cell_2_29px"):
        scene = checker(CELLS[name])
        row = {}
        for v in VARIANTS:
            f = run(v, lambda k: scene, schedule)
            fixed = run(v, lambda k: scene, [16] * 24)
            reference = sum(contrast(x) for x in fixed[16:24]) / 8
            c = [contrast(x) for x in f]
            row[v] = {"contrast_frames_13_to_20": [r(x) for x in c[12:20]], "matched_cycle_reference_16": r(reference),
                      "ratio_third_16": r(c[18] / reference), "ratio_fourth_16": r(c[19] / reference),
                      "ratio_first_4_vs_last_12": r(c[13] / c[12])}
        rows[name] = row
    return rows


def stale_shading():
    """Shadow factor 0.25 for frames 1-16, lit from 17; the caster flag is set on frame 17 only."""
    cell = CELLS["cell_5_33px"]
    lit, shadowed = checker(cell, 1.0), checker(cell, 0.25)
    rows = {}
    for v in VARIANTS:
        lit_ref = run(v, lambda k: lit, [8] * 32)[-1]
        shadow_ref = run(v, lambda k: shadowed, [8] * 32)[-1]
        scale = mean_abs(shadow_ref, lit_ref)
        f = run(v, lambda k: shadowed if k <= 16 else lit, [8] * 40, caster_change=[17])
        rows[v] = {f"frame{k}": r(mean_abs(f[k - 1], lit_ref) / scale) for k in (17, 18, 20, 24, 32, 40)}
    return rows


def cut_and_local_reset():
    scene = checker(CELLS["cell_5_33px"])
    rows = {}
    for v in VARIANTS:
        reference = run(v, lambda k: scene, [8] * 48)[-1]
        cut = run(v, lambda k: scene, [8] * 48, cut=[17])
        local = run(v, lambda k: scene, [8] * 48, local_reset=(17, lambda x, y: x < N // 2))

        def strip(f):
            return sum(abs(f[y][x] - reference[y][x]) for y in range(N) for x in range(N // 2)) / (N * N // 2)
        rows[v] = {"cut_error": {f"frame{k}": r(mean_abs(cut[k - 1], reference)) for k in (17, 18, 20, 24, 32)},
                   "local_reset_strip_error": {f"frame{k}": r(strip(local[k - 1])) for k in (17, 18, 20, 24, 32)}}
    return rows


# ------------------------------------------------------------- thin lines over clear (R4)

def line_alpha(y, y0, thickness):
    return 1.0 if abs(y - y0) < thickness / 2 else 0.0


def thin_line(thickness, y0, jitter, persist, frames=24):
    """Premultiplied coverage of a horizontal line over clear at 8/16, static camera.

    Today (both variants) and repair_up: a pixel whose 2x2 taps mix line and clear
    exits current-only with the bilinear premultiplied estimate (resolve*.wgsl).
    repair_own (persist): a mixed exit keeps an owner record and a negative depth; on
    static frames a pixel with such a record persists, mix(bilinear current, own
    history, min(h, W / (W + 1))), including when all its current taps are clear.
    """
    grid = Grid(8, jitter)
    out, has_history, mass, cap = None, None, 0.0, H / (1 - H)
    presence = []
    for k in range(frames):
        jx, jy = jitter[k % 8]
        img = [[line_alpha(grid.position(ty, jy) % N, y0, thickness)] * grid.ni for ty in range(grid.ni)]
        w_frame = 0.0 if out is None else min(mass, cap)
        new = [[0.0] * N for _ in range(N)]
        hist = [[False] * N for _ in range(N)]
        for py in range(N):
            qy = (py + 0.5) / grid.ratio + jy - 0.5
            for px in range(N):
                qx = (px + 0.5) / grid.ratio + jx - 0.5
                current = reconstruct(img, grid.ni, qx, qy, "bilinear")
                by = math.floor(qy)
                fy = qy - by
                taps = [(img[by % grid.ni][0], 1 - fy), (img[(by + 1) % grid.ni][0], fy)]
                geometry = any(a > 0 and w > 0 for a, w in taps)
                if persist and out is not None and has_history[py][px]:
                    fraction = min(H, w_frame / (w_frame + 1))
                    new[py][px] = current + (out[py][px] - current) * fraction
                    hist[py][px] = True
                else:
                    new[py][px] = current
                    hist[py][px] = persist and geometry
        out, has_history = new, hist
        mass = min(w_frame + 1.0, cap)
        rows = [y for y in range(N) if abs(y + 0.5 - y0) <= 3.5]
        presence.append(sum(max(out[y][x] for y in rows) for x in range(N)) / N)
    return presence


def thin_lines():
    rows = {}
    for thickness in (1.6, 0.8, 0.4):
        for name, jitter in (("samples", SAMPLES), ("latin_alternative", LATIN)):
            today = thin_line(thickness, 16.5, jitter, persist=False)
            repair = thin_line(thickness, 16.5, jitter, persist=True)
            rows[f"{thickness}px_{name}"] = {
                "full_presence": 1.0,
                "current_up_own_and_repair_up_frames_17_24": [r(x, 3) for x in today[16:24]],
                "repair_own_static_frames_17_24": [r(x, 3) for x in repair[16:24]],
            }
    lattice = {}
    for name, jitter in (("samples", SAMPLES), ("latin_alternative", LATIN)):
        grid = Grid(8, jitter)
        missed = sum(not any(line_alpha(grid.position(ty, jitter[p][1]) % N, 16.0 + k * grid.ratio / 64, 0.4) > 0
                             for p in range(8) for ty in range(grid.ni)) for k in range(64))
        lattice[name] = {"distinct_y_offsets": len({j[1] for j in jitter}), "never_sampled_fraction_0p4px": r(missed / 64, 3)}
    rows["lattice_coverage_8_16"] = lattice
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    result = {
        "model": "0073 owner_quality_repair independent scalar resolve control (CPU only)",
        "policy": {"history_weight": H, "reactive_threshold": THRESHOLD, "reactive_strength": STRENGTH},
        "output_pixels_per_side": N,
        "units": "linear scene values; contrast = standard deviation; activity = mean per-pixel range over 8 frames",
        "steady_detail_8_16": steady_detail(),
        "pan_8_16": pans(),
        "transitions_8_12_4_16_8": transitions(),
        "stale_shading_residual_fraction": stale_shading(),
        "cut_and_local_reset_error": cut_and_local_reset(),
        "thin_lines_over_clear_8_16": thin_lines(),
        "scope": "scalar static-camera or constant-pan mirror; not a rendered image, GPU result or acceptance evidence",
    }
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        with open(args.out, "w") as fh:
            fh.write(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
