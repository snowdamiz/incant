#!/usr/bin/env python3
"""Which UI/text shader bytes each frozen binary embeds (handoff 0063). CPU only.

Counts exact occurrences of the complete ui.wgsl and text.wgsl sources and of
each retained cooked stage artifact (Astra's committed evidence copies, gunzipped)
inside every frozen baseline and candidate executable. Never runs a binary.

  python3 -I -B embed.py <new-output-json>
"""

import gzip
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

ARTIFACT_DIRS = {"ui": "docs/spikes/evidence/cooked-ui-shaders/metal/artifacts",
                 "text": "docs/spikes/evidence/cooked-text-shaders/cooked-text-metal-001/artifacts"}
WGSL = {"ui": "crates/incant_render/src/ui/ui.wgsl", "text": "crates/incant_render/src/text/text.wgsl"}


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    needles = {f"{family}.wgsl": (c.ROOT / path).read_bytes() for family, path in WGSL.items()}
    for family, directory in ARTIFACT_DIRS.items():
        for path in sorted((c.ROOT / directory).glob("*.shader.gz")):
            needles[f"{family}:{path.name[:-3]}"] = gzip.decompress(path.read_bytes())
    report = {"needles": {k: {"bytes": len(v), "sha256": c.sha_bytes(v)} for k, v in needles.items()},
              "binaries": {}}
    for variant in ("baseline", "candidate"):
        frozen = c.read_json(c.EVIDENCE / "build" / f"{variant}.json")["frozen"]
        for name, record in frozen.items():
            data = Path(record["path"]).read_bytes()
            if c.sha_bytes(data) != record["sha256"]:
                raise SystemExit(f"{variant}/{name} changed since freezing")
            report["binaries"][f"{variant}/{name}"] = {"sha256": record["sha256"],
                                                       "counts": {k: data.count(v) for k, v in needles.items()}}
    c.write_new_json(sys.argv[1], report)
    for key, value in report["binaries"].items():
        print(key, json.dumps(value["counts"]))


if __name__ == "__main__":
    main()
