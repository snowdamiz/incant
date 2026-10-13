#!/usr/bin/env python3
"""0073 implementation-4 memory admission control. Arithmetic only, standard
library, deterministic. No image simulation.

    python3 -I -B handoffs/0073-upscaling-review/repair-model/admission_v4.py \
        --out handoffs/0073-upscaling-review/repair-model/results-4.json

Byte terms are logical payload: width x height x texel size for textures and
requested sizes for buffers. They exclude row pitch, tiling and compression
metadata, depth-format padding, allocator rounding, driver memory and work
still queued on the GPU, as the engine's existing constants do.

Accounting modes under the unchanged cap MAX_UPSCALE_TRANSITION_BYTES:
  conservative  today's preflight: accepted set plus a full candidate set,
                even when the candidate reuses the accepted allocation;
  live          each live allocation once: the accepted set plus only the
                components whose extent changes (old and new coexist until
                acceptance).
"""
import argparse
import json
import math
import sys

sys.dont_write_bytecode = True

P = 4 * 1024 * 1024                              # MAX_MOTION_PIXELS
INSTANCE = 64 * 1024 * 1024                      # MAX_MOTION_INSTANCE_BYTES per owner
FIXED = 16384                                    # existing fixed packet metadata
CAP = 128 * P + 2 * INSTANCE + FIXED             # MAX_UPSCALE_TRANSITION_BYTES
OUTPUT = {"colour pair Rgba16Float": 16, "depth pair R32Float": 8, "composite Rgba16Float": 8}
INTERNAL = {"overlay Rgba16Float": 8, "motion Rgba32Float": 16, "depth pair Depth32Float": 8}
OWNER = {"owner R32Uint": 4}                     # this slice
META = {"meta pair Rgba32Uint": 32}              # later pixel-history slice (addendum 3)
LATER_FIXED = 912 + 12512                        # Portable uniform + ChangedBits uniform, per prepared frame

SIZES = {"1280x720": (1280, 720), "1920x1080": (1920, 1080), "2560x1080": (2560, 1080),
         "2556x1179": (2556, 1179), "2796x1290": (2796, 1290), "2560x1440": (2560, 1440),
         "2560x1600": (2560, 1600), "2048x2048": (2048, 2048)}
SCALES = (16, 15, 12, 8, 4)


def source(size, n):
    # FramePlan: exact binary scale, ceil only on attachments.
    return tuple(math.ceil(v * n / 16) for v in size)


def px(size):
    return size[0] * size[1]


def per_px(table):
    return sum(table.values())


class Layout:
    def __init__(self, name, out_bpp, src_bpp, fixed):
        self.name, self.out_bpp, self.src_bpp, self.fixed = name, out_bpp, src_bpp, fixed

    def out_bytes(self, size):
        return self.out_bpp * px(size)

    def src_bytes(self, size, n):
        return self.src_bpp * px(source(size, n))

    def set_bytes(self, size, n):
        return self.out_bytes(size) + self.src_bytes(size, n)


LAYOUTS = (Layout("today", per_px(OUTPUT), per_px(INTERNAL), 0),
           Layout("owner slice", per_px(OUTPUT), per_px(INTERNAL) + per_px(OWNER), 0),
           Layout("later history", per_px(OUTPUT) + per_px(META), per_px(INTERNAL) + per_px(OWNER),
                  LATER_FIXED))


def total(layout, mode, old, new):
    """old/new: (size, n) of the accepted and candidate frames."""
    (so, no), (sn, nn) = old, new
    fixed = 2 * INSTANCE + FIXED + 2 * layout.fixed
    if mode == "conservative":
        return layout.set_bytes(so, no) + layout.set_bytes(sn, nn) + fixed
    live = layout.set_bytes(so, no)
    if so != sn:
        live += layout.out_bytes(sn)
    if source(so, no) != source(sn, nn):
        live += layout.src_bytes(sn, nn)
    return live + fixed


def admitted(layout, mode, old, new):
    caps = px(old[0]) <= P and px(new[0]) <= P
    return caps and total(layout, mode, old, new) <= CAP


