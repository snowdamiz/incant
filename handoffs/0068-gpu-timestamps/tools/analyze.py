#!/usr/bin/env python3
"""CPU-only analysis and evidence publication for handoff 0068. Standard library only.
Never starts a renderer, compiler or GPU child.

  python3 -I -B analyze.py check     summarize the ledger with independent re-checks
  python3 -I -B analyze.py publish   after gpu.py release and lease.py release:
                                     copy receipts and a small screenshot selection
                                     into the packet directory (create-only)

Independent of the Rust helper: PNGs are decoded here (8-bit RGBA, all five
filters) and compared to the helper's RGBA hashes and to each other; expected
timestamp groups are re-derived from the recorded scene statistics; numeric-test
samples are parsed from its stderr and checked for sequence, labels, ordering
and tick-to-nanosecond conversion. Durations are never interpreted as
performance.
"""

import hashlib
import json
import shutil
import struct
import sys
import zlib
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import common as c  # noqa: E402

PASSES = ["InitializeScene", "AssignLights", "Shadows", "Models", "Sprites", "Text", "Display"]
RANK = dict(zip(PASSES, [0, 0, 0, 1, 2, 3, 4]))
# Numeric-test fixture (tests/support fixture): two cooked model instances, no
# lights, sprites or text. The test itself asserts only a subset of this.
NUMERIC_EXPECTED = {"InitializeScene": True, "AssignLights": False, "Shadows": True,
                    "Models": True, "Sprites": False, "Text": False, "Display": True}


def png_rgba(path):
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"{path} is not a PNG")
    pos, idat, size = 8, bytearray(), None
    while pos < len(data):
        length = int.from_bytes(data[pos:pos + 4], "big")
        kind, body = data[pos + 4:pos + 8], data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, color, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if (depth, color, interlace) != (8, 6, 0):
                raise ValueError(f"{path}: expected 8-bit non-interlaced RGBA")
            size = (width, height)
        elif kind == b"IDAT":
            idat += body
        elif kind == b"IEND":
            break
    width, height = size
    raw, stride = zlib.decompress(bytes(idat)), width * 4
    out, previous = bytearray(), bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        for i in range(stride):
            left = line[i - 4] if i >= 4 else 0
            up = previous[i]
            corner = previous[i - 4] if i >= 4 else 0
            if kind == 1:
                line[i] = (line[i] + left) & 255
            elif kind == 2:
                line[i] = (line[i] + up) & 255
            elif kind == 3:
                line[i] = (line[i] + ((left + up) >> 1)) & 255
            elif kind == 4:
                p = left + up - corner
                pa, pb, pc = abs(p - left), abs(p - up), abs(p - corner)
                line[i] = (line[i] + (left if pa <= pb and pa <= pc else up if pb <= pc else corner)) & 255
            elif kind != 0:
                raise ValueError(f"{path}: filter {kind}")
        out += line
        previous = line
    return width, height, bytes(out)


