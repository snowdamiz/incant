#!/usr/bin/env python3
"""Copy the compact 0062 evidence set into the packet and write evidence/manifest.json.

Captures are copied byte-for-byte (unedited). Text logs drop only libtest's final
blank line so `git diff --check` passes; the manifest records raw and committed
hashes for every file. Large Cargo JSON logs, binaries and private fixtures stay
in the ignored evidence directory and are recorded by hash only.

  python3 -I -B package.py            # create evidence/ (refuses if it exists)
  python3 -I -B package.py --check    # re-verify every manifest entry
"""

import hashlib
import json
import shutil
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PACKET = Path(__file__).resolve().parents[1]
OUT = PACKET / "evidence"
SRC = Path("/Users/andreyyurlov/.codex/incant-evidence/0062")
CAPTURES = [("A-pp", 0), ("A-ortho", 0), ("A-persp", 0), ("W-pp", 0), ("W-ortho", 0), ("B-pp", 0), ("B-pp", 48),
            ("B-ortho", 0)]
DERIVED = ["ties-A2-A6-A7", "atlas-edges-ring", "plain-sheet-neighbours", "premultiplied-alpha",
           "order-ties-depth", "tilemap-vs-cells", "flipbooks-tick48", "wide-ends"]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def plan():
    items = []
    for case, tick in CAPTURES:
        items.append((SRC / f"captures/candidate/{case}/tick-{tick:05d}.png", f"captures/{case}-tick-{tick:05d}.png",
                      "capture"))
    for name in DERIVED:
        items.append((SRC / f"analysis/figures/{name}.png", f"derived/{name}.png", "derived"))
    items.append((SRC / "analysis/figures/figures.json", "derived/figures.json", "json"))
    for name in ("analysis.json", "selfcheck.json"):
        items.append((SRC / "analysis" / name, f"analysis/{name}", "json"))
    items += [(SRC / "gpu-ledger.jsonl", "gpu/ledger.jsonl", "json"),
              (SRC / "gpu-released.json", "gpu/released.json", "json"),
              (SRC / "gpu-review-ready", "gpu/grant-marker-copy.json", "json"),
              (SRC / "target-lease.json", "lease/target-lease.json", "json")]
    for path in sorted((SRC / "gpu-logs").glob("*.txt")):
        items.append((path, f"logs/{path.name}", "text"))
    for path in sorted((SRC / "captures").glob("*/*/report.json")):
        items.append((path, f"reports/{path.parent.parent.name}-{path.parent.name}.json", "json"))
    for path in sorted((SRC / "build").glob("*.json")):
        items.append((path, f"build/{path.name}", "json"))
    for path in sorted((SRC / "logs").glob("*.txt")):
        items.append((path, f"build/{path.name}", "text"))
    r2 = SRC / "fixtures/r2"
    for rel in ["build.json", "engine-calls.jsonl", "source-free-A.json", "source-free-W.json", "source-free-B.json",
                "art/manifest.json"] + [f"boards/{p.name}" for p in sorted((r2 / "boards").iterdir())] + \
            [f"requests/{p.name}" for p in sorted((r2 / "requests").iterdir())]:
        items.append((r2 / rel, f"fixtures/{rel.replace('art/manifest.json', 'art-manifest.json')}", "json"))
    items.append((SRC / "fixtures/r1-failed-layer-ulid/engine-calls.jsonl", "fixtures/r1-failed-engine-calls.jsonl",
                  "json"))
    return items


def committed_bytes(raw, kind):
    if kind == "text" and raw.endswith(b"\n\n"):
        return raw[:-1]
    return raw


def hashed_only():
    files = sorted((SRC / "logs").glob("*.jsonl")) + sorted((SRC / "bin").rglob("*"))
    return {str(p): sha(p.read_bytes()) for p in files if p.is_file()}


def build():
    if OUT.exists():
        raise SystemExit("evidence/ exists; use --check")
    entries = []
    for source, rel, kind in plan():
        raw = source.read_bytes()
        data = committed_bytes(raw, kind)
        target = OUT / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        with open(target, "xb") as out:
            out.write(data)
        entries.append({"path": rel, "kind": kind, "source": str(source), "raw_sha256": sha(raw),
                        "committed_sha256": sha(data), "normalized": raw != data, "bytes": len(data)})
    manifest = {"packet": "0062-cooked-sprite-shaders", "files": entries, "hashed_only": hashed_only(),
                "note": "Captures are unedited candidate PNGs (byte-identical to baseline). Derived figures are "
                        "CPU crops/zooms of those captures, not renders. Text logs drop only a final blank line."}
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
    print(json.dumps({"files": len(entries), "bytes": sum(e["bytes"] for e in entries),
                      "normalized": sum(e["normalized"] for e in entries)}))


def check():
    manifest = json.loads((OUT / "manifest.json").read_text())
    problems = []
    for entry in manifest["files"]:
        data = (OUT / entry["path"]).read_bytes()
        source = Path(entry["source"])
        if sha(data) != entry["committed_sha256"]:
            problems.append(f"committed hash {entry['path']}")
        if source.exists() and sha(source.read_bytes()) != entry["raw_sha256"]:
            problems.append(f"source changed {entry['source']}")
        if source.exists() and committed_bytes(source.read_bytes(), entry["kind"]) != data:
            problems.append(f"normalization {entry['path']}")
    for path, digest in manifest["hashed_only"].items():
        if Path(path).exists() and sha(Path(path).read_bytes()) != digest:
            problems.append(f"hashed-only changed {path}")
    extra = sorted(str(p.relative_to(OUT)) for p in OUT.rglob("*") if p.is_file()
                   and str(p.relative_to(OUT)) not in {e["path"] for e in manifest["files"]} | {"manifest.json"})
    print(json.dumps({"checked": len(manifest["files"]), "problems": problems, "unlisted": extra}))
    if problems or extra:
        raise SystemExit(1)


if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        check()
    elif not sys.argv[1:]:
        build()
    else:
        raise SystemExit(__doc__)
