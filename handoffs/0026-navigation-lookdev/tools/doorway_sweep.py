#!/usr/bin/env python3
"""Doorway-width sweep for handoff 0026 (diagnostic; no GPU capture).

For each width, builds a new project through the public CLI (init, one rpc
command.execute, project.save, validate), with an 8 x 6 m box floor and a 0.3 m
partition at x = 0 whose single doorway spans z in [-w/2, w/2]. Every piece is a
`collider` navigation source and the navigation settings equal the look-dev
scene's. `play --ticks 1` runs doorway_probe.ts, which asks for a route from
(-3, 0, 0) to (3, 0, 0). Reports, per width, whether a route exists and its
clearance to the doorway jambs.

Usage (repo root):
  python3 -I handoffs/0026-navigation-lookdev/tools/doorway_sweep.py OUT_DIR [WIDTH ...]
OUT_DIR must not exist.
"""

import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import navigation_lookdev as nl  # noqa: E402  (same-directory helper; no third-party code)

PROBE = Path(__file__).resolve().with_name("doorway_probe.ts")


def build(out, width):
    out.mkdir(parents=True)
    project = out / "doorway.incant.json"
    journal = project.with_suffix(".journal.jsonl")
    nl.run(nl.CLI, "init", project, "--name", f"Doorway {width}", "--entities", "0")
    scene = next(iter(json.loads(project.read_text(encoding="utf-8"))["scenes"]))
    half, t = width / 2, nl.WALL_T / 2
    boxes = [("Floor", (4.0, 0.1, 3.0), (0, -0.1, 0)),
             ("Wall north", (t, 0.5, (3.0 - half) / 2), (0, 0.5, -(3.0 + half) / 2)),
             ("Wall south", (t, 0.5, (3.0 - half) / 2), (0, 0.5, (3.0 + half) / 2))]
    entities, sources = [], []
    for i, (name, ext, center) in enumerate(boxes):
        eid = nl.new_ulid(500 + i)
        entities.append({"id": eid, "name": name, "parent": None, "components": {
            "Transform": nl.transform(center),
            "Collider": nl.collider({"type": "box", "half_extents": list(ext)})}})
        sources.append({"entity": eid, "geometry": "collider"})
    settings = dict(nl.NAVIGATION, min=[-4, -1, -3], max=[4, 2.5, 3])
    entities.append({"id": nl.new_ulid(599), "name": "Navigation", "parent": None,
                     "components": {"NavigationMesh": {"settings": settings, "sources": sources}}})
    revision = nl.rpc(project, journal, [{"id": 1, "method": "project.read"}])[0]["result"]["revision"]
    for reply in nl.rpc(project, journal, [
        {"id": 2, "method": "command.execute", "params": {
            "commands": [{"op": "create_entity", "scene_id": scene, "entity": e} for e in entities],
            "expected_revision": revision, "description": f"Doorway {width} m"}},
        {"id": 3, "method": "project.save"}]):
        if "error" in reply:
            raise SystemExit(f"RPC error: {reply['error']}")
    nl.run(nl.CLI, "validate", project)
    return project


def main():
    out = Path(sys.argv[1]).resolve()
    widths = [float(w) for w in sys.argv[2:]] or [0.8, 0.9, 1.0, 1.1, 1.2, 1.3, 1.4, 1.6]
    if out.exists():
        raise SystemExit("Output directory already exists; choose a new directory.")
    before = nl.verify_binary()
    out.mkdir(parents=True)
    script = out / "doorway_probe.js"
    nl.run("node", nl.ROOT / "tools/build_script.mjs", PROBE, script)
    rows = []
    for width in widths:
        project = build(out / f"w{width:.2f}", width)
        logs = out / f"w{width:.2f}.logs.jsonl"
        nl.run(nl.CLI, "play", project, "--ticks", 1, "--compiled-script", script, "--log-output", logs)
        event = json.loads(json.loads(logs.read_text(encoding="utf-8").splitlines()[0])["message"])
        points = event["path"]
        row = {"width": width, "route": points is not None,
               "free_width_needed_for_radius": round(width - 2 * nl.NAVIGATION["agent_radius"], 3)}
        if points:
            # Clearance from the route to the nearest jamb corner (x = +-0.15, z = +-w/2).
            jambs = [(sx * nl.WALL_T / 2, sz * width / 2) for sx in (-1, 1) for sz in (-1, 1)]
            best = math.inf
            for a, b in zip(points, points[1:]):
                for k in range(201):
                    x, z = a[0] + (b[0] - a[0]) * k / 200, a[2] + (b[2] - a[2]) * k / 200
                    if abs(x) <= nl.WALL_T / 2:
                        best = min(best, width / 2 - abs(z))
                    best = min(best, *(math.hypot(x - jx, z - jz) for jx, jz in jambs))
            row["route_clearance_to_jambs"] = round(best, 3)
            row["points"] = points
        rows.append(row)
        print(json.dumps({k: v for k, v in row.items() if k != "points"}))
    after = nl.verify_binary()
    (out / "sweep.json").write_text(json.dumps({"binary_sha256_before": before, "binary_sha256_after": after,
                                                "settings": nl.NAVIGATION, "rows": rows}, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