def largest_steady(layout, mode, n):
    """Largest square output (w = h) admitted at steady state, in pixels."""
    lo, hi = 1, 2048
    while lo < hi:
        mid = (lo + hi + 1) // 2
        size = (mid, mid)
        if admitted(layout, mode, (size, n), (size, n)):
            lo = mid
        else:
            hi = mid - 1
    return lo * lo


def report():
    res = {"cap_bytes": CAP, "pixel_cap": P, "instance_cap_bytes": INSTANCE, "fixed_bytes": FIXED,
           "bytes_per_output_px": {**OUTPUT, **META}, "bytes_per_internal_px": {**INTERNAL, **OWNER},
           "later_fixed_per_prepared_frame": LATER_FIXED, "checks": [], "tables": {}}

    def check(name, ok, **values):
        res["checks"].append({"check": name, "ok": bool(ok), "values": values})

    full = ((2048, 2048), 16)
    check("today's cap equals two maximum sets plus instances and metadata",
          CAP == 671105024 and total(LAYOUTS[0], "conservative", full, full) == CAP,
          cap=CAP, two_max_sets=total(LAYOUTS[0], "conservative", full, full))
    addendum3 = 2 * (P * (per_px(OUTPUT) + per_px(META)) + P * (per_px(INTERNAL) + per_px(OWNER))) \
        + 2 * INSTANCE + 2 * (912 + 12512) + FIXED
    check("addendum 3's conservative worst case is 973,121,760 B and exceeds the cap",
          addendum3 == 973121760 and addendum3 > CAP, addendum3=addendum3, excess=addendum3 - CAP)
    # Exhaustive grid: live accounting never admits more live bytes than the cap.
    grid = [(w, h) for w in range(256, 2049, 128) for h in range(256, 2049, 128)]
    worst_live = 0
    refused = 0
    for layout in LAYOUTS:
        for so in grid[::3]:
            for sn in grid[::5]:
                for no in (16, 12, 8, 4):
                    for nn in (16, 15, 8, 4):
                        if admitted(layout, "live", (so, no), (sn, nn)):
                            worst_live = max(worst_live, total(layout, "live", (so, no), (sn, nn)))
                        else:
                            refused += 1
    check("live accounting admits nothing above the cap on the transition grid", worst_live <= CAP,
          largest_admitted_live=worst_live, refused_transitions=refused)

    for layout in LAYOUTS:
        rows = {}
        for name, size in SIZES.items():
            row = {}
            for n in SCALES:
                lower = max(n - 1, 4)
                row[f"{n}/16"] = {
                    "steady_conservative": admitted(layout, "conservative", (size, n), (size, n)),
                    "steady_live": admitted(layout, "live", (size, n), (size, n)),
                    "drs_live_to_%d" % lower: admitted(layout, "live", (size, n), (size, lower)),
                }
            rows[name] = row
        largest = {f"{n}/16": {"steady_conservative_px": largest_steady(layout, "conservative", n),
                               "steady_live_px": largest_steady(layout, "live", n)} for n in SCALES}
        # Output resize between two large outputs at the same scale (both sets live).
        resize = {}
        for n in SCALES:
            budget = CAP - 2 * INSTANCE - FIXED - 2 * layout.fixed
            per_out = layout.out_bpp + layout.src_bpp * (n / 16) ** 2
            resize[f"{n}/16"] = {"max_sum_of_old_and_new_output_px": int(budget // per_out)}
        res["tables"][layout.name] = {"bytes_per_output_px": layout.out_bpp,
                                      "bytes_per_internal_px": layout.src_bpp,
                                      "fixed_per_prepared_frame": layout.fixed,
                                      "extents": rows, "largest_square_steady": largest,
                                      "resize_live": resize}
    return res


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out")
    args = parser.parse_args()
    res = report()
    text = json.dumps({"model": "0073 implementation-4 admission arithmetic",
                       "python": sys.version.split()[0], **res}, indent=1)
    if args.out:
        with open(args.out, "w") as f:
            f.write(text + "\n")
    print(text)
    return 0 if all(c["ok"] for c in res["checks"]) else 1


if __name__ == "__main__":
    sys.exit(main())
