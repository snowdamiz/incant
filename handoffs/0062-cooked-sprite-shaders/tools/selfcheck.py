#!/usr/bin/env python3
"""CPU self-checks for the 0062 analysis tools.

1. A single 1-LSB change in a decoded capture is reported by compare() as one
   differing pixel with worst delta 1, and changes the RGBA hash.
2. Every capture decodes, re-encodes through pngio.write and decodes back to the
   same RGBA rows (reader/writer consistency), and carries only IHDR/IDAT/IEND.

  python3 -I -B selfcheck.py <evidence-dir> <out-json>
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402
from analyze import Frame, compare  # noqa: E402


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, out = Path(sys.argv[1]), Path(sys.argv[2])
    scratch = evidence / "analysis/selfcheck-scratch"
    scratch.mkdir(parents=True)
    pngs = sorted((evidence / "captures").rglob("*.png"))
    frame = Frame(pngs[0])
    original = frame.crop(0, 0, frame.width, frame.height)
    mutated = [list(row) for row in original]
    r, g, b, a = mutated[100][200]
    mutated[100][200] = (r ^ 1, g, b, a)
    mutation = compare(original, mutated)
    rows = [bytes(c for p in row for c in p) for row in mutated]
    mutation["rgba_hash_changed"] = hashlib.sha256(b"".join(rows)).hexdigest() != frame.rgba_sha
    roundtrip = []
    for index, path in enumerate(pngs):
        f = Frame(path)
        copy = scratch / f"{index:02d}.png"
        pngio.write(copy, f.width, f.height, f.rows)
        w, h, rows2 = pngio.read(copy)
        roundtrip.append({"png": str(path.relative_to(evidence)), "chunks": f.chunks,
                          "roundtrip_equal": (w, h, rows2) == (f.width, f.height, f.rows)})
        copy.unlink()
    scratch.rmdir()
    report = {"mutation": mutation, "mutation_detected": mutation["differing"] == 1
              and mutation["worst_channel_delta"] == 1 and mutation["rgba_hash_changed"],
              "roundtrip_all_equal": all(r["roundtrip_equal"] for r in roundtrip),
              "only_ihdr_idat_iend": all(set(r["chunks"]) == {"IHDR", "IDAT", "IEND"} for r in roundtrip),
              "captures_checked": len(roundtrip), "roundtrip": roundtrip}
    with open(out, "x") as handle:
        json.dump(report, handle, indent=1)
        handle.write("\n")
    print(json.dumps({k: v for k, v in report.items() if k != "roundtrip"}))


if __name__ == "__main__":
    main()
