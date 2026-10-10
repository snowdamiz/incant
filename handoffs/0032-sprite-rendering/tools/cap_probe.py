#!/usr/bin/env python3
"""Round five: does the 128 MiB sprite geometry cap fail explicitly, and where?

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/cap_probe.py OUT_DIR [--binary NAME] [--counts 341,342]

Builds a minimal project through the public init/import/RPC interface: one 2048 x 2048
texture and N nearest sprites showing the whole texture, plus a camera. With 64-texel
spans each sprite retains 32 x 32 sub-quads (6,144 vertices of 64 bytes), so 341
sprites fit under 134,217,728 bytes and 342 do not; with 512-texel spans each keeps 96
vertices. For each count, sprites are added by one command transaction and a 64 x 64
screenshot is attempted; the exit status and diagnostic are recorded. OUT_DIR must not
exist. Standard library only.
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pngio  # noqa: E402
import sprite_lookdev as L  # noqa: E402


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("out", type=Path)
    parser.add_argument("--binary", choices=sorted(L.BINARIES), default="active")
    parser.add_argument("--counts", default="341,342")
    args = parser.parse_args()
    L.CLI = L.ROOT / "artifacts/tools" / L.BINARIES[args.binary][0]
    L.BINARY_JSON = L.ROOT / "artifacts/tools" / L.BINARIES[args.binary][1]
    out = args.out.resolve()
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    counts = sorted(int(c) for c in args.counts.split(","))
    start = L.verify_binary()
    project_dir = out / "project"
    (project_dir / "art").mkdir(parents=True)
    project = project_dir / "cap.incant.json"
    journal = project_dir / "cap.incant.journal.jsonl"
    # A 2048 x 2048 texture in four flat quadrants (any content will do; size matters).
    quad = [(60, 90, 120, 255), (120, 60, 90, 255), (90, 120, 60, 255), (150, 150, 150, 255)]
    pngio.write_rgba(project_dir / "art/cap2048.png", 2048, 2048,
                     [quad[(y // 1024) * 2 + x // 1024] for y in range(2048) for x in range(2048)])
    L.engine("init", project, "--name", "Sprite geometry cap", "--entities", "0")
    scene_id = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))
    texture = json.loads(L.engine("import", project, "art/cap2048.png", "--texture-usage", "color").stdout)["asset"]["id"]
    camera = L.new_ulid(1)
    entities = [{"id": camera, "name": "camera", "parent": None,
                 "components": {"Camera": L.camera_value(L.ortho(4.0)), "Transform": L.transform((0, 0, 10.0))}}]
    record = {"binary": args.binary, "binary_sha256_start": start, "counts": {}}
    made, n_id = 0, 1
    for count in counts:
        batch = []
        while made < count:
            n_id += 1
            made += 1
            batch.append({"id": L.new_ulid(n_id), "name": f"sprite{made}", "parent": None, "components": {
                "Transform": L.transform((0.0, 0.0, 0.0)),
                "SpriteRenderer": {"texture": texture, "region": [0, 0, 2048, 2048], "size": [4.0, 4.0],
                                   "pivot": [0.5, 0.5], "tint": [1, 1, 1, 1], "flip_x": False, "flip_y": False,
                                   "layer": 0, "order": 0, "sampling": "nearest", "mode": {"kind": "simple"}}}})
        revision = L.rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
        L.rpc(project, journal, [
            {"id": 2, "method": "command.execute", "params": {
                "commands": [{"op": "create_entity", "scene_id": scene_id, "entity": e} for e in entities + batch],
                "expected_revision": revision, "description": f"Geometry cap probe: {count} sprites (handoff 0032)"}},
            {"id": 3, "method": "project.save"}])
        entities = []
        png = out / f"cap-{count}-sprites-64x64.png"
        L.account(1, 64 * 64 * 4)
        result = L.engine("screenshot", project, png, "--camera", camera, "--width", 64, "--height", 64, check=False)
        record["counts"][str(count)] = {"exit": result.returncode, "image_written": png.exists(),
                                        "sha256": L.sha(png) if png.exists() else None,
                                        "stderr": result.stderr.strip()[-400:]}
    record["binary_sha256_end"] = L.verify_binary()
    record["binary_unchanged_every_call"] = L.CALLS["unchanged"] and start == record["binary_sha256_end"]
    record["engine_calls"] = L.CALLS["count"]
    record["budget"] = dict(L.BUDGET)
    (out / "cap.json").write_text(json.dumps(record, indent=1), encoding="utf-8")
    print(json.dumps(record))


if __name__ == "__main__":
    main()
