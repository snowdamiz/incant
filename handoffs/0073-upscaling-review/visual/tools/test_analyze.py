#!/usr/bin/env python3
"""CPU-only synthetic controls for analyze.py's fail-closed verdict. Standard library only.

It builds, once, a deterministic passing evidence set for the real plan's four
cases: every planned PNG at 960x540, figures assembled from the helper's
documented layout (separator fill, nearest crops, each tile's last row and
column kept as separator), reports carrying exactly the plan's contract checks,
a ledger, debits, build record, grant, released lease and release latch bound
to them. Each control writes a fresh copy into a temporary evidence directory,
changes one thing, and runs the real check/publish functions (and, for exit
codes, the real CLI entry point in a child interpreter). Only the visual-tree
identity is replaced by a test double; nothing outside the temporary directory
is touched.

  python3 -I -B test_analyze.py [--receipt NEW_PATH]
"""

import hashlib
import io
import json
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from contextlib import ExitStack, redirect_stdout
from pathlib import Path
from unittest import mock

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import analyze  # noqa: E402
import common as c  # noqa: E402

W, H = 960, 540
SEPARATOR = bytes([8, 8, 10, 255])
FROZEN = "f" * 64
TREE = "a" * 40
RECORDS = []
CROPS = {"stationary": {"wires": [296, 82, 80, 45], "card_edge": [180, 309, 80, 45],
                        "sign": [648, 191, 80, 45], "sprite": [447, 422, 80, 45],
                        "text": [417, 266, 80, 45]},
         "motion": {"mover": [605, 237, 120, 67], "pan": [378, 197, 120, 67]},
         "transitions": {"strip": [631, 196, 80, 45]}}
# The CLI child: same patched paths and visual-tree double, then the real main().
BOOT = r"""
import json, sys
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
import common as c
for name, value in json.loads(Path(sys.argv[2]).read_text()).items():
    setattr(c, name, Path(value))
c.tools_identity = lambda tree: {"visual_tree": tree, "test_double": True}
import analyze
sys.exit(analyze.main(sys.argv[3:]))
"""


def png(rgba):
    rows = b"".join(b"\0" + rgba[y * W * 4:(y + 1) * W * 4] for y in range(H))
    chunk = lambda kind, body: (struct.pack(">I", len(body)) + kind + body  # noqa: E731
                                + struct.pack(">I", zlib.crc32(kind + body)))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(rows, 1)) + chunk(b"IEND", b""))


def frame(seed):
    out = bytearray(W * H * 4)
    for p in range(W * H):
        x, y = p % W, p // W
        out[p * 4:p * 4 + 4] = bytes([(x * 7 + y * 3 + seed) % 256, (x * y + seed) % 256,
                                      (x ^ y ^ seed) % 256, 255])
    return out


