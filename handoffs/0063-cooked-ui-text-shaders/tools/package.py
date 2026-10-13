#!/usr/bin/env python3
"""Copy the compact 0063 evidence set into the packet and write evidence/manifest.json.

Captures are copied byte-for-byte (unedited candidate PNGs, each byte-identical to
its baseline). Text logs drop only trailing whitespace and a final blank line so
`git diff --check` passes; the manifest records raw and committed hashes for every
file. Cargo JSON logs, binaries, analysis drafts, superseded r2 frames and private
fixture copies stay in the ignored evidence directory and are recorded by hash only.

  python3 -I -B package.py            # create evidence/ (refuses if it exists)
  python3 -I -B package.py --check    # re-verify every manifest entry
"""

import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

OUT = c.PACKET_DIR / "evidence"
SRC = c.EVIDENCE
CAPTURES = ["W-ortho/t1", "W-persp/t1", "W-persp/t61", "U-1x/idle", "U-1x/select", "U-15x-name/preedit",
            "U-2x/focus", "U-1x-unorm/idle"]


def plan():
    items = []
    for frame in CAPTURES:
        case, name = frame.split("/")
        items.append((SRC / f"captures/candidate/{case}/{name}.png", f"captures/{case}-{name}.png", "binary"))
    for path in sorted((SRC / "analysis/figures").glob("*.png")):
        items.append((path, f"derived/{path.name}", "binary"))
    items.append((SRC / "analysis/figures/figures.json", "derived/figures.json", "json"))
    for name in ("analysis.json", "selfcheck.json", "embed.json"):
        items.append((SRC / "analysis" / name, f"analysis/{name}", "json"))
    items += [(SRC / "gpu-ledger.jsonl", "gpu/ledger.jsonl", "json"),
              (SRC / "gpu-released.json", "gpu/released.json", "json"),
              (SRC / "review-ready.json", "gpu/grant-copy.json", "json"),
              (SRC / "target-lease.json", "lease/target-lease.json", "json"),
              (SRC / "captures-r2-superseded/MOVED.json", "gpu/r2-superseded-moved.json", "json"),
              (SRC / "release-receipt.json", "gpu/release-receipt.json", "json"),
              (SRC / "commands.json", "ledger/commands.json", "json")]
    for path in sorted((SRC / "gpu-logs").glob("*.txt")):
        items.append((path, f"logs/{path.name}", "text"))
    for path in sorted((SRC / "captures").glob("*/*/report.json")):
        items.append((path, f"reports/{path.parent.parent.name}-{path.parent.name}.json", "json"))
    for path in sorted((SRC / "build").glob("*.json")):
        items.append((path, f"build/{path.name}", "json"))
    items.append((SRC / "build/review-main.rs.as-built", "build/review-main.rs.as-built", "text"))
    for path in sorted((SRC / "logs").glob("*.txt")):
        items.append((path, f"build/{path.name}", "text"))
    for run in ("r1", "r2", "r3"):
        root = SRC / "fixtures" / run
        names = ["build.json", "engine-calls.jsonl"]
        if run == "r3":
            names += [f"boards/{p.name}" for p in sorted((root / "boards").iterdir())]
            names += [f"requests/{p.name}" for p in sorted((root / "requests").iterdir())]
            names += [f"cases-extra/{p.name}" for p in sorted((root / "cases-extra").iterdir())]
            names += ["source-free-W.json", "source-free-U.json"]
        for rel in names:
            items.append((root / rel, f"fixtures/{run}/{rel}", "json"))
    return items


def committed_bytes(raw, kind):
    if kind != "text":
        return raw
    text = raw.decode("utf-8")
    text = re.sub(r"[ \t]+$", "", text, flags=re.M)
    while text.endswith("\n\n"):
        text = text[:-1]
    return text.encode("utf-8")


def hashed_only():
    files = sorted((SRC / "logs").glob("*.jsonl")) + sorted((SRC / "bin").rglob("*")) + \
        sorted((SRC / "analysis").glob("analysis-draft*.json")) + \
        sorted((SRC / "captures-r2-superseded").rglob("*.png")) + \
        sorted((SRC / "captures/baseline").rglob("*.png")) + \
        sorted((SRC / "captures/candidate").glob("*-source-free/*.png")) + \
        sorted((SRC / "fixtures/r3/private-fixture").rglob("*")) + \
        sorted((SRC / "fixtures/r3/cooked").glob("*.scene"))
    return {str(p): c.sha(p) for p in files if p.is_file()}


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
        entries.append({"path": rel, "kind": kind, "source": str(source), "raw_sha256": c.sha_bytes(raw),
                        "committed_sha256": c.sha_bytes(data), "normalized": raw != data, "bytes": len(data)})
    manifest = {"packet": c.PACKET, "files": entries, "hashed_only": hashed_only(),
                "note": "Captures are unedited candidate PNGs, each byte-identical to its baseline (and, where run, "
                        "to its source-free re-render). Derived figures are CPU crops/nearest zooms of those "
                        "captures, not renders. Text files drop only trailing whitespace and a final blank line."}
    (OUT / "manifest.json").write_text(json.dumps(manifest, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"files": len(entries), "bytes": sum(e["bytes"] for e in entries),
                      "normalized": sum(e["normalized"] for e in entries), "hashed_only": len(manifest["hashed_only"])}))


def check():
    manifest = json.loads((OUT / "manifest.json").read_text(encoding="utf-8"))
    problems = []
    for entry in manifest["files"]:
        data = (OUT / entry["path"]).read_bytes()
        source = Path(entry["source"])
        if c.sha_bytes(data) != entry["committed_sha256"]:
            problems.append(f"committed hash {entry['path']}")
        if source.exists() and c.sha(source) != entry["raw_sha256"]:
            problems.append(f"source changed {entry['source']}")
        if source.exists() and committed_bytes(source.read_bytes(), entry["kind"]) != data:
            problems.append(f"normalization {entry['path']}")
    for path, digest in manifest["hashed_only"].items():
        if Path(path).exists() and c.sha(path) != digest:
            problems.append(f"hashed-only changed {path}")
    listed = {e["path"] for e in manifest["files"]} | {"manifest.json"}
    extra = sorted(str(p.relative_to(OUT)) for p in OUT.rglob("*") if p.is_file() and
                   str(p.relative_to(OUT)) not in listed)
    print(json.dumps({"checked": len(manifest["files"]), "hashed_only": len(manifest["hashed_only"]),
                      "problems": problems, "unlisted": extra}))
    if problems or extra:
        raise SystemExit(1)


if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        check()
    elif not sys.argv[1:]:
        build()
    else:
        raise SystemExit(__doc__)
