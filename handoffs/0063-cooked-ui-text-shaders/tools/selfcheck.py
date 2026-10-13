#!/usr/bin/env python3
"""Self-checks for the handoff 0063 comparison tooling. CPU only, standard library only.

1. A single one-LSB change in a copy of a capture is reported as exactly one pixel with delta 1.
2. Every capture decodes, re-encodes and decodes to identical RGBA (the stdlib codec is lossless).
3. A horizontally mirrored crop compared with the unmirrored crop differs, so the mirror controls
   are not vacuous.

  python3 -I -B selfcheck.py <evidence-dir> <new-output-json>
"""

import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402
from analyze import flip_h  # noqa: E402
from pngio import Raster, decode, diff, encode  # noqa: E402


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    evidence = Path(sys.argv[1])
    pngs = sorted((evidence / "captures").rglob("*.png"))
    frame = Raster.load(evidence / "captures/candidate/W-ortho/t1.png")
    mutated = bytearray(frame.rgba)
    index = 4 * (200 * frame.width + 400)
    mutated[index] = (mutated[index] + 1) % 256
    single = diff(frame, Raster(frame.width, frame.height, mutated))
    round_trips = {}
    for png in pngs:
        w, h, rgba = decode(png.read_bytes())
        round_trips[str(png.relative_to(evidence))] = decode(encode(w, h, rgba))[2] == rgba
    crop = frame.crop(333, 178, 304, 38)
    mirror_detected = diff(flip_h(crop), crop)
    record = {"one_lsb_mutation": single, "one_lsb_detected": single["differing_pixels"] == 1
              and single["max_channel_delta"] == 1, "round_trips": round_trips,
              "all_round_trip": all(round_trips.values()),
              "unmirrored_vs_mirrored_control_crop": mirror_detected,
              "mirror_controls_not_vacuous": mirror_detected["differing_pixels"] > 0}
    c.write_new_json(sys.argv[2], record)
    print({k: record[k] for k in ("one_lsb_detected", "all_round_trip", "mirror_controls_not_vacuous")},
          len(round_trips), mirror_detected["differing_pixels"])


if __name__ == "__main__":
    main()
