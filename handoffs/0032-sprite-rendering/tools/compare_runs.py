#!/usr/bin/env python3
"""Before/after comparison of two sprite_lookdev.py runs of the same scene.

Usage (repo root):
  python3 -I handoffs/0032-sprite-rendering/tools/compare_runs.py BEFORE_RUN AFTER_RUN OUT_JSON

BEFORE_RUN and AFTER_RUN render the same scene on two builds with different nearest
rules (each run's rule follows its recorded build source). Every PNG both runs wrote
is compared byte for byte, then pixel by pixel. Each changed pixel is classified:
  "rule delta"  the AFTER scene predicted with the BEFORE rule and with the AFTER
                rule gives different colours there, so the rule change explains it;
  "fixed"       otherwise, AFTER is within tolerance of its own model and closer to it
                than BEFORE was to BEFORE's model;
  "new mismatch" otherwise, the reverse ("new mismatch at a flagged pixel" when the AFTER
                model itself marks the pixel ambiguous: a texel tie or an outer edge);
  "texel tie"   otherwise, either model marks a nearest sample on a texel boundary;
  "edge tie"    otherwise, either model marks a pixel centre on an outer edge;
  "other"       none of these. A sampling-rule change must leave "other" at zero.
The AFTER frame's top sprite and its sampling mode are reported alongside.
OUT_JSON must not exist. Standard library only.
"""

import hashlib
import json
import sys
import time
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyse  # noqa: E402
import pngio  # noqa: E402
import reference  # noqa: E402



def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def frame_spec(rel):
    """(camera, width, height, texture variants) for a static, reimport or source-free PNG."""
    parts = Path(rel).parts
    stem = Path(rel).stem
    if parts[0] == "shots":
        stem = stem.rsplit("-", 1)[0]  # drop -r1
        textures = {}
    elif parts[0] == "reimport":
        step = next(k for k in analyse.REIMPORT_VARIANTS if stem.startswith(k + "-"))
        stem = stem[len(step) + 1:]
        textures = analyse.REIMPORT_VARIANTS[step]
    elif parts[0] == "source-free":
        textures = {}
    else:
        return None
    name, size = stem.rsplit("-", 1)
    w, h = map(int, size.split("x"))
    return name, w, h, textures


def logical_index(view, frame, x, y):
    if view.output is None:
        return y * frame.w + x
    _, _, s, ox, oy = view.output
    lx, ly = (x - ox) // s, (y - oy) // s
    return ly * frame.w + lx if 0 <= lx < frame.w and 0 <= ly < frame.h else None


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    before, after = analyse.Run(sys.argv[1]), analyse.Run(sys.argv[2])
    out_json = Path(sys.argv[3])
    if out_json.exists():
        raise SystemExit("OUT_JSON exists; choose a new path.")
    if before.rule == after.rule:
        raise SystemExit(f"both runs use the {before.rule} rule; nothing to compare")
    t0 = time.time()

    def pngs(run):
        return {str(p.relative_to(run.path)): p for p in sorted(run.path.rglob("*.png"))
                if p.parts[-2] != "art" and "project" not in p.parts and not p.name.endswith("-r2.png")
                and "-r2" not in p.parent.name}
    a, b = pngs(before), pngs(after)
    common = sorted(set(a) & set(b))
    files, totals = {}, Counter()
    for rel in common:
        if sha(a[rel]) == sha(b[rel]):
            totals["identical_files"] += 1
            continue
        ia, ib = pngio.read(a[rel]), pngio.read(b[rel])
        changed = [(x, y) for y in range(ib.height) for x in range(ib.width) if ia.px(x, y) != ib.px(x, y)]
        entry = {"changed_pixels": len(changed)}
        spec = frame_spec(rel)
        if spec is None:
            entry["classified"] = False  # live frames: reported raw
        else:
            name, w, h, textures = spec
            reference.NEAREST_RULE = after.rule
            rows_after, frame_after, view_after = after.render(name, w, h, **textures)
            reference.NEAREST_RULE = before.rule
            rows_switched, _, _ = after.render(name, w, h, **textures)
            rows_before, frame_before, view_before = before.render(name, w, h, **textures)
            owners, classes, tops = Counter(), Counter(), Counter()
            for x, y in changed:
                i = logical_index(view_after, frame_after, x, y)
                j = logical_index(view_before, frame_before, x, y)
                owners[frame_after.owner[i] if i is not None else None] += 1
                tops["linear top" if i is not None and frame_after.top_linear[i] else "nearest or opaque top"] += 1
                delta = rows_after[y][x] != rows_switched[y][x]
                tol = analyse.TOL_LINEAR if i is not None and frame_after.linear[i] else analyse.TOL_NEAREST
                bd = max(abs(p - q) for p, q in zip(ia.rgb(x, y), rows_before[y][x]))
                ad = max(abs(p - q) for p, q in zip(ib.rgb(x, y), rows_after[y][x]))
                texel_tie = (j is not None and frame_before.tie_texel[j]) or (i is not None and frame_after.tie_texel[i])
                edge_tie = (j is not None and frame_before.tie_edge[j]) or (i is not None and frame_after.tie_edge[i])
                flagged = i is not None and frame_after.ambiguous[i]
                classes["rule delta" if delta else "fixed" if ad < bd and ad <= tol
                        else ("new mismatch at a flagged pixel" if flagged else "new mismatch") if bd < ad and bd <= tol
                        else "texel tie" if texel_tie else "edge tie" if edge_tie else "other"] += 1
            entry.update(classified=True, by_class=dict(classes), by_top_sampling=dict(tops),
                         by_top_sprite=dict(owners.most_common()))
            for k, v in classes.items():
                totals[k] += v
        totals["changed_files"] += 1
        totals["changed_pixels"] += len(changed)
        files[rel] = entry
    reference.NEAREST_RULE = after.rule
    result = {"rules": {"before": before.rule, "after": after.rule},
              "before": {"run": before.path.name, **{k: before.helper.get(k) for k in ("binary", "binary_source_commit",
                                                                                        "binary_sha256_start")}},
              "after": {"run": after.path.name, **{k: after.helper.get(k) for k in ("binary", "binary_source_commit",
                                                                                     "binary_sha256_start")}},
              "compared_files": len(common), "only_before": sorted(set(a) - set(b)), "only_after": sorted(set(b) - set(a)),
              "totals": dict(totals), "files": files, "seconds": round(time.time() - t0, 1)}
    out_json.write_text(json.dumps(result, indent=1), encoding="utf-8")
    print(json.dumps({"compared_files": len(common), "totals": dict(totals),
                      "changed": {k: (v["changed_pixels"], v.get("by_class")) for k, v in files.items()},
                      "seconds": result["seconds"]}))


if __name__ == "__main__":
    main()
