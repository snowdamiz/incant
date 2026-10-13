#!/usr/bin/env python3
"""Retain handoff 0067 evidence in the packet directory. CPU only; standard library.

  python3 -I -B retain.py <evidence-dir> <new-packet-evidence-dir>

Copies the bounded evidence (ledgers, raw receipts, regression outputs, build
identities, analysis, appearance captures, derived figures, executed helper
sources) into the repository packet. Files above 64 KiB, and every build JSON
stream, are stored as deterministic gzip (mtime 0, no name), as is raw text that git's
whitespace check would reject. The manifest maps
each retained file to its original path and raw/retained SHA-256. Frozen
binaries stay outside the repository; their hashes are in the build records.
"""

import gzip
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path

LIMIT = 64 * 1024
COPY = [
    ("gpu-ledger.jsonl", "gpu"), ("gpu-released.json", "gpu"), ("gpu-refusals.jsonl", "gpu"),
    ("target-lease.json", "gpu"),
    ("build/source-freeze.json", "build"), ("build/tests.json", "build"), ("build/helper.json", "build"),
    ("build/helper-r2.json", "build"), ("build/helper-r3.json", "build"),
    ("analysis/analysis-v3.json", "analysis"),
    ("captures/appearance/report.json", "appearance"), ("figures/figures.json", "figures"),
    ("executed-source/helper-r3-sources.tar.gz", "executed-source"),
]
GLOBS = [
    ("build/helper-lock-*.json", "build"), ("gpu-logs/*.txt", "gpu-logs"), ("logs/*", "build-logs"),
    ("captures/appearance/*.png", "appearance"), ("captures/recheck/*.png", "recheck"),
    ("figures/*.png", "figures"), ("tmp/*.console.txt", "build-logs"),
]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    source, target = Path(sys.argv[1]), Path(sys.argv[2])
    target.mkdir(parents=True, exist_ok=False)
    items = [(source / rel, sub) for rel, sub in COPY]
    for pattern, sub in GLOBS:
        items.extend((p, sub) for p in sorted(source.glob(pattern)) if p.is_file())
    manifest = []
    for path, sub in items:
        data = path.read_bytes()
        destination = target / sub
        destination.mkdir(parents=True, exist_ok=True)
        # Raw text that git's whitespace check would reject (libtest's final blank
        # line, rustfmt diff context) is kept byte-exact inside gzip instead.
        awkward = data.endswith(b"\n\n") or re.search(rb"[ \t]+\n", data) is not None
        compress = (len(data) > LIMIT or awkward or path.suffix == ".jsonl" and "build-" in path.name) \
            and path.suffix not in (".png", ".gz")
        name = path.name.replace(".log", ".txt")
        if compress:
            out = destination / (name + ".gz")
            blob = gzip.compress(data, compresslevel=9, mtime=0)
            out.write_bytes(blob)
        else:
            out = destination / name
            shutil.copyfile(path, out)
            blob = data
        manifest.append({"original": str(path), "retained": str(out.relative_to(target)),
                         "raw_sha256": sha(data), "sha256": sha(blob), "bytes": len(data),
                         "compression": "gzip" if compress else None})
    (target / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
    print(json.dumps({"files": len(manifest), "bytes": sum(m["bytes"] for m in manifest),
                      "compressed": sum(1 for m in manifest if m["compression"])}, indent=1))


if __name__ == "__main__":
    main()
