"""Bounded CPU screen of the next SSAO sampling repair (next-repair-preparation.md).

CPU only: no renderer, GPU, Cargo or target use. Standard library only; uses
../ssao-repair/quality_model.py and quality_report.py read only, and reads the
retained 0070-ssao-filter-001 R8 stage planes (never written).

Configurations, all declared before any was measured. Every one keeps the
16-direction kernel, the binary single-layer test, radius 2, bias 0.01, the
Bayer rotation table and R3's plane-weighted 5-tap H/V filter with R8 writes:

  D    the measured R1/R3 design: t = 0.5 + 0.5 (B[x][y] + 0.5)/16, shared by
       every probe of the texel, 4 steps (64 probes).
  S16  the 16-probe full-stratum candidate proposed in filter-result.md:
       t = (B[x][y] + 0.5)/16, 16 steps (256 probes).
  C1   tile pairing: t = (rev4(r) + 0.5)/16 with r = B[y][x], the rotation
       stratum. (r, rev4(r)) is a 2-D Hammersley (0,4,2)-net; the design's
       transposed pairing fills only 4 of the 16 coarse rotation x phase cells.
  C2   C1 plus a better per-ray phase: t_i = ((rev4(r) ^ s(i)) + 0.5)/16, s
       the second Sobol' coordinate. Per texel (elevation i, azimuth rev4(i),
       phase s(i)) is a (0,4,3)-net, so the 64 probe radii fill all 64 radial
       strata; over the tile every ray keeps a digitally shifted net pairing.
  C3   C2 plus per-step decorrelation: t_ij = ((rev4(r) ^ s(i) ^ T[j]) +
       0.5)/16, T = (0, 5, 10, 15), so a ray's four probes are no longer
       equally spaced.

Metrics per configuration, both projections: N8 window peak-to-peak on the ROI
row and the next six rows (each relative to the shipped algorithm's own value
on that row), N6 authored, N6T (perspective in scope, orthographic as a
diagnostic) and N7 on the three fixtures. The design is bound to the device:
the same metrics are computed from the retained 001 V planes, and the model's
R8 codes are compared texel by texel.

  /opt/homebrew/bin/python3 -I -B screen.py > screen-001.json
"""

import json
import math
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path[:0] = [str(HERE.parent / "ssao-repair")]
import quality_model as m  # noqa: E402
import quality_report as qr  # noqa: E402

E001 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001/outputs")
B = m.BAYER


def rev4(v):
    return int(f"{v:04b}"[::-1], 2)


def sobol2(i):
    """Second Sobol' coordinate of i in sixteenths (direction numbers 1, 3, 5, 15)."""
    out = 0
    for bit, v in enumerate((8, 12, 10, 15)):
        if i >> bit & 1:
            out ^= v
    return out


SIGMA = tuple(sobol2(i) for i in range(16))
TAU = (0, 5, 10, 15)


def is_net(points, dims, m_bits=4):
    """Every elementary box of volume 2^-m holds exactly one point (base 2)."""
    def splits(k, n):
        if k == 1:
            yield (n,)
            return
        for a in range(n + 1):
            for rest in splits(k - 1, n - a):
                yield (a,) + rest
    for split in splits(dims, m_bits):
        boxes = {}
        for p in points:
            key = tuple(v >> (m_bits - a) for v, a in zip(p, split))
            boxes[key] = boxes.get(key, 0) + 1
        if len(boxes) != 2 ** m_bits or any(n != 1 for n in boxes.values()):
            return False
    return True


def tile_pairs(phase_index):
    return [(B[y][x], phase_index(x, y)) for y in range(4) for x in range(4)]


def coarse_cells(pairs):
    return len({(r >> 2, p >> 2) for r, p in pairs})


CONFIGS = {
    "D": {"steps": 4, "phase": lambda x, y, i, j: 0.5 + 0.5 * (B[x % 4][y % 4] + 0.5) / 16},
    "S16": {"steps": 16, "phase": lambda x, y, i, j: (B[x % 4][y % 4] + 0.5) / 16},
    "C1": {"steps": 4, "phase": lambda x, y, i, j: (rev4(B[y % 4][x % 4]) + 0.5) / 16},
    "C2": {"steps": 4, "phase": lambda x, y, i, j: ((rev4(B[y % 4][x % 4]) ^ SIGMA[i]) + 0.5) / 16},
    "C3": {"steps": 4, "phase": lambda x, y, i, j: ((rev4(B[y % 4][x % 4]) ^ SIGMA[i] ^ TAU[j]) + 0.5) / 16},
}


