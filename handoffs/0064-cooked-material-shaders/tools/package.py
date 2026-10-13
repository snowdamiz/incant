#!/usr/bin/env python3
"""Copy the compact 0064 evidence set into the packet and write evidence/manifest.json.

Adapted from handoff 0062's committed tools/package.py (read-only reuse).
Captures are copied byte-for-byte (unedited). Text logs drop line-end whitespace
and a final blank line so `git diff --check` passes; the manifest records raw and
committed hashes for every file. Cargo JSON logs, binaries, cooked scenes, caches and private
fixtures stay in the ignored evidence directory and are recorded by hash only.

  python3 -I -B package.py            # create evidence/ (refuses if it exists)
  python3 -I -B package.py --check    # re-verify every manifest entry
"""

import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
PACKET = Path(__file__).resolve().parents[1]
OUT = PACKET / "evidence"
SRC = Path("/Users/andreyyurlov/.codex/incant-evidence/0064")
RUN = SRC / "fixtures/r1"


def sha(data):
    return hashlib.sha256(data).hexdigest()


RUN2 = SRC / "fixtures/r2"
DERIVED = ["M-wide-culling-row", "S-shadows-near", "S-shadows-far"]
REPORTS = [("baseline", "main"), ("candidate", "main"), ("baseline", "shadow"), ("candidate", "shadow"),
           ("candidate", "source-free")]


def plan():
    """Compact selection. The source-free PNG is byte-identical to M-wide.png and the dry-run
    reports repeat the main/shadow frame hashes; those stay hashed-only (analysis.json compares them)."""
    items = []
    index = json.loads((RUN / "plans/index.json").read_text())
    for name in index["main"]["pngs"]:
        items.append((SRC / "captures/candidate/main" / name, f"captures/{name}", "capture"))
    items.append((SRC / "captures/candidate/shadow/S-shadows.png", "captures/S-shadows.png", "capture"))
    for name in DERIVED:
        items.append((SRC / f"analysis/figures/{name}.png", f"derived/{name}.png", "derived"))
    items.append((SRC / "analysis/figures/figures.json", "derived/figures.json", "json"))
    for path in sorted((SRC / "analysis").glob("*.json")):
        items.append((path, f"analysis/{path.name}", "json"))
    items += [(SRC / "gpu-ledger.jsonl", "gpu/ledger.jsonl", "json"),
              (SRC / "gpu-released.json", "gpu/released.json", "json"),
              (SRC / "review-ready.json", "gpu/grant-copy.json", "json"),
              (SRC / "target-lease.json", "lease/target-lease.json", "json")]
    for path in sorted((SRC / "gpu-logs").glob("*.txt")):
        items.append((path, f"logs/{path.name}", "text"))
    for variant, name in REPORTS:
        items.append((SRC / "captures" / variant / name / "report.json", f"reports/{variant}-{name}.json", "json"))
    for path in sorted((SRC / "build").iterdir()):
        kind = "json" if path.suffix == ".json" else "source" if path.name.endswith(".as-built") else "text"
        items.append((path, f"build/{path.name}", kind))
    for path in sorted((SRC / "logs").glob("*.txt")):
        items.append((path, f"build/{path.name}", "text"))
    for run, label in ((RUN, "r1"), (RUN2, "r2")):
        rels = ["build.json", "engine-calls.jsonl", "art/manifest.json"]
        rels += [f"boards/{p.name}" for p in sorted((run / "boards").iterdir())]
        rels += [f"plans/{p.name}" for p in sorted((run / "plans").iterdir())]
        rels += [p.name for p in sorted(run.glob("source-free-*.json"))]
        for rel in rels:
            items.append((run / rel, f"fixtures/{label}/{rel.replace('art/manifest.json', 'art-manifest.json')}",
                          "json"))
    return items


def committed_bytes(raw, kind):
    """Text logs drop line-end spaces/tabs/CR and a final blank line (git diff --check);
    every other kind, including the as-built helper source, is committed byte-for-byte."""
    if kind != "text":
        return raw
    data = b"\n".join(line.rstrip(b" \t\r") for line in raw.split(b"\n"))
    while data.endswith(b"\n\n"):
        data = data[:-1]
    return data


def hashed_only():
    files = sorted((SRC / "logs").glob("*.jsonl")) + sorted((SRC / "bin").rglob("*"))
    files += sorted((SRC / "captures").rglob("*"))
    files += sorted((SRC / "analysis/figures").glob("*.png"))
    for run in (RUN, RUN2):
        files += sorted((run / "cooked").glob("*.scene")) + sorted(run.glob("*/.incant/cache/*/*"))
        files += sorted((run / "art").iterdir()) + sorted((run / "private-fixture").rglob("*"))
    committed = {str(source) for source, _, _ in plan()}
    return {str(p): sha(p.read_bytes()) for p in files if p.is_file() and str(p) not in committed}


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
    manifest = {"packet": "0064-cooked-material-shaders", "files": entries, "hashed_only": hashed_only(),
                "note": "Captures are unedited candidate PNGs; analysis/analysis.json records whether each is "
                        "byte-identical to its baseline file. Derived figures are CPU crops/zooms of captures, "
                        "not renders. Text logs drop only line-end whitespace and a final blank line."}
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
