#!/usr/bin/env python3
"""CPU-only analysis and evidence publication for the 0073 upscaling visual stage.
Standard library only. Never starts a renderer, compiler or GPU child.

Adapted from handoff 0068's committed, root-reviewed tools/analyze.py (read-only
reuse; its PNG decoder and text normalization are unchanged).

  python3 -I -B analyze.py check     independent re-checks of every completed case
  python3 -I -B analyze.py publish   after gpu.py release and lease.py release:
                                     copy receipts and the retained PNGs (<= 16)
                                     into the packet directory (create-only)

Independent of the Rust helper: every PNG is decoded here and compared with the
helper's file and RGBA hashes; figure tiles that show a retained raw frame are
re-cut from that frame and compared byte for byte; the UI-interior and
viewport-chrome identities are re-checked on the raw frames; and the helper's
FULL-versus-candidate statistics for the retained frames are recomputed. None
of this is an appearance judgment; the PNGs are inspected separately.
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

W, H = 960, 540
TILE = (240, 135)
EDGE_THRESHOLD = 24
# Mirrors of the helper's constants; selfcheck.py proves they match the source.
UI_BUTTON = (24.0, 456.0, 236.0, 516.0)
UI_LABEL = (252.0, 488.0, 600.0, 520.0)
UI_INTERIOR = (30, 462, 230, 510)
VIEWPORT_RECT = (37.5, 21.25, 885.0, 497.5)
# (figure, slot, raw PNG, crop key in report["crops"] or a literal rect source, zoom)
TILES = {
    "stationary": [("stationary-detail.png", 4 * row + column, raw, key, 3)
                   for row, key in enumerate(["wires", "card_edge", "sign", "sprite"])
                   for column, raw in enumerate(["stationary-full.png", "stationary-drs50.png",
                                                 "stationary-up50.png"])]
    + [("stationary-phases.png", 7, "stationary-up50.png", "wires", 3),
       ("stationary-phases.png", 15, "stationary-up50.png", "text", 3)],
    "motion": [("motion-detail.png", 11, "motion-up50.png", "pan", 2),
               ("motion-detail.png", 12, "motion-full.png", "pan", 2)],
    "transitions": [("transitions-strip.png", 0, "transitions-up50.png", "strip", 3),
                    ("transitions-strip.png", 9, "transitions-cut.png", "strip", 3),
                    ("transitions-strip.png", 13, "transitions-full.png", "strip", 3)],
}


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


def crop_zoom(rgba, rect, zoom):
    x0, y0, w, h = rect
    rows = []
    for y in range(h * zoom):
        start = ((y0 + y // zoom) * W + x0) * 4
        row = rgba[start:start + w * 4]
        rows.append(b"".join(row[i * 4:i * 4 + 4] * zoom for i in range(w)))
    return rows, w * zoom


def tile_matches(figure, slot, rows, tile_width):
    """The figure keeps a tile's last column and row as a separator."""
    column, row = slot % 4, slot // 4
    for y in range(min(len(rows), TILE[1] - 1)):
        width = min(tile_width, TILE[0] - 1)
        start = ((row * TILE[1] + y) * W + column * TILE[0]) * 4
        if figure[start:start + width * 4] != rows[y][:width * 4]:
            return False
    return True


def spread(a, i, b, j):
    return max(abs(a[i] - b[j]), abs(a[i + 1] - b[j + 1]), abs(a[i + 2] - b[j + 2]))


def in_rect_ui(x, y):
    return any(r[0] - 2 <= x < r[2] + 2 and r[1] - 2 <= y < r[3] + 2 for r in (UI_BUTTON, UI_LABEL))


def inside(rect, inset):
    def keep(x, y):
        px, py = x + 0.5, y + 0.5
        return (rect[0] + inset <= px < rect[0] + rect[2] - inset
                and rect[1] + inset <= py < rect[1] + rect[3] - inset)
    return keep


def edges(reference):
    mask = bytearray(W * H)
    for y in range(H):
        for x in range(W):
            i = (y * W + x) * 4
            for nx, ny in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                if 0 <= nx < W and 0 <= ny < H and spread(reference, i, reference, (ny * W + nx) * 4) > EDGE_THRESHOLD:
                    mask[y * W + x] = 1
                    break
    return mask


