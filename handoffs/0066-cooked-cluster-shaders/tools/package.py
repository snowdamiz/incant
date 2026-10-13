#!/usr/bin/env python3
"""Copy the compact 0066 evidence set into the packet and write evidence/manifest.json.

Adapted from handoff 0064's committed tools/package.py (read-only reuse).
Captures are copied byte-for-byte (unedited). Text logs drop line-end whitespace
and a final blank line so `git diff --check` passes. JSON files over 200 KB are
committed as deterministic gzip (mtime 0); decompressing them restores the raw
bytes. The manifest records raw and committed hashes for every file. Cargo JSON
logs, binaries, cooked scenes, caches, art, private fixture copies, dry-run
reports and duplicate PNGs stay in the ignored evidence directory, recorded by
hash only. The packet helper's committed sources must equal the sources its
frozen binaries were built from.

  python3 -I -B package.py            # create evidence/ (refuses if it exists)
  python3 -I -B package.py --check    # re-verify every manifest entry
"""

import gzip
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PACKET = Path(__file__).resolve().parents[1]
ROOT = PACKET.parents[1]
OUT = PACKET / "evidence"
SRC = Path("/Users/andreyyurlov/.codex/incant-evidence/0066")
RUN = SRC / "fixtures/r1"
GZIP_OVER = 200 * 1024
REPORTS = [("baseline", "main"), ("candidate", "main"), ("candidate", "source-free")]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def plan():
    """Compact selection: (source, committed relative path, kind)."""
    items = []
    index = json.loads((RUN / "plans/index.json").read_text())
    for name in index["main"]["pngs"]:
        items.append((SRC / "captures/candidate/main" / name, f"captures/{name}", "capture"))
    for path in sorted((SRC / "analysis/figures").glob("*.png")):
        items.append((path, f"derived/{path.name}", "derived"))
    items.append((SRC / "analysis/figures/figures.json", "derived/figures.json", "json"))
    for path in sorted((SRC / "analysis").glob("*.json")):
        items.append((path, f"analysis/{path.name}", "json"))
    items += [(SRC / "gpu-ledger.jsonl", "gpu/ledger.jsonl", "json"),
              (SRC / "gpu-released.json", "gpu/released.json", "json"),
              (SRC / "gpu-refusals.jsonl", "gpu/refusals.jsonl", "json"),
              (SRC / "review-ready.json", "gpu/grant-copy.json", "json"),
              (SRC / "target-lease.json", "lease/target-lease.json", "json")]
    for path in sorted((SRC / "gpu-logs").glob("*.txt")):
        items.append((path, f"logs/{path.name}", "text"))
    for variant, name in REPORTS:
        items.append((SRC / "captures" / variant / name / "report.json", f"reports/{variant}-{name}.json", "json"))
    for path in sorted((SRC / "build").iterdir()):
        items.append((path, f"build/{path.name}", "json"))
    for path in sorted((SRC / "logs").glob("*.txt")):
        items.append((path, f"build/{path.name}", "text"))
    rels = ["build.json", "engine-calls.jsonl", "art/manifest.json"]
    rels += [f"boards/{p.name}" for p in sorted((RUN / "boards").iterdir())]
    rels += [f"plans/{p.name}" for p in sorted((RUN / "plans").iterdir())]
    rels += [p.name for p in sorted(RUN.glob("source-free-*.json"))]
    for rel in rels:
        items.append((RUN / rel, f"fixtures/r1/{rel.replace('art/manifest.json', 'art-manifest.json')}", "json"))
    return items


def committed(raw, kind):
    """(bytes, suffix, encoding). Text logs drop line-end spaces/tabs/CR and a final blank line;
    large JSON is gzip-compressed losslessly; everything else is byte-for-byte."""
    if kind == "text":
        data = b"\n".join(line.rstrip(b" \t\r") for line in raw.split(b"\n"))
        while data.endswith(b"\n\n"):
            data = data[:-1]
        return data, "", "normalized-text" if data != raw else "raw"
    if kind == "json" and len(raw) > GZIP_OVER:
        return gzip.compress(raw, compresslevel=9, mtime=0), ".gz", "gzip"
    return raw, "", "raw"