def raw(grid, config, pixels):
    """quality_model.raw's single-layer test with a per-probe phase."""
    steps, phase = CONFIGS[config]["steps"], CONFIGS[config]["phase"]
    w, project, hits, r2 = grid.w, grid.camera.project, grid.hits, m.RADIUS * m.RADIUS
    out = [None] * len(hits)
    for index in pixels:
        hit = hits[index]
        if hit is None:
            out[index] = 1.0
            continue
        y, x = divmod(index, w)
        px, py, pz = hit[0], hit[1], hit[2]
        cos, sin = m.ROTATIONS[B[y % 4][x % 4]]
        count = 0
        for ray, (kx, ky, kz) in enumerate(m.DIRECTIONS):
            dx = kx * cos - ky * sin
            dy = kx * sin + ky * cos
            for j in range(steps):
                s = m.RADIUS * (j + phase(x, y, ray, j)) / steps
                qx, qy, qz = px + dx * s, py + dy * s, pz + kz * s
                at = project(qx, qy, qz)
                if at is None:
                    continue
                sx, sy = int(math.floor(at[0])), int(math.floor(at[1]))
                if not (0 <= sx < w and 0 <= sy < grid.h):
                    continue
                surface = hits[sy * w + sx]
                if surface is None:
                    continue
                ex, ey, ez = surface[0] - px, surface[1] - py, surface[2] - pz
                if surface[2] >= qz + m.BIAS and ez > m.BIAS and ex * ex + ey * ey + ez * ez <= r2:
                    count += 1
                    break
        out[index] = 1.0 - count / 16
    return out


def output(grid, config, pixels):
    """Filtered R8 output on `pixels` (raw evaluated on their 5x5 support)."""
    support = m.support(grid, set(pixels))
    return m.filtered(grid, raw(grid, config, support), pixels=set(pixels))


def full(grid, config):
    return m.filtered(grid, raw(grid, config, range(len(grid.hits))))


def device_plane(case, label):
    return [c / 255 for c in (E001 / case / f"{label}.v.r8").read_bytes()]


class Fixtures:
    """Grids, ROI windows, shipped baselines and continuous truths, shared by all configurations."""

    def __init__(self):
        self.stripes = {o: m.Grid(qr.STRIPES, o) for o in (False, True)}
        self.slab = {o: m.Grid("white_slab", o) for o in (False, True)}
        self.edge = m.Grid("edge_slab", False)
        self.rows, self.shipped = {}, {}
        for o, g in self.stripes.items():
            row, cols = qr.n8_roi(g)
            self.rows[o] = {r: [r * g.w + x for x in cols] for r in range(row, row + 7)}
            for r, idx in self.rows[o].items():
                old = m.raw(g, rotate=False, jitter=False, pixels=set(idx))
                self.shipped[(o, r)] = max(old[i] for i in idx) - min(old[i] for i in idx)
        self.span, self.truth, self.pairs = {}, {}, {}
        for o, g in self.slab.items():
            at = (lambda gg: lambda x: qr.N6_ROW * gg.w + x)(g)
            self.span[o] = [at(x) for x in range(qr.N6_X[0], qr.N6_X[1] + 1)]
            raw_truth = m.raw_directions(g, m.support(g, set(self.span[o])), *qr.CONTINUOUS)
            self.truth[o] = m.filtered(g, [raw_truth.get(k) for k in range(len(g.hits))], quantize=False,
                                       pixels=set(self.span[o]))
            self.pairs[o] = qr.n6_pairs(g)

    def n8(self, o, values_by_row):
        out = {}
        for r, idx in self.rows[o].items():
            v = values_by_row[r]
            p2p = max(v[i] for i in idx) - min(v[i] for i in idx)
            out[str(r)] = {"ratio": round(p2p / self.shipped[(o, r)], 3), "p2p_255": round(p2p * 255, 2),
                           "shipped_255": round(self.shipped[(o, r)] * 255, 2)}
        return out

    def n6t(self, o, values):
        g, truth = self.slab[o], self.truth[o]
        at = lambda x: qr.N6_ROW * g.w + x  # noqa: E731
        return round(max(abs((values[at(x + 1)] - values[at(x)]) - (truth[at(x + 1)] - truth[at(x)]))
                         for x in self.pairs[o]) * 255, 2)


def metrics(fx, values):
    """values: {'stripes_p', 'stripes_o', 'slab_p', 'slab_o', 'edge_p'} -> full-length value lists."""
    n6, at = m.n6_row(fx.slab[False], values["slab_p"])
    return {
        "n8_persp": fx.n8(False, {r: values["stripes_p"] for r in fx.rows[False]}),
        "n8_ortho": fx.n8(True, {r: values["stripes_o"] for r in fx.rows[True]}),
        "n6_authored_persp": {"step_255": round(n6 * 255, 2), "at": at},
        "n6t_persp_255": fx.n6t(False, values["slab_p"]),
        "n6t_ortho_255_diagnostic": fx.n6t(True, values["slab_o"]),
        "n7": {"white_slab_persp": m.n7_steps(fx.slab[False], values["slab_p"]),
               "white_slab_ortho": m.n7_steps(fx.slab[True], values["slab_o"]),
               "edge_slab_persp": m.n7_steps(fx.edge, values["edge_p"])},
    }