def split(a, b, edge, keep):
    """The helper's split(): mean RGB difference, p99 of the max channel and the
    changed fraction, by control edge mask over the kept pixels."""
    sums, groups = [0.0, 0.0], [[], []]
    for y in range(H):
        for x in range(W):
            if not keep(x, y):
                continue
            i = (y * W + x) * 4
            k = edge[y * W + x]
            total = abs(a[i] - b[i]) + abs(a[i + 1] - b[i + 1]) + abs(a[i + 2] - b[i + 2])
            sums[k] += total / 3.0
            groups[k].append(spread(a, i, b, i))
    result = {}
    for name, k in (("interior", 0), ("edge", 1)):
        values = sorted(groups[k])
        count = len(values)
        result[name] = {"pixels": count, "mean_abs": sums[k] / max(count, 1),
                        "p99_max_channel": values[count * 99 // 100] if count else None,
                        "changed_fraction": sum(1 for v in values if v > 0) / max(count, 1)}
    return result


def same_stats(ours, theirs):
    for part in ("interior", "edge"):
        a, b = ours[part], theirs[part]
        if a["pixels"] != b["pixels"] or a["p99_max_channel"] != b["p99_max_channel"]:
            return False
        for key in ("mean_abs", "changed_fraction"):
            if abs(a[key] - b[key]) > 1e-9 * max(1.0, abs(b[key])):
                return False
    return True


def ui_interior(rgba):
    left, top, right, bottom = UI_INTERIOR
    return b"".join(rgba[(y * W + left) * 4:(y * W + right) * 4] for y in range(top, bottom))


def chrome_pixels():
    r = VIEWPORT_RECT
    return [p for p in range(W * H)
            if (p % W + 0.5 < r[0] - 1 or p % W + 0.5 >= r[0] + r[2] + 1
                or p // W + 0.5 < r[1] - 1 or p // W + 0.5 >= r[1] + r[3] + 1)]


def case_review(entry):
    output = Path(entry["output"])
    report_path = output / "report.json"
    if not report_path.exists():
        return {"case": entry["case"], "report": None, "exit": entry["exit"]}
    report = c.read_json(report_path)
    decoded, pngs = {}, []
    for png in report.get("pngs", []):
        width, height, rgba = png_rgba(output / png["name"])
        decoded[png["name"]] = rgba
        pngs.append({"name": png["name"], "size": [width, height],
                     "file_sha256_matches": c.sha(output / png["name"]) == png["sha256"],
                     "decoded_rgba_matches_helper": hashlib.sha256(rgba).hexdigest() == png["rgba_sha256"]})
    crops = report.get("crops", {})
    if isinstance(crops, list):
        crops = {item["name"]: item["rect"] for item in crops}
    tiles = []
    for figure, slot, raw, key, zoom in TILES.get(entry["case"], []):
        if figure in decoded and raw in decoded and key in crops:
            rows, width = crop_zoom(decoded[raw], crops[key], zoom)
            tiles.append({"figure": figure, "slot": slot, "raw": raw, "crop": key,
                          "matches": tile_matches(decoded[figure], slot, rows, width)})
    extra = {}
    if entry["case"] == "stationary" and {"stationary-full.png", "stationary-drs50.png",
                                          "stationary-up50.png"} <= set(decoded):
        full = decoded["stationary-full.png"]
        edge = edges(full)
        keep = lambda x, y: not in_rect_ui(x, y)  # noqa: E731
        vs = report["observations"]["vs_full"]
        drs = split(decoded["stationary-drs50.png"], full, edge, keep)
        up = split(decoded["stationary-up50.png"], full, edge, keep)
        extra = {"ui_interior_identical_full_drs_up24": len({ui_interior(decoded[n]) for n in (
                     "stationary-full.png", "stationary-drs50.png", "stationary-up50.png")}) == 1,
                 "recomputed_drs50_vs_full": drs, "recomputed_up50_frame24_vs_full": up,
                 "helper_drs50_stats_reproduced": same_stats(drs, vs["drs50"]),
                 "helper_up50_frame24_stats_reproduced": same_stats(up, vs["up50_frame24"])}
    if entry["case"] == "transitions" and {"transitions-full.png", "transitions-up50.png",
                                           "transitions-cut.png"} <= set(decoded):
        full = decoded["transitions-full.png"]
        chrome = chrome_pixels()
        extra = {"chrome_identical_full_up10_up23": all(
            decoded[n][p * 4:p * 4 + 4] == full[p * 4:p * 4 + 4]
            for n in ("transitions-up50.png", "transitions-cut.png") for p in chrome),
            "chrome_pixels": len(chrome)}
    return {"case": entry["case"], "exit": entry["exit"], "timed_out": entry["timed_out"],
            "outcome": report.get("outcome"), "checks": report.get("checks"),
            "frames_charged": report.get("frames_charged"), "pngs": pngs,
            "figure_tiles_recut_from_raw": tiles, **extra}


def check():
    c.grant()
    entries = c.read_lines(c.LEDGER)
    summary = {"recorded": c.now(), "ledger_sha256": c.sha(c.LEDGER) if c.LEDGER.exists() else None,
               "debits_sha256": c.sha(c.DEBITS) if c.DEBITS.exists() else None,
               "children": len(entries),
               "frames_debited": sum(d["frames"] for d in c.read_lines(c.DEBITS)),
               "pngs_debited": sum(d["pngs"] for d in c.read_lines(c.DEBITS)),
               "stop_latched": c.STOP.exists(),
               "builds": [{k: e.get(k) for k in ("label", "exit", "timed_out", "started", "ended")}
                          for e in entries if e["kind"] == "build"],
               "listing": [{"listed": e.get("listed"), "ok": e.get("ok")}
                           for e in entries if e["kind"] == "list"],
               "reviews": [case_review(e) for e in entries if e["kind"] == "review"],
               "all_children_group_empty": all(e.get("group_empty_after", True) for e in entries),
               "binaries_unchanged": all(e.get("binary_sha256_before") == e.get("binary_sha256_after")
                                         for e in entries if e["kind"] != "build")}
    path = c.EVIDENCE / "analysis" / ("summary-" + summary["recorded"].replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({"summary": str(path), "children": summary["children"],
                      "frames_debited": summary["frames_debited"],
                      "pngs_debited": summary["pngs_debited"]}, indent=1))


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
    destination = c.PACKET_DIR / "evidence" / "visual"
    screenshots = c.PACKET_DIR / "screenshots"
    if destination.exists() or screenshots.exists():
        c.refuse("published evidence already exists; it is never replaced")
    entries = c.read_lines(c.LEDGER)
    picks = [(c.GRANT, "gpu/grant-copy.json"), (c.LEDGER, "gpu/ledger.jsonl"),
             (c.DEBITS, "gpu/debits.jsonl"), (c.LATCH, "gpu/gpu-released.json"),
             (c.LEASE, "lease/target-lease.json")]
    for optional, relative in ((c.REFUSALS, "gpu/refusals.jsonl"), (c.STOP, "gpu/stop.json"),
                               (c.POST_RELEASE_LEDGER, "gpu/post-release-ledger.jsonl")):
        if optional.exists():
            picks.append((optional, relative))
    # checkout.json (one row per exported file) stays in the evidence directory.
    for name in ("source.json", "toolchain.json", "review.json", "lint.json"):
        if (c.BUILD / name).exists():
            picks.append((c.BUILD / name, "build/" + name))
    for entry in entries + c.read_lines(c.POST_RELEASE_LEDGER):
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
    selection = []
    for entry in entries:
        if entry["kind"] == "review" and entry.get("pngs"):
            output = Path(entry["output"])
            for name in c.plan()["review"]["cases"][entry["case"]]["pngs"]:
                if name not in entry["pngs"]:
                    continue
                target = screenshots / name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(output / name, target)
                if c.sha(target) != entry["pngs"][name]:
                    raise SystemExit(f"screenshot copy mismatch for {target}")
                selection.append({"path": str(target.relative_to(c.PACKET_DIR)),
                                  "source": str(output / name), "sha256": entry["pngs"][name]})
    if len(selection) > c.plan()["budgets"]["pngs_max"]:
        raise SystemExit("more PNGs than the packet allows")
    c.write_new_json(destination / "manifest.json", {"published": c.now(), "files": manifest,
                                                      "screenshots": selection})
    print(json.dumps({"files": len(manifest), "screenshots": len(selection)}, indent=1))


if __name__ == "__main__":
    steps = {"check": check, "publish": publish}
    if len(sys.argv) != 2 or sys.argv[1] not in steps:
        raise SystemExit(__doc__)
    steps[sys.argv[1]]()
