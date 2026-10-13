#!/usr/bin/env python3
"""Self-checks for the 0064 analysis tools. CPU only; standard library only.

  python3 -I -B selfcheck.py <evidence-dir> <new-output.json>

1. A single 1-LSB change to one pixel of a real capture is reported by
   analyze.pixel_diff as exactly one differing pixel with delta 1, and changes
   the decoded RGBA hash.
2. Every capture PNG decodes, re-encodes and decodes again to identical RGBA,
   and its decoded RGBA hash equals the helper's own readback hash.
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import pngio  # noqa: E402


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence, output = Path(sys.argv[1]), Path(sys.argv[2])
    result = {"mutation": None, "round_trip": {}}
    first = sorted((evidence / "captures/candidate/main").glob("*.png"))[0]
    width, height, rows = pngio.read(first)
    mutated = [bytearray(r) for r in rows]
    x, y = width // 2, height // 2
    mutated[y][4 * x] = mutated[y][4 * x] ^ 1
    diff = analyze.pixel_diff(rows, [bytes(r) for r in mutated], width)
    result["mutation"] = {"capture": first.name, "pixel": [x, y], "diff": diff,
                          "hash_changed": hashlib.sha256(b"".join(rows)).hexdigest()
                          != hashlib.sha256(b"".join(bytes(r) for r in mutated)).hexdigest(),
                          "detected_exactly": diff["differing_pixels"] == 1 and diff["max_channel_delta"] == 1}
    for report_path in sorted((evidence / "captures").glob("*/*/report.json")):
        report = json.loads(report_path.read_text())
        for frame in report["frames"]:
            if "png" not in frame:
                continue
            path = report_path.parent / frame["png"]["name"]
            w, h, decoded = pngio.read(path)
            again = pngio.read(pngio.encode(w, h, decoded))[2]
            digest = hashlib.sha256(b"".join(decoded)).hexdigest()
            result["round_trip"][str(path.relative_to(evidence))] = {
                "round_trip_identical": again == decoded, "matches_helper_readback": digest == frame["rgba_sha256"],
                "png_sha256_matches_report": hashlib.sha256(path.read_bytes()).hexdigest() == frame["png"]["sha256"]}
    result["all_pass"] = (result["mutation"]["detected_exactly"] and result["mutation"]["hash_changed"]
                          and all(all(v.values()) for v in result["round_trip"].values()))
    with open(output, "x") as out:
        out.write(json.dumps(result, indent=1) + "\n")
    print(json.dumps({"all_pass": result["all_pass"], "captures": len(result["round_trip"]),
                      "mutation": result["mutation"]["diff"]}))


if __name__ == "__main__":
    main()