def diff(a, b, width):
    differing, worst, first = 0, 0, None
    for index in range(0, len(a), 4):
        if a[index:index + 4] != b[index:index + 4]:
            differing += 1
            worst = max(worst, max(abs(x - y) for x, y in zip(a[index:index + 4], b[index:index + 4])))
            first = first or [(index // 4) % width, (index // 4) // width]
    return {"equal": a == b, "differing_pixels": differing, "max_channel_delta": worst,
            "first_difference": first}


def expected_groups(stats, local_lights):
    models = stats["model_draw_calls"] > 0
    return {"InitializeScene": True, "AssignLights": models and local_lights > 0, "Shadows": models,
            "Models": models, "Sprites": stats["sprite_entities"] + stats["tilemap_entities"] > 0,
            "Text": stats["text_glyph_quads"] > 0, "Display": True}


def sample_review(spans, frame, period, expected):
    """spans: {pass: {start_ticks, end_ticks, nanoseconds} | None}."""
    present = {name: spans.get(name) is not None for name in PASSES}
    all_spans = [s for s in spans.values() if s] + [frame]
    conversion = all(abs(s["nanoseconds"] - (s["end_ticks"] - s["start_ticks"]) * period)
                     <= 1e-6 * max(1.0, abs(s["nanoseconds"])) for s in all_spans)
    order = [f"{a} > {b}" for a in PASSES for b in PASSES
             if spans.get(a) and spans.get(b) and RANK[a] < RANK[b]
             and spans[a]["end_ticks"] > spans[b]["start_ticks"]]
    return {"labels_match_expected": present == expected,
            "missing": [n for n in PASSES if expected[n] and not present[n]],
            "unexpected": [n for n in PASSES if present[n] and not expected[n]],
            "spans_ordered": all(s["end_ticks"] >= s["start_ticks"] for s in all_spans),
            "conversion_matches_period": conversion,
            "observation_dependency_order_violations": order,
            "observation_within_frame": all(s["start_ticks"] >= frame["start_ticks"]
                                            and s["end_ticks"] <= frame["end_ticks"]
                                            for s in spans.values() if s),
            "observation_zero_tick_groups": [n for n in PASSES if spans.get(n)
                                             and spans[n]["start_ticks"] == spans[n]["end_ticks"]]}


def numeric_review(entry):
    samples = entry.get("samples", [])
    reviews = []
    for sample in samples:
        spans = dict(zip(PASSES, sample["passes"]))
        reviews.append({"sequence": sample["sequence"], "period_ns": sample["timestamp_period_ns"],
                        **sample_review(spans, sample["frame"], sample["timestamp_period_ns"],
                                        NUMERIC_EXPECTED)})
    return {"exit": entry["exit"], "timed_out": entry["timed_out"], "libtest": entry["libtest"],
            "adapter_lines": entry.get("adapter_lines"),
            "sequences": sorted(s["sequence"] for s in samples), "samples": reviews}


def review_review(entry):
    output = Path(entry["output"])
    report_path = output / "report.json"
    if not report_path.exists():
        return {"case": entry["case"], "report": None, "exit": entry["exit"],
                "timed_out": entry["timed_out"]}
    report = c.read_json(report_path)
    decoded, pngs = {}, []
    for png in report.get("pngs", []):
        width, height, rgba = png_rgba(output / png["name"])
        decoded[png["name"]] = (width, rgba)
        pngs.append({"name": png["name"], "size": [width, height],
                     "file_sha256_matches": c.sha(output / png["name"]) == png["sha256"],
                     "decoded_rgba_matches_helper": hashlib.sha256(rgba).hexdigest() == png["rgba_sha256"]})
    pairs = []
    for a, b in c.plan()["totals"]["retained_png_pairs"]:
        if a in decoded and b in decoded:
            pairs.append({"a": a, "b": b, **diff(decoded[a][1], decoded[b][1], decoded[a][0])})
    samples = []
    scene = report.get("scene")
    timing = report.get("timing", {})
    takes = timing.get("drain", []) + timing.get("reuse", []) + timing.get("takes", [])
    if scene:
        expected = expected_groups(scene["stats"], scene["local_lights"])
        for take in takes:
            if take.get("outcome") == "sample":
                raw = take["raw"]
                samples.append({"sequence": raw["sequence"], **sample_review(
                    take["labeled"], raw["frame"], raw["timestamp_period_ns"], expected)})
    return {"case": entry["case"], "exit": entry["exit"], "timed_out": entry["timed_out"],
            "outcome": report.get("outcome"), "checks": report.get("checks"),
            "frames_rendered": report.get("frames_rendered"),
            "timed_submissions": report.get("timed_submissions"),
            "take_outcomes": [t.get("outcome") for t in takes], "pngs": pngs,
            "independent_pairs": pairs, "independent_samples": samples}


def check():
    c.grant()
    entries = [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]
    summary = {"recorded": c.now(), "ledger_sha256": c.sha(c.LEDGER), "children": len(entries),
               "frames_reserved": sum(e.get("frames_reserved", 0) for e in entries),
               "submissions_reserved": sum(e.get("submissions_reserved", 0) for e in entries),
               "builds": [{k: e[k] for k in ("label", "exit", "timed_out", "started", "ended")}
                          for e in entries if e["kind"] == "build"],
               "listings": {e["item"]: {"count": e.get("listed_count"),
                                        "matches_reviewed_base": e.get("matches_reviewed_base")}
                            for e in entries if e["kind"] == "list"},
               "numeric": [numeric_review(e) for e in entries if e["kind"] == "numeric"],
               "suites": {e["item"]: {"exit": e["exit"], "timed_out": e["timed_out"],
                                      "libtest": e["libtest"],
                                      "expected_running": e.get("expected_running"),
                                      "running_matches_listing": e.get("running_matches_listing"),
                                      "skipped_names_absent_from_output":
                                          e.get("skipped_names_absent_from_output")}
                          for e in entries if e["kind"] == "suite"},
               "reviews": [review_review(e) for e in entries if e["kind"] == "review"],
               "all_children_group_empty": all(e.get("group_empty_after", True) for e in entries),
               "binaries_unchanged": all(e.get("binary_sha256_before") == e.get("binary_sha256_after")
                                         for e in entries if e["kind"] != "build")}
    path = c.EVIDENCE / "analysis" / ("summary-" + summary["recorded"].replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({"summary": str(path), "children": summary["children"],
                      "frames_reserved": summary["frames_reserved"],
                      "submissions_reserved": summary["submissions_reserved"]}, indent=1))


def normalized_text(raw):
    """Trailing spaces and blank last lines fail `git diff --check`; strip only those."""
    lines = raw.decode("utf-8", errors="replace").split("\n")
    text = "\n".join(line.rstrip() for line in lines).rstrip("\n") + "\n"
    return text.encode()


def publish():
    c.grant()
    lease = c.read_json(c.LEASE)
    if not c.LATCH.exists() or not lease.get("released_at"):
        c.refuse("publish only after gpu.py release and lease.py release")
    destination = c.PACKET_DIR / "evidence"
    screenshots = c.PACKET_DIR / "screenshots"
    if destination.exists() or screenshots.exists():
        c.refuse("published evidence already exists; it is never replaced")
    entries = [json.loads(line) for line in c.LEDGER.read_text().splitlines() if line.strip()]
    picks = [(c.GRANT, "gpu/grant-copy.json"), (c.LEDGER, "gpu/ledger.jsonl"),
             (c.LATCH, "gpu/gpu-released.json"), (c.LEASE, "lease/target-lease.json")]
    if c.REFUSALS.exists():
        picks.append((c.REFUSALS, "gpu/refusals.jsonl"))
    if c.POST_RELEASE_LEDGER.exists():
        picks.append((c.POST_RELEASE_LEDGER, "gpu/post-release-ledger.jsonl"))
        entries_after = [json.loads(line) for line in c.POST_RELEASE_LEDGER.read_text().splitlines()
                         if line.strip()]
        picks += [(Path(e["stderr"]), "logs/" + Path(e["stderr"]).name) for e in entries_after]
    for name in ("source.json", "toolchain.json", "tests.json", "review.json", "lint.json"):
        if (c.BUILD / name).exists():
            picks.append((c.BUILD / name, "build/" + name))
    for entry in entries:
        picks.append((Path(entry["stderr"]), "logs/" + Path(entry["stderr"]).name))
        if entry["kind"] != "build":
            picks.append((Path(entry["stdout"]), "logs/" + Path(entry["stdout"]).name))
        if entry["kind"] == "review" and entry.get("report_sha256"):
            output = Path(entry["output"])
            for name in ("report.json", "events.jsonl"):
                picks.append((output / name, f"captures/{output.name}/{name}"))
    analyses = sorted((c.EVIDENCE / "analysis").glob("summary-*.json"))
    if analyses:
        picks.append((analyses[-1], "analysis/" + analyses[-1].name))
    manifest = []
    for source, relative in picks:
        raw = source.read_bytes()
        text = relative.endswith((".txt", ".json", ".jsonl"))
        data = normalized_text(raw) if text else raw
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        with open(target, "xb") as out:
            out.write(data)
        manifest.append({"path": relative, "source": str(source), "source_sha256": c.sha_bytes(raw),
                         "sha256": c.sha_bytes(data), "normalized": data != raw})
    # Unedited byte copies, one per distinct PNG within a run; byte-identical
    # timing-off/on frames are represented by one file listing every name.
    selection = []
    for entry in entries:
        if entry["kind"] == "review" and entry.get("pngs"):
            output = Path(entry["output"])
            groups = {}
            for name in c.plan()["review"]["cases"][entry["case"]]["pngs"]:
                if name in entry["pngs"]:
                    groups.setdefault(entry["pngs"][name], []).append(name)
            for digest, names in groups.items():
                target = screenshots / f"{output.name}-{names[0]}"
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(output / names[0], target)
                if c.sha(target) != digest:
                    raise SystemExit(f"screenshot copy mismatch for {target}")
                selection.append({"path": str(target.relative_to(c.PACKET_DIR)),
                                  "represents": names, "source_dir": str(output), "sha256": digest})
    c.write_new_json(destination / "manifest.json", {"published": c.now(), "files": manifest,
                                                      "screenshots": selection})
    print(json.dumps({"files": len(manifest), "screenshots": len(selection)}, indent=1))


if __name__ == "__main__":
    steps = {"check": check, "publish": publish}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