def hashed_only():
    files = sorted((SRC / "logs").glob("*.jsonl")) + sorted((SRC / "bin").rglob("*"))
    files += sorted((SRC / "captures").rglob("*")) + sorted((SRC / "scratch").rglob("*"))
    files += sorted((RUN / "cooked").glob("*.scene")) + sorted(RUN.glob("*/.incant/cache/*/*"))
    files += sorted(RUN.glob("*/.incant-source-transactions/*"))
    files += sorted((RUN / "art").iterdir()) + sorted((RUN / "private-fixture").rglob("*"))
    listed = {str(source) for source, _, _ in plan()}
    return {str(p): sha(p.read_bytes()) for p in files if p.is_file() and str(p) not in listed}


def helper_identity():
    built = json.loads((SRC / "build/review-candidate.json").read_text())["helper_sources"]
    other = json.loads((SRC / "build/review-baseline.json").read_text())["helper_sources"]
    now = {rel: sha((ROOT / rel).read_bytes()) for rel in built}
    return {"built_from": built, "same_for_both_variants": built == other, "committed": now,
            "committed_equals_built": now == built}


def tools_identity():
    return {str(p.relative_to(ROOT)): sha(p.read_bytes()) for p in sorted((PACKET / "tools").glob("*.py"))}


def build():
    if OUT.exists():
        raise SystemExit("evidence/ exists; use --check")
    helper = helper_identity()
    if not (helper["committed_equals_built"] and helper["same_for_both_variants"]):
        raise SystemExit(f"helper sources differ from the frozen builds: {helper}")
    entries = []
    for source, rel, kind in plan():
        raw = source.read_bytes()
        data, suffix, encoding = committed(raw, kind)
        target = OUT / (rel + suffix)
        target.parent.mkdir(parents=True, exist_ok=True)
        with open(target, "xb") as out:
            out.write(data)
        entries.append({"path": rel + suffix, "kind": kind, "source": str(source), "raw_sha256": sha(raw),
                        "committed_sha256": sha(data), "encoding": encoding, "bytes": len(data)})
    manifest = {"packet": "0066-cooked-cluster-shaders", "files": entries, "hashed_only": hashed_only(),
                "helper": helper, "tools": tools_identity(),
                "note": "Captures are unedited candidate PNGs; analysis/analysis.json records that each is "
                        "byte-identical to its baseline file. Derived figures are CPU crops/zooms of captures "
                        "with tile-edge ticks in an added margin, not renders. Text logs drop only line-end "
                        "whitespace and a final blank line; *.json.gz decompress to the raw bytes."}
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
    print(json.dumps({"files": len(entries), "bytes": sum(e["bytes"] for e in entries),
                      "encodings": {k: sum(e["encoding"] == k for e in entries)
                                    for k in ("raw", "normalized-text", "gzip")},
                      "hashed_only": len(manifest["hashed_only"])}))


def check():
    manifest = json.loads((OUT / "manifest.json").read_text())
    problems = []
    for entry in manifest["files"]:
        data = (OUT / entry["path"]).read_bytes()
        source = Path(entry["source"])
        if sha(data) != entry["committed_sha256"]:
            problems.append(f"committed hash {entry['path']}")
        restored = gzip.decompress(data) if entry["encoding"] == "gzip" else None
        if restored is not None and sha(restored) != entry["raw_sha256"]:
            problems.append(f"gzip does not restore raw bytes {entry['path']}")
        if source.exists():
            raw = source.read_bytes()
            if sha(raw) != entry["raw_sha256"]:
                problems.append(f"source changed {entry['source']}")
            elif committed(raw, entry["kind"])[0] != data:
                problems.append(f"encoding {entry['path']}")
    for path, digest in manifest["hashed_only"].items():
        if Path(path).exists() and sha(Path(path).read_bytes()) != digest:
            problems.append(f"hashed-only changed {path}")
    if not helper_identity()["committed_equals_built"]:
        problems.append("helper sources changed after the frozen builds")
    if tools_identity() != manifest["tools"]:
        problems.append("packet tools changed after packaging")
    listed = {e["path"] for e in manifest["files"]} | {"manifest.json"}
    extra = sorted(str(p.relative_to(OUT)) for p in OUT.rglob("*") if p.is_file()
                   and str(p.relative_to(OUT)) not in listed)
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