def model_values(fx, config):
    """Full frames for N7 (which also cover N6); windows for N8."""
    stripes = {}
    for o, g in fx.stripes.items():
        window = [i for idx in fx.rows[o].values() for i in idx]
        stripes[o] = output(g, config, window)
    return {"stripes_p": stripes[False], "stripes_o": stripes[True],
            "slab_p": full(fx.slab[False], config), "slab_o": full(fx.slab[True], config),
            "edge_p": full(fx.edge, config)}


def agreement(model, device, indices):
    diffs = [abs(round(model[i] * 255) - round(device[i] * 255)) for i in indices]
    return {"texels": len(diffs), "equal": sum(d == 0 for d in diffs), "max_abs_codes": max(diffs)}


def self_checks(fx):
    """The generalised raw reproduces quality_model.raw bit for bit (D and S16)."""
    g = fx.stripes[False]
    window = m.support(g, {i for idx in fx.rows[False].values() for i in idx})
    d = raw(g, "D", window) == m.raw(g, pixels=window)
    s = raw(g, "S16", window) == m.raw(g, full_stratum=True, steps=16, pixels=window)
    slab = fx.slab[False]
    full_d = raw(slab, "D", range(len(slab.hits))) == m.raw(slab)
    return {"D_equals_quality_model_raw": d and full_d, "S16_equals_quality_model_raw": s}


def main():
    start = time.time()
    fx = Fixtures()
    nets = {
        "rays_elevation_azimuth_phase_0_4_3_net": is_net([(i, rev4(i), SIGMA[i]) for i in range(16)], 3),
        "design_tile_pairing_0_4_2_net": is_net(tile_pairs(lambda x, y: B[x][y]), 2),
        "design_tile_pairing_coarse_cells_of_16": coarse_cells(tile_pairs(lambda x, y: B[x][y])),
        "c1_tile_pairing_0_4_2_net": is_net(tile_pairs(lambda x, y: rev4(B[y][x])), 2),
        "c1_tile_pairing_coarse_cells_of_16": coarse_cells(tile_pairs(lambda x, y: rev4(B[y][x]))),
        "c2_every_ray_tile_pairing_0_4_2_net": all(
            is_net(tile_pairs(lambda x, y, c=SIGMA[i]: rev4(B[y][x]) ^ c), 2) for i in range(16)),
        "c3_every_probe_tile_pairing_0_4_2_net": all(
            is_net(tile_pairs(lambda x, y, c=SIGMA[i] ^ t: rev4(B[y][x]) ^ c), 2) for i in range(16) for t in TAU),
        "c3_per_texel_radial_strata_64": all(
            len({(j, p ^ SIGMA[i] ^ TAU[j]) for i in range(16) for j in range(4)}) == 64 for p in range(16)),
    }
    report = {"format": "incant-0070-next-repair-screen-v1", "sigma": SIGMA, "tau": TAU, "constructions": nets,
              "self_checks": self_checks(fx), "configs": {}}
    timings = {}
    for config in CONFIGS:
        t0 = time.time()
        values = model_values(fx, config)
        report["configs"][config] = {"probes_per_texel": 16 * CONFIGS[config]["steps"], **metrics(fx, values)}
        if config == "D":
            design = values
        timings[config] = round(time.time() - t0, 1)
    device = {"stripes_p": device_plane("case-ssao_quality", "p_ao_stripes"),
              "stripes_o": device_plane("case-ssao_quality", "o_ao_stripes"),
              "slab_p": device_plane("case-ssao_core", "p_ao_white"),
              "slab_o": device_plane("case-ssao_core", "o_ao_white"),
              "edge_p": device_plane("case-ssao_core", "p_ao_edge")}
    windows = {o: [i for idx in fx.rows[o].values() for i in idx] for o in (False, True)}
    interior = {k: [i for i in range(len(g.hits)) if g.interior(*reversed(divmod(i, g.w)))]
                for k, g in (("slab_p", fx.slab[False]), ("slab_o", fx.slab[True]), ("edge_p", fx.edge))}
    report["device_design_001"] = {
        "source": "retained 0070-ssao-filter-001 V planes (final R8 codes), Apple M5 Pro / Metal",
        **metrics(fx, device),
        "model_agreement": {
            "n8_windows_persp": agreement(design["stripes_p"], device["stripes_p"], windows[False]),
            "n8_windows_ortho": agreement(design["stripes_o"], device["stripes_o"], windows[True]),
            **{f"{k}_interior": agreement(design[k], device[k], v) for k, v in interior.items()}}}
    report["seconds"] = {**timings, "total": round(time.time() - start, 1)}
    json.dump(report, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
