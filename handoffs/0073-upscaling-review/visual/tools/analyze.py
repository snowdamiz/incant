#!/usr/bin/env python3
"""CPU-only, fail-closed analysis and evidence publication for the 0073 upscaling
visual stage. Standard library only. Never starts a renderer, compiler or GPU child.

Adapted from handoff 0068's committed, root-reviewed tools/analyze.py (read-only
reuse; its PNG decoder and text normalization are unchanged apart from a fast
path for unfiltered rows).

  python3 -I -B analyze.py check     validate the complete current evidence; always
                                     writes analysis/summary-<time>.json and exits 0
                                     only when every requirement below holds
  python3 -I -B analyze.py publish   copy receipts and the plan's PNGs (<= 16) into
                                     the packet (create-only), only from the latest
                                     summary, which must pass and still bind the
                                     current evidence

`check` passes only if all of these hold; none is optional, and absence fails:
- the GPU grant and the committed visual tree it names (as every GPU step), and
  the build record is still the one the grant binds by SHA-256;
- no stop latch; the build lease is released; the GPU release latch exists,
  lists no owned or live process or orphan debit, and binds the current ledger
  and debits;
- ledgered builds succeeded and their logs match; exactly one successful listing naming exactly the
  plan's tests; exactly one successful review child per plan case, in plan order,
  and no other GPU child; one debit per GPU child, each case debited its plan
  frames and PNGs, totals within the plan budgets;
- per case: exit 0, no timeout, interrupt or exec error, empty process group, and
  the build record's binary before and after; the ledger's log, report, events
  and PNG hashes equal the files now; the output directory holds exactly
  report.json, events.jsonl and the plan's PNGs; report.json names the case, says
  pass, lists exactly the plan's contract checks, all true, and charged exactly
  the plan's frames; every PNG is 960x540 with file and decoded-RGBA hashes equal
  to the report's;
- the independent comparisons each case defines: figure tiles re-cut from the
  retained raw frames, the stationary UI-interior identity and reproduced
  FULL-versus-variant statistics, and the transitions chrome identity.
Image differences, activity and wire presence stay observations in report.json:
they are never thresholds and never decide the verdict. Appearance is judged
only by inspecting the PNGs.
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
# (figure, slot, raw PNG, crop key in report["crops"], zoom)
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
STATIONARY_RAW = ("stationary-full.png", "stationary-drs50.png", "stationary-up50.png")
TRANSITION_RAW = ("transitions-full.png", "transitions-up50.png", "transitions-cut.png")


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
    if size is None:
        raise ValueError(f"{path}: no IHDR")
    width, height = size
    raw, stride = zlib.decompress(bytes(idat)), width * 4
    out, previous = bytearray(), bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1:start + 1 + stride])
        if kind == 0:  # unfiltered: bytes are final
            out += line
            previous = line
            continue
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
            else:
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
    if not isinstance(theirs, dict):
        return False
    for part in ("interior", "edge"):
        a, b = ours[part], theirs.get(part)
        if not isinstance(b, dict) or a["pixels"] != b.get("pixels") \
                or a["p99_max_channel"] != b.get("p99_max_channel"):
            return False
        for key in ("mean_abs", "changed_fraction"):
            if not isinstance(b.get(key), (int, float)) \
                    or abs(a[key] - b[key]) > 1e-9 * max(1.0, abs(b[key])):
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


# ------------------------------------------------------------------ validation

def digest(path):
    path = Path(path) if path else None
    return c.sha(path) if path is not None and path.is_file() else None


def bindings(entries, visual_tree):
    """Current hashes of everything a verdict depends on. A summary binds these;
    publication recomputes them, so any later change makes the summary stale."""
    files = {}
    for entry in entries:
        for key in ("stdout", "stderr"):
            if entry.get(key):
                files[entry[key]] = digest(entry[key])
        if entry.get("kind") == "review" and entry.get("output"):
            output = Path(entry["output"])
            files[str(output)] = "directory" if output.is_dir() else None
            if output.is_dir():
                for path in sorted(output.iterdir()):
                    files[str(path)] = digest(path) if path.is_file() else "directory"
    return {"visual_tree": visual_tree, "plan": digest(c.PLAN_PATH), "grant": digest(c.GRANT),
            "build_record": digest(c.BUILD / "review.json"), "lease": digest(c.LEASE),
            "ledger": digest(c.LEDGER), "debits": digest(c.DEBITS), "latch": digest(c.LATCH),
            "stop": digest(c.STOP), "files": files}


def rect_ok(rect, zoom):
    return (isinstance(rect, list) and len(rect) == 4 and all(isinstance(v, int) and v >= 0 for v in rect)
            and rect[0] + rect[2] <= W and rect[1] + rect[3] <= H
            and rect[2] * zoom <= TILE[0] and rect[3] * zoom <= TILE[1])


def validate_case(entry, details, frozen_sha, p):
    """One review child against the plan; returns (detail, problems)."""
    problems = []

    def need(condition, message):
        if not condition:
            problems.append(message)
        return bool(condition)

    case = entry.get("item")
    need(entry.get("ok") is True, "the ledger marks the child unsuccessful")
    need(entry.get("exit") == 0 and entry.get("timed_out") is False and entry.get("interrupted_by") == []
         and entry.get("group_empty_after") is True and "launch_error" not in entry,
         f"child exit {entry.get('exit')}, timeout {entry.get('timed_out')}, interrupts "
         f"{entry.get('interrupted_by')}, group empty {entry.get('group_empty_after')}")
    need(frozen_sha is not None and entry.get("binary_sha256_before") == frozen_sha
         and entry.get("binary_sha256_after") == frozen_sha,
         "the child did not run the build record's frozen binary unchanged")
    for key in ("stdout", "stderr"):
        need(entry.get(key) and entry.get(key + "_sha256")
             and digest(entry[key]) == entry[key + "_sha256"], f"{key} log differs from the ledger")
    output = Path(entry.get("output") or "/nonexistent-0073-output")
    report_path = output / "report.json"
    if not need(report_path.is_file(), "report.json is absent"):
        return {"report": None}, problems
    need(entry.get("report_sha256") and digest(report_path) == entry["report_sha256"],
         "report.json differs from the ledger")
    need(entry.get("events_sha256") and digest(output / "events.jsonl") == entry["events_sha256"],
         "events.jsonl is absent or differs from the ledger")
    report = c.read_json(report_path)
    checks = report.get("checks")
    expected_checks = sorted(details["checks"])
    need(report.get("case") == case, f"report.json names case {report.get('case')}")
    need(report.get("outcome") == "pass", f"report outcome is {report.get('outcome')}")
    need(isinstance(checks, dict) and sorted(checks) == expected_checks,
         f"contract checks {sorted(checks) if isinstance(checks, dict) else checks} are not "
         f"exactly the plan's {expected_checks}")
    false = sorted(k for k, v in (checks.items() if isinstance(checks, dict) else []) if v is not True)
    need(isinstance(checks, dict) and checks and not false,
         f"contract checks empty or not all true (false: {false})")
    need(report.get("frames_charged") == details["frames"],
         f"charged {report.get('frames_charged')} frames, plan {details['frames']}")
    present = sorted(path.name for path in output.iterdir()) if output.is_dir() else []
    expected_files = sorted(["events.jsonl", "report.json", *details["pngs"]])
    need(present == expected_files, f"output holds {present}, plan {expected_files}")
    report_pngs = {png.get("name"): png for png in report.get("pngs", []) if isinstance(png, dict)}
    ledger_pngs = entry.get("pngs") if isinstance(entry.get("pngs"), dict) else {}
    need(sorted(report_pngs) == sorted(details["pngs"]), "report.json PNG list differs from the plan")
    need(sorted(ledger_pngs) == sorted(details["pngs"]), "ledger PNG list differs from the plan")
    decoded, pngs = {}, []
    for name in details["pngs"]:
        path = output / name
        if not need(path.is_file(), f"{name} is absent"):
            continue
        file_sha = digest(path)
        row = {"name": name, "file_sha256": file_sha,
               "matches_ledger": file_sha == ledger_pngs.get(name),
               "matches_report": file_sha == report_pngs.get(name, {}).get("sha256")}
        need(row["matches_ledger"], f"{name} differs from the ledger hash")
        need(row["matches_report"], f"{name} differs from the report hash")
        try:
            width, height, rgba = png_rgba(path)
        except (ValueError, zlib.error, struct.error) as error:
            need(False, f"{name} does not decode: {error}")
            pngs.append(row)
            continue
        row["size"] = [width, height]
        row["rgba_matches_report"] = (hashlib.sha256(rgba).hexdigest()
                                      == report_pngs.get(name, {}).get("rgba_sha256"))
        need(row["size"] == p["budgets"]["png_size"], f"{name} is {width}x{height}")
        need(row["rgba_matches_report"], f"{name} decoded RGBA differs from the report")
        decoded[name] = rgba
        pngs.append(row)
    crops = report.get("crops", {})
    if isinstance(crops, list):
        crops = {item.get("name"): item.get("rect") for item in crops if isinstance(item, dict)}
    tiles = []
    for figure, slot, raw, key, zoom in TILES.get(case, []):
        label = f"tile {figure}#{slot} ({raw}, crop {key})"
        if not need(figure in decoded and raw in decoded and isinstance(crops, dict)
                    and rect_ok(crops.get(key), zoom), f"{label} cannot be re-cut"):
            continue
        rows, width = crop_zoom(decoded[raw], crops[key], zoom)
        matches = tile_matches(decoded[figure], slot, rows, width)
        need(matches, f"{label} differs from its raw frame")
        tiles.append({"figure": figure, "slot": slot, "raw": raw, "crop": key, "matches": matches})
    detail = {"report_sha256": digest(report_path), "outcome": report.get("outcome"),
              "checks": checks, "frames_charged": report.get("frames_charged"), "pngs": pngs,
              "tiles_recut_from_raw": tiles}
    if case == "stationary" and need(all(n in decoded for n in STATIONARY_RAW),
                                     "stationary raw frames missing for UI/statistics checks"):
        full = decoded["stationary-full.png"]
        detail["ui_interior_identical_full_drs_up24"] = len(
            {ui_interior(decoded[n]) for n in STATIONARY_RAW}) == 1
        need(detail["ui_interior_identical_full_drs_up24"], "UI interior differs between raw frames")
        vs = ((report.get("observations") or {}).get("vs_full") or {})
        edge = edges(full)
        keep = lambda x, y: not in_rect_ui(x, y)  # noqa: E731
        for key, name in (("drs50", "stationary-drs50.png"), ("up50_frame24", "stationary-up50.png")):
            reproduced = same_stats(split(decoded[name], full, edge, keep), vs.get(key))
            detail[f"{key}_vs_full_statistics_reproduced"] = reproduced
            need(reproduced, f"helper statistic {key} is not reproduced from the raw frames")
    if case == "transitions" and need(all(n in decoded for n in TRANSITION_RAW),
                                      "transition raw frames missing for the chrome check"):
        full = decoded["transitions-full.png"]
        chrome = chrome_pixels()
        detail["chrome_identical_full_up10_up23"] = all(
            decoded[n][q * 4:q * 4 + 4] == full[q * 4:q * 4 + 4]
            for n in TRANSITION_RAW[1:] for q in chrome)
        need(detail["chrome_identical_full_up10_up23"], "viewport chrome differs between raw frames")
    return detail, problems


def validate(entries, debits, grant):
    """Every requirement in the module docstring; returns (cases, problems)."""
    p = c.plan()
    spec = p["review"]
    order = spec.get("order") or []
    problems = []

    def need(condition, message):
        if not condition:
            problems.append(message)
        return bool(condition)

    need(order and sorted(order) == sorted(spec["cases"]), "the plan has no cases or its order differs")
    if c.STOP.exists():
        need(False, "stop latched: " + json.dumps(c.read_json(c.STOP))[:300])
    lease = c.read_json(c.LEASE) if c.LEASE.is_file() else {}
    need(lease.get("released_at"), "the CPU build lease is absent or still held")
    latch = c.read_json(c.LATCH) if c.LATCH.is_file() else None
    if need(latch is not None, "the GPU release latch is absent; run gpu.py release first"):
        need(latch.get("owned_processes") == [] and latch.get("ledger_pids_alive") == [],
             "the release latch lists owned or live processes")
        need(latch.get("debits_without_ledger_row") == [], "the release latch lists orphan debits")
        need(latch.get("ledger_sha256") == digest(c.LEDGER) and latch.get("ledger_sha256"),
             "the release latch does not bind the current ledger")
        need(latch.get("debits_sha256") == digest(c.DEBITS) and latch.get("debits_sha256"),
             "the release latch does not bind the current debits")
        need(latch.get("stop_latched") is False, "the release latch recorded a stop latch")
    record = c.BUILD / "review.json"
    frozen_sha = c.read_json(record).get("frozen", {}).get("sha256") if record.is_file() else None
    need(frozen_sha, "the build record or its frozen binary hash is absent")
    need(digest(record) == grant.get("build_record_sha256"),
         "the build record is not the one review-ready.json binds")
    builds = [e for e in entries if e.get("kind") == "build"]
    need(all(e.get("exit") == 0 and not e.get("timed_out") and not e.get("interrupted_by")
             and "launch_error" not in e for e in builds), "a ledgered build did not succeed")
    need(all(e.get(key) and e.get(key + "_sha256") and digest(e[key]) == e[key + "_sha256"]
             for e in builds for key in ("stdout", "stderr")), "a build log differs from the ledger")
    gpu = [e for e in entries if e.get("kind") != "build"]
    need(all(e.get("kind") in ("list", "review") for e in gpu), "the ledger holds an unknown child kind")
    listings = [e for e in gpu if e.get("kind") == "list"]
    tests = sorted(spec["cases"][case]["test"] for case in order if case in spec["cases"])
    if need(len(listings) == 1, f"expected one listing child, found {len(listings)}"):
        need(listings[0].get("ok") is True and sorted(listings[0].get("listed") or []) == tests,
             "the listing failed or did not name exactly the plan's tests")
    reviews = [e for e in gpu if e.get("kind") == "review"]
    need([e.get("item") for e in reviews] == order,
         f"review children {[e.get('item') for e in reviews]} are not the plan cases {order} once each in order")
    labels = [e.get("label") for e in gpu]
    need(len(debits) == len(gpu) and sorted(d.get("label") for d in debits) == sorted(labels),
         "debits and GPU ledger rows are not one to one")
    budgets = p["budgets"]
    need(sum(d.get("frames", 0) for d in debits) <= budgets["frames_max"]
         and sum(d.get("pngs", 0) for d in debits) <= budgets["pngs_max"], "debits exceed the plan budgets")
    cases = {}
    for case in order:
        details = spec["cases"].get(case, {})
        charged = [d for d in debits if d.get("item") == case]
        need(len(charged) == 1 and charged[0].get("frames") == details.get("frames")
             and charged[0].get("pngs") == len(details.get("pngs", [])),
             f"case {case}: not exactly one debit of the plan's frames and PNGs")
        rows = [e for e in reviews if e.get("item") == case]
        if not need(len(rows) == 1, f"case {case}: missing or duplicate review child ({len(rows)} rows)"):
            cases[case] = {"present": False, "rows": len(rows)}
            continue
        detail, case_problems = validate_case(rows[0], details, frozen_sha, p)
        cases[case] = {"present": True, **detail}
        problems.extend(f"case {case}: {message}" for message in case_problems)
    return cases, problems


def check():
    data, _ = c.grant()
    tools = c.tools_identity(data["visual_tree"])
    try:
        entries, debits = c.read_lines(c.LEDGER), c.read_lines(c.DEBITS)
    except (OSError, ValueError) as error:
        entries, debits, unreadable = [], [], repr(error)
    else:
        unreadable = None
    try:
        cases, problems = validate(entries, debits, data)
    except Exception as error:  # a malformed record is a failed analysis, never a pass
        cases, problems = {}, [f"analysis error: {error!r}"]
    if unreadable:
        problems.insert(0, "ledger or debits unreadable: " + unreadable)
    recorded = c.now()
    summary = {"recorded": recorded, "verdict": "fail" if problems else "pass",
               "complete": not problems, "problems": problems, "cases": cases,
               "bindings": bindings(entries, tools["visual_tree"]),
               "observations": "report.json observations are reference data; they never decide the verdict"}
    path = c.EVIDENCE / "analysis" / ("summary-" + recorded.replace(":", "") + ".json")
    c.write_new_json(path, summary)
    print(json.dumps({"summary": str(path), "verdict": summary["verdict"],
                      "problems": problems[:20]}, indent=1))
    return 0 if not problems else 1


def latest_summary():
    summaries = sorted((c.EVIDENCE / "analysis").glob("summary-*.json"))
    return summaries[-1] if summaries else None


def normalized_text(raw):
    """Trailing spaces and blank last lines fail `git diff --check`; strip only those."""
    lines = raw.decode("utf-8", errors="replace").split("\n")
    text = "\n".join(line.rstrip() for line in lines).rstrip("\n") + "\n"
    return text.encode()


def publish():
    data, _ = c.grant()
    tools = c.tools_identity(data["visual_tree"])
    latest = latest_summary()
    if latest is None:
        c.refuse("no analysis exists; run analyze.py check first")
    summary = c.read_json(latest)
    if summary.get("verdict") != "pass" or summary.get("problems") or summary.get("complete") is not True:
        c.refuse(f"the latest analysis {latest.name} did not pass; publication needs a complete "
                 "passing analysis")
    entries = c.read_lines(c.LEDGER)
    if summary.get("bindings") != bindings(entries, tools["visual_tree"]):
        c.refuse(f"the latest analysis {latest.name} is stale: the grant, build record, lease, "
                 "ledger, debits, latches, plan, visual tree or a case file changed since it ran")
    destination = c.PACKET_DIR / "evidence" / "visual"
    screenshots = c.PACKET_DIR / "screenshots"
    if destination.exists() or screenshots.exists():
        c.refuse("published evidence already exists; it is never replaced")
    picks = [(c.GRANT, "gpu/grant-copy.json"), (c.LEDGER, "gpu/ledger.jsonl"),
             (c.DEBITS, "gpu/debits.jsonl"), (c.LATCH, "gpu/gpu-released.json"),
             (c.LEASE, "lease/target-lease.json"), (latest, "analysis/" + latest.name)]
    if c.BUILD_GRANT.exists():
        picks.append((c.BUILD_GRANT, "build/build-grant-copy.json"))
    for optional, relative in ((c.REFUSALS, "gpu/refusals.jsonl"),
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
        if entry["kind"] == "review":
            output = Path(entry["output"])
            for name in ("report.json", "events.jsonl"):
                picks.append((output / name, f"captures/{output.name}/{name}"))
    manifest = []
    for source, relative in picks:
        raw = source.read_bytes()
        text = relative.endswith((".txt", ".json", ".jsonl"))
        data_bytes = normalized_text(raw) if text else raw
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        with open(target, "xb") as out:
            out.write(data_bytes)
        manifest.append({"path": relative, "source": str(source), "source_sha256": c.sha_bytes(raw),
                         "sha256": c.sha_bytes(data_bytes), "normalized": data_bytes != raw})
    selection = []
    for entry in entries:
        if entry["kind"] != "review":
            continue
        output = Path(entry["output"])
        for name in c.plan()["review"]["cases"][entry["item"]]["pngs"]:
            target = screenshots / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(output / name, target)
            if c.sha(target) != entry["pngs"][name]:
                raise SystemExit(f"screenshot copy mismatch for {target}")
            selection.append({"path": str(target.relative_to(c.PACKET_DIR)),
                              "source": str(output / name), "sha256": entry["pngs"][name]})
    if len(selection) > c.plan()["budgets"]["pngs_max"]:
        raise SystemExit("more PNGs than the packet allows")
    c.write_new_json(destination / "manifest.json", {
        "published": c.now(), "analysis": {"path": latest.name, "sha256": c.sha(latest)},
        "files": manifest, "screenshots": selection})
    print(json.dumps({"files": len(manifest), "screenshots": len(selection),
                      "analysis": latest.name}, indent=1))
    return 0


def main(argv):
    steps = {"check": check, "publish": publish}
    if len(argv) != 1 or argv[0] not in steps:
        raise SystemExit(__doc__)
    return steps[argv[0]]()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