def figure(tiles, zoom):
    """tiles: {slot: (frame, rect)}; nearest crops at `zoom`, separator kept."""
    out = bytearray(SEPARATOR * (W * H))
    for slot, (source, (x0, y0, w, h)) in tiles.items():
        column, row = slot % 4, slot // 4
        for y in range(min(h * zoom, 134)):
            for x in range(min(w * zoom, 239)):
                i = ((y0 + y // zoom) * W + x0 + x // zoom) * 4
                j = ((row * 135 + y) * W + column * 240 + x) * 4
                out[j:j + 4] = source[i:i + 4]
    return out


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Synthetic:
    """A passing evidence set as {relative path: bytes} plus per-case RGBA."""

    def __init__(self):
        p = c.plan()
        self.plan = p
        self.rgba = {}
        full, drs, up = frame(1), frame(40), frame(90)
        left, top, right, bottom = analyze.UI_INTERIOR
        for image in (drs, up):
            for y in range(top, bottom):
                image[(y * W + left) * 4:(y * W + right) * 4] = full[(y * W + left) * 4:(y * W + right) * 4]
        crops = CROPS["stationary"]
        raws = {"stationary-full.png": full, "stationary-drs50.png": drs, "stationary-up50.png": up}
        self.rgba["stationary"] = {
            **raws,
            "stationary-detail.png": figure({4 * row + column: (raws[name], crops[key])
                                             for row, key in enumerate(["wires", "card_edge", "sign", "sprite"])
                                             for column, name in enumerate(raws)}, 3),
            "stationary-phases.png": figure({7: (up, crops["wires"]), 15: (up, crops["text"])}, 3),
            "stationary-activity.png": bytearray(bytes([0, 0, 0, 255]) * (W * H))}
        edge = analyze.edges(bytes(full))
        keep = lambda x, y: not analyze.in_rect_ui(x, y)  # noqa: E731
        self.vs_full = {"drs50": analyze.split(bytes(drs), bytes(full), edge, keep),
                        "up50_frame24": analyze.split(bytes(up), bytes(full), edge, keep)}
        m_full, m_up = frame(7), frame(8)
        pan = CROPS["motion"]["pan"]
        self.rgba["motion"] = {"motion-full.png": m_full, "motion-up50.png": m_up,
                               "motion-detail.png": figure({11: (m_up, pan), 12: (m_full, pan)}, 2)}
        t_full = frame(11)
        inner = analyze.inside(analyze.VIEWPORT_RECT, 0.)
        variants = []
        for seed in (12, 13):
            other, image = frame(seed), bytearray(t_full)
            for q in range(W * H):
                if inner(q % W, q // W):
                    image[q * 4:q * 4 + 4] = other[q * 4:q * 4 + 4]
            variants.append(image)
        strip = CROPS["transitions"]["strip"]
        self.rgba["transitions"] = {
            "transitions-full.png": t_full, "transitions-up50.png": variants[0],
            "transitions-cut.png": variants[1],
            "transitions-strip.png": figure({0: (variants[0], strip), 9: (variants[1], strip),
                                             13: (t_full, strip)}, 3)}
        self.encoded = {case: {name: png(bytes(rgba)) for name, rgba in images.items()}
                        for case, images in self.rgba.items()}

    def write(self, root):
        """Writes the evidence under root; returns the common.py paths to patch."""
        p = self.plan
        e = root / "evidence"
        paths = {"EVIDENCE": e, "GRANT": e / "review-ready.json", "BUILD_GRANT": e / "build-ready.json",
                 "LEASE": e / "target-lease.json", "LATCH": e / "gpu-released.json",
                 "STOP": e / "stop.json", "LEDGER": e / "gpu-ledger.jsonl", "DEBITS": e / "debits.jsonl",
                 "POST_RELEASE_LEDGER": e / "post-release-ledger.jsonl",
                 "REFUSALS": e / "gpu-refusals.jsonl", "LOCK": e / "gpu.lock",
                 "GLOBAL_LOCK": root / "native-gpu.lock", "CHECKOUT": e / "checkout",
                 "BUILD": e / "build", "BIN": e / "bin", "LOGS": e / "logs", "SCRATCH": e / "tmp",
                 "CAPTURES": e / "captures", "PACKET_DIR": root / "packet"}
        for name in ("BUILD", "LOGS", "CAPTURES"):
            paths[name].mkdir(parents=True)
        (paths["BUILD"] / "review.json").write_text(json.dumps({"frozen": {"sha256": FROZEN}}))
        prior = root / "prior-release.json"
        prior.write_text(json.dumps({"owned_processes": [], "ledger_pids_alive": []}))
        paths["GRANT"].write_text(json.dumps({
            "packet": c.PACKET, "stage": c.STAGE, "visual_tree": TREE,
            "build_record_sha256": sha((paths["BUILD"] / "review.json").read_bytes()),
            "prior_synthetic_release_path": str(prior),
            "prior_synthetic_release_sha256": sha(prior.read_bytes())}))
        paths["LEASE"].write_text(json.dumps({"grant_sha256": "b" * 64, "released_at": "synthetic"}))

        def logs(label, text):
            out, err = paths["LOGS"] / f"gpu-{label}.stdout.txt", paths["LOGS"] / f"gpu-{label}.stderr.txt"
            out.write_text(text)
            err.write_text("")
            return {"stdout": str(out), "stdout_sha256": sha(out.read_bytes()),
                    "stderr": str(err), "stderr_sha256": sha(err.read_bytes())}

        common_row = {"exit": 0, "timed_out": False, "interrupted_by": [], "group_empty_after": True,
                      "binary_sha256_before": FROZEN, "binary_sha256_after": FROZEN, "ok": True}
        tests = [p["review"]["cases"][case]["test"] for case in p["review"]["order"]]
        ledger = [{"label": "review", "kind": "build", "exit": 0, "timed_out": False,
                   "interrupted_by": [], **logs("build-review", "{}\n")},
                  {"label": "list-review", "kind": "list", "item": "review", **common_row,
                   **logs("list-review", "".join(f"{t}: test\n" for t in tests)), "listed": tests}]
        debits = [{"label": "list-review", "kind": "list", "item": "review", "frames": 0, "pngs": 0}]
        for case in p["review"]["order"]:
            details = p["review"]["cases"][case]
            output = paths["CAPTURES"] / f"{case}-r1"
            output.mkdir()
            files = self.encoded.get(case, {})
            for name, data in files.items():
                (output / name).write_bytes(data)
            (output / "events.jsonl").write_text('{"event": "open"}\n')
            report = {"case": case, "outcome": "pass", "frames_charged": details["frames"],
                      "checks": {name: True for name in details["checks"]},
                      "pngs": [{"name": name, "sha256": sha(files[name]),
                                "rgba_sha256": sha(bytes(self.rgba[case][name]))} for name in details["pngs"]]}
            if case == "stationary":
                report["crops"] = [{"name": k, "rect": v} for k, v in CROPS["stationary"].items()]
                report["observations"] = {"vs_full": self.vs_full}
            elif case in CROPS:
                report["crops"] = CROPS[case]
            (output / "report.json").write_text(json.dumps(report))
            label = f"review-{case}-r1"
            ledger.append({"label": label, "kind": "review", "item": case, **common_row,
                           **logs(label, "test result: ok. 1 passed; 0 failed\n"), "case": case,
                           "output": str(output), "report_sha256": sha((output / "report.json").read_bytes()),
                           "events_sha256": sha((output / "events.jsonl").read_bytes()),
                           "pngs": {name: sha(data) for name, data in files.items()}})
            debits.append({"label": label, "kind": "review", "item": case,
                           "frames": details["frames"], "pngs": len(details["pngs"])})
        paths["LEDGER"].write_text("".join(json.dumps(row) + "\n" for row in ledger))
        paths["DEBITS"].write_text("".join(json.dumps(row) + "\n" for row in debits))
        rebind(paths)
        return paths


def rebind(paths, **extra):
    """A release latch bound to the current ledger and debits."""
    def digest(path):
        return sha(path.read_bytes()) if path.exists() else None
    paths["LATCH"].write_text(json.dumps({
        "owned_processes": [], "ledger_pids_alive": [], "debits_without_ledger_row": [],
        "stop_latched": False, "ledger_sha256": digest(paths["LEDGER"]),
        "debits_sha256": digest(paths["DEBITS"]), **extra}))


class Analyzer(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.synthetic = Synthetic()

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="incant-0073-analyze-")
        self.root = Path(self.directory.name)
        self.paths = self.synthetic.write(self.root)
        self.stack = ExitStack()
        self.stack.enter_context(mock.patch.multiple(c, **self.paths))
        self.stack.enter_context(mock.patch.object(
            c, "tools_identity", side_effect=lambda tree: {"visual_tree": tree, "test_double": True}))
        self.stack.enter_context(redirect_stdout(io.StringIO()))

    def tearDown(self):
        self.stack.close()
        self.directory.cleanup()

    # -------------------------------------------------------------- helpers

    def ledger(self, change):
        rows = c.read_lines(self.paths["LEDGER"])
        rows = change(rows) or rows
        self.paths["LEDGER"].write_text("".join(json.dumps(row) + "\n" for row in rows))
        rebind(self.paths)

    def row(self, case):
        return next(r for r in c.read_lines(self.paths["LEDGER"]) if r.get("item") == case)

    def report(self, case, change):
        """Edits report.json and keeps every binding consistent except the change."""
        path = Path(self.row(case)["output"]) / "report.json"
        body = json.loads(path.read_text())
        change(body)
        path.write_text(json.dumps(body))

        def bind(rows):
            for r in rows:
                if r.get("item") == case:
                    r["report_sha256"] = sha(path.read_bytes())
        self.ledger(bind)

    def image(self, case, name, offsets, rebind_hashes=True):
        """Inverts bytes of one PNG; optionally re-binds its report and ledger hashes."""
        rgba = bytearray(self.synthetic.rgba[case][name])
        for offset in offsets:
            rgba[offset] ^= 0xFF
        data = png(bytes(rgba))
        path = Path(self.row(case)["output"]) / name
        path.write_bytes(data)
        if rebind_hashes:
            def change(body):
                for item in body["pngs"]:
                    if item["name"] == name:
                        item["sha256"], item["rgba_sha256"] = sha(data), sha(bytes(rgba))
            self.report(case, change)

            def bind(rows):
                for r in rows:
                    if r.get("item") == case:
                        r["pngs"][name] = sha(data)
            self.ledger(bind)

    def summary(self):
        return c.read_json(analyze.latest_summary())

    def failed(self, test, fragments):
        code = analyze.check()
        summary = self.summary()
        found = [f for f in fragments if not any(f in problem for problem in summary["problems"])]
        self.assertEqual(code, 1)
        self.assertEqual((summary["verdict"], summary["complete"]), ("fail", False))
        self.assertEqual(found, [], summary["problems"])
        with self.assertRaisesRegex(SystemExit, "did not pass"):
            analyze.publish()
        self.assertFalse((self.paths["PACKET_DIR"] / "screenshots").exists())
        RECORDS.append({"test": test, "exit": code, "verdict": summary["verdict"],
                        "diagnostic_summary_retained": True, "publish_refused": True,
                        "problems": summary["problems"][:6]})

    # -------------------------------------------------------------- controls

    def test_complete_current_evidence_passes_and_publishes(self):
        code = analyze.check()
        summary = self.summary()
        self.assertEqual((code, summary["verdict"], summary["problems"]), (0, "pass", []))
        tiles = [t for case in summary["cases"].values() for t in case.get("tiles_recut_from_raw", [])]
        self.assertEqual((len(tiles), all(t["matches"] for t in tiles)), (19, True))
        self.assertEqual(analyze.publish(), 0)
        manifest = c.read_json(self.paths["PACKET_DIR"] / "evidence/visual/manifest.json")
        self.assertEqual(len(manifest["screenshots"]), 13)
        self.assertEqual(manifest["analysis"]["path"], analyze.latest_summary().name)
        RECORDS.append({"test": "complete_current_evidence", "exit": code, "tiles": len(tiles),
                        "published_pngs": len(manifest["screenshots"])})

    def test_missing_case(self):
        self.ledger(lambda rows: [r for r in rows if r.get("item") != "motion"])
        rows = [d for d in c.read_lines(self.paths["DEBITS"]) if d.get("item") != "motion"]
        self.paths["DEBITS"].write_text("".join(json.dumps(d) + "\n" for d in rows))
        rebind(self.paths)
        self.failed("missing_case", ["case motion: missing or duplicate review child"])

    def test_missing_report(self):
        (Path(self.row("transitions")["output"]) / "report.json").unlink()
        self.failed("missing_report", ["case transitions: report.json is absent"])

    def test_missing_png(self):
        (Path(self.row("motion")["output"]) / "motion-detail.png").unlink()
        self.failed("missing_png", ["case motion: motion-detail.png is absent",
                                    "case motion: tile motion-detail.png#11"])

    def test_bad_hash(self):
        self.image("stationary", "stationary-detail.png", [0], rebind_hashes=False)
        self.failed("bad_hash", ["stationary-detail.png differs from the ledger hash",
                                 "stationary-detail.png differs from the report hash"])

    def test_false_contract(self):
        self.report("transitions", lambda body: body["checks"].update(frames_nonblank=False))
        self.failed("false_contract", ["contract checks empty or not all true (false: ['frames_nonblank'])"])

    def test_false_tile(self):
        offset = ((1 * 135 + 10) * W + 2 * 240 + 10) * 4  # slot 6: card-edge crop of UP frame 24
        self.image("stationary", "stationary-detail.png", [offset])
        self.failed("false_tile", ["tile stationary-detail.png#6"])

    def test_false_statistic(self):
        self.report("stationary", lambda body: body["observations"]["vs_full"]["drs50"]["interior"]
                    .update(mean_abs=0.0))
        self.failed("false_statistic", ["helper statistic drs50 is not reproduced"])

    def test_false_ui_identity(self):
        offset = (470 * W + 100) * 4
        self.image("stationary", "stationary-up50.png", [offset])
        self.failed("false_ui_identity", ["UI interior differs"])

    def test_false_chrome_identity(self):
        self.image("transitions", "transitions-cut.png", [(5 * W + 5) * 4])
        self.failed("false_chrome_identity", ["viewport chrome differs"])

    def test_failed_child(self):
        def fail(rows):
            for r in rows:
                if r.get("item") == "motion":
                    r.update(ok=False, exit=101)
        self.ledger(fail)
        self.failed("failed_child", ["case motion: the ledger marks the child unsuccessful",
                                     "case motion: child exit 101"])

    def test_replaced_build_record(self):
        (self.paths["BUILD"] / "review.json").write_text(json.dumps({"frozen": {"sha256": FROZEN},
                                                                    "replaced": True}))
        self.failed("replaced_build_record", ["the build record is not the one review-ready.json binds"])

    def test_stop_latch(self):
        self.paths["STOP"].write_text(json.dumps({"reason": "synthetic"}))
        self.failed("stop_latch", ["stop latched"])

    def test_zero_verdict_evidence(self):
        self.paths["LEDGER"].write_text("")
        self.paths["DEBITS"].write_text("")
        rebind(self.paths)
        self.failed("zero_verdict_empty_ledger", ["expected one listing child, found 0",
                                                  "case probe: missing or duplicate review child",
                                                  "case transitions: missing or duplicate review child"])

    def test_zero_verdict_checks(self):
        self.report("probe", lambda body: body.update(checks={}))
        self.failed("zero_verdict_empty_checks", ["case probe: contract checks [] are not exactly",
                                                  "case probe: contract checks empty or not all true"])

    def test_stale_or_superseded_summary_blocks_publication(self):
        with self.assertRaisesRegex(SystemExit, "no analysis exists"):
            analyze.publish()
        self.assertEqual(analyze.check(), 0)
        rebind(self.paths, note="written after the analysis")
        with self.assertRaisesRegex(SystemExit, "is stale"):
            analyze.publish()
        rebind(self.paths)
        self.assertEqual(analyze.check(), 0)
        self.paths["STOP"].write_text(json.dumps({"reason": "synthetic"}))
        self.assertEqual(analyze.check(), 1)
        self.paths["STOP"].unlink()
        with self.assertRaisesRegex(SystemExit, "did not pass"):
            analyze.publish()
        self.assertFalse((self.paths["PACKET_DIR"] / "screenshots").exists())
        RECORDS.append({"test": "stale_or_superseded_summary", "publish_refused_without_analysis": True,
                        "publish_refused_stale": True, "publish_refused_after_newer_failure": True})

    def test_cli_exit_codes(self):
        mapping = self.root / "paths.json"
        mapping.write_text(json.dumps({k: str(v) for k, v in self.paths.items()}))

        def run(step):
            result = subprocess.run([sys.executable, "-I", "-B", "-c", BOOT, str(c.TOOLS), str(mapping), step],
                                    capture_output=True, text=True, timeout=600)
            return result.returncode, result.stderr
        passed = run("check")
        self.image("stationary", "stationary-detail.png", [0], rebind_hashes=False)
        failed = run("check")
        refused = run("publish")
        self.assertEqual(passed[0], 0)
        self.assertEqual(failed[0], 1)
        self.assertEqual(self.summary()["verdict"], "fail")
        self.assertEqual(refused[0], 1)
        self.assertIn("did not pass", refused[1])
        RECORDS.append({"test": "cli_exit_codes", "check_pass": passed[0], "check_fail": failed[0],
                        "publish_refused": refused[0]})


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Analyzer)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if len(sys.argv) == 3 and sys.argv[1] == "--receipt":
        with open(sys.argv[2], "x") as file:
            json.dump({"cpu_only": True, "passed": result.wasSuccessful(), "tests_run": result.testsRun,
                       "failures": len(result.failures), "errors": len(result.errors),
                       "python": sys.version.split()[0], "records": RECORDS}, file, indent=2)
            file.write("\n")
    elif len(sys.argv) != 1:
        raise SystemExit("use no arguments or --receipt NEW_PATH")
    raise SystemExit(0 if result.wasSuccessful() else 1)
