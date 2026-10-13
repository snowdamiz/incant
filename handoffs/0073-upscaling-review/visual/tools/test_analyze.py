#!/usr/bin/env python3
"""CPU-only synthetic test of analyze.py's independent re-checks. Standard library only.

It writes deterministic 960x540 frames and a 4x4 figure (built here from the
helper's documented layout: separator fill, 80x45 crops at 3x, each tile's last
row and column kept as separator) as PNGs, then runs analyze.case_review on a
synthetic stationary report. Hash checks, figure re-cuts, UI identity and the
recomputed statistics must all pass, and a one-pixel change in a tile must be
detected. Nothing outside a temporary directory is touched.

  python3 -I -B test_analyze.py [--receipt NEW_PATH]
"""

import hashlib
import json
import struct
import sys
import tempfile
import zlib
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402

W, H = 960, 540
SEPARATOR = bytes([8, 8, 10, 255])


def write_png(path, rgba):
    rows = b"".join(b"\0" + rgba[y * W * 4:(y + 1) * W * 4] for y in range(H))
    chunk = lambda kind, body: (struct.pack(">I", len(body)) + kind + body  # noqa: E731
                                + struct.pack(">I", zlib.crc32(kind + body)))
    Path(path).write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def frame(seed):
    out = bytearray(W * H * 4)
    for p in range(W * H):
        x, y = p % W, p // W
        out[p * 4:p * 4 + 4] = bytes([(x * 7 + y * 3 + seed) % 256, (x * y + seed) % 256,
                                      (x ^ y ^ seed) % 256, 255])
    return out


def figure(tiles):
    """tiles: {slot: (frame, rect)}; 3x nearest crops, separator kept."""
    out = bytearray(SEPARATOR * (W * H))
    for slot, (source, (x0, y0, w, h)) in tiles.items():
        column, row = slot % 4, slot // 4
        for y in range(min(h * 3, 134)):
            for x in range(min(w * 3, 239)):
                i = ((y0 + y // 3) * W + x0 + x // 3) * 4
                j = ((row * 135 + y) * W + column * 240 + x) * 4
                out[j:j + 4] = source[i:i + 4]
    return out


def main():
    results = {}
    with tempfile.TemporaryDirectory(prefix="incant-0073-analyze-") as directory:
        output = Path(directory) / "stationary-r1"
        output.mkdir()
        full, drs, up = frame(1), frame(40), frame(90)
        # Identical opaque UI interiors in all three raw frames.
        left, top, right, bottom = analyze.UI_INTERIOR
        for image in (drs, up):
            for y in range(top, bottom):
                image[(y * W + left) * 4:(y * W + right) * 4] = full[(y * W + left) * 4:(y * W + right) * 4]
        crops = {"wires": [296, 82, 80, 45], "card_edge": [180, 309, 80, 45],
                 "sign": [648, 191, 80, 45], "sprite": [447, 422, 80, 45], "text": [417, 266, 80, 45]}
        raws = {"stationary-full.png": full, "stationary-drs50.png": drs, "stationary-up50.png": up}
        detail = figure({4 * row + column: (raws[name], crops[key])
                         for row, key in enumerate(["wires", "card_edge", "sign", "sprite"])
                         for column, name in enumerate(raws)})
        phases = figure({7: (up, crops["wires"]), 15: (up, crops["text"])})
        images = {**raws, "stationary-detail.png": detail, "stationary-phases.png": phases}
        for name, rgba in images.items():
            write_png(output / name, bytes(rgba))
        keep = lambda x, y: not analyze.in_rect_ui(x, y)  # noqa: E731
        edge = analyze.edges(bytes(full))
        report = {"outcome": "pass", "checks": {}, "frames_charged": 36,
                  "crops": [{"name": k, "rect": v} for k, v in crops.items()],
                  "observations": {"vs_full": {
                      "drs50": analyze.split(bytes(drs), bytes(full), edge, keep),
                      "up50_frame24": analyze.split(bytes(up), bytes(full), edge, keep)}},
                  "pngs": [{"name": n, "sha256": hashlib.sha256((output / n).read_bytes()).hexdigest(),
                            "rgba_sha256": hashlib.sha256(bytes(rgba)).hexdigest()}
                           for n, rgba in images.items()]}
        (output / "report.json").write_text(json.dumps(report))
        entry = {"case": "stationary", "output": str(output), "exit": 0, "timed_out": False}
        review = analyze.case_review(entry)
        results["hashes_match"] = all(p["file_sha256_matches"] and p["decoded_rgba_matches_helper"]
                                      for p in review["pngs"]) and len(review["pngs"]) == 5
        results["fourteen_tiles_recut_and_match"] = (len(review["figure_tiles_recut_from_raw"]) == 14
                                                     and all(t["matches"] for t in review["figure_tiles_recut_from_raw"]))
        results["ui_interior_identity"] = review["ui_interior_identical_full_drs_up24"]
        results["stats_reproduced"] = (review["helper_drs50_stats_reproduced"]
                                       and review["helper_up50_frame24_stats_reproduced"])
        results["stats_nonzero"] = report["observations"]["vs_full"]["drs50"]["interior"]["mean_abs"] > 0
        # A single changed pixel in one tile must be detected.
        changed = bytearray(detail)
        j = ((1 * 135 + 10) * W + 2 * 240 + 10) * 4
        changed[j] ^= 0xFF
        write_png(output / "stationary-detail.png", bytes(changed))
        report["pngs"][3]["sha256"] = hashlib.sha256((output / "stationary-detail.png").read_bytes()).hexdigest()
        report["pngs"][3]["rgba_sha256"] = hashlib.sha256(bytes(changed)).hexdigest()
        (output / "report.json").write_text(json.dumps(report))
        mutated = analyze.case_review(entry)
        mismatched = [(t["figure"], t["slot"]) for t in mutated["figure_tiles_recut_from_raw"] if not t["matches"]]
        results["one_pixel_change_detected_in_slot_6_only"] = mismatched == [("stationary-detail.png", 6)]
    passed = all(results.values())
    receipt = {"cpu_only": True, "passed": passed, "results": results, "python": sys.version.split()[0]}
    if len(sys.argv) == 3 and sys.argv[1] == "--receipt":
        with open(sys.argv[2], "x") as out:
            json.dump(receipt, out, indent=1)
            out.write("\n")
    elif len(sys.argv) != 1:
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    print(json.dumps(receipt, indent=1))
    raise SystemExit(0 if passed else 1)


if __name__ == "__main__":
    main()
