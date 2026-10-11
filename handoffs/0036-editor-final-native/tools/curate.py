"""Builds the committed screenshot set from the raw native captures.

Usage: python3 -I curate.py <native helper> <raw dir> <handoff dir>

Full windows are copied unedited. Crops are unscaled 2x pixels, cut at the
accessibility frames recorded during the run (window origin 36,33 pt). Writes
evidence/screenshots.sha256 for every committed image.
"""

import hashlib
import json
import shutil
import subprocess
import sys
from pathlib import Path

helper, raw, handoff = (Path(a) for a in sys.argv[1:4])
out = handoff / "screenshots"
out.mkdir(parents=True, exist_ok=True)

# Unedited window captures: (raw name, committed name).
FULL = [
    ("n10-camera-perspective-1440x874", "normal-camera-perspective-1440x874"),
    ("n45-import-success-focus-1440x874", "normal-import-success-1440x874"),
    ("m10-pixel-camera-1000x650", "minimum-pixel-camera-1000x650"),
    ("m20-account-unavailable-1000x650", "minimum-account-unavailable-1000x650"),
    ("m30-shortcuts-1000x650", "minimum-shortcuts-1000x650"),
    ("m41-atlas-details-narrow-1000x650", "minimum-atlas-narrow-inspector-1000x650"),
]

# Pixel rectangles (x, y, w, h) derived from AX frames minus the window origin, times 2.
INSPECTOR = (2160, 80, 720, 1538)   # Inspector [1116,73,360,769] pt at 1440x874
NARROW = (1440, 80, 560, 1090)      # Inspector [756,73,280,545] pt at 1000x650
ACCOUNT = (1012, 678, 856, 392)     # dialog [546,376,420,188] pt, 4 pt margin
SHORTCUTS = (832, 186, 1216, 1376)  # dialog [456,130,600,680] pt, 4 pt margin
TITLEBAR = (0, 0, 2880, 80)
TITLEBAR_END = (2480, 0, 400, 80)   # account [1400,39,30,28] and shortcuts buttons
LEFT_PANEL = (0, 80, 490, 760)
DOCK = (492, 1218, 1666, 474)
MENU = (0, 300, 640, 560)           # tree rows and the menu [72,321,196,92] pt

CROPS = [
    ("n01-launch-inactive-1440x874", "titlebar-inactive", TITLEBAR),
    ("n02-launch-active-1440x874", "titlebar-active", TITLEBAR),
    ("n23-account-closed-focus-1440x874", "titlebar-end-account-focus", TITLEBAR_END),
    ("n11-camera-orthographic-1440x874", "camera-orthographic-inspector", INSPECTOR),
    ("n12-camera-pixel-perfect-1440x874", "camera-pixel-perfect-inspector", INSPECTOR),
    ("n14-dumbbell-part2-1440x874", "dumbbell-part2-inspector", INSPECTOR),
    ("n15-atlas-sprite-1440x874", "atlas-sprite-inspector", INSPECTOR),
    ("n20-account-unavailable-1440x874", "account-unavailable-dialog", ACCOUNT),
    ("n21-account-close-focus-1440x874", "account-close-focus-dialog", ACCOUNT),
    ("n30-shortcuts-mac-1440x874", "shortcuts-mac-dialog", SHORTCUTS),
    ("n31-shortcuts-filter-hierarchy-1440x874", "shortcuts-filter-hierarchy-dialog", SHORTCUTS),
    ("n40-assets-list-1440x874", "assets-list-before-import", LEFT_PANEL),
    ("n41-atlas-details-1440x874", "atlas-details-inspector", INSPECTOR),
    ("n42-atlas-reimported-focus-1440x874", "atlas-reimported-noop-inspector", INSPECTOR),
    ("n43-import-refused-json-1440x874", "import-refused-json-inspector", INSPECTOR),
    ("n44-import-typed-rows-1440x874", "import-typed-rows-inspector", INSPECTOR),
    ("n52-import-traversal-refused-1440x874", "import-traversal-refused-inspector", INSPECTOR),
    ("n53-import-missing-failure-1440x874", "import-missing-failure-inspector", INSPECTOR),
    ("n47-ordinary-linear-pending-1440x874", "ordinary-linear-pending-inspector", INSPECTOR),
    ("n48-ordinary-reimported-linear-1440x874", "ordinary-reimported-linear-inspector", INSPECTOR),
    ("n50-ogg-details-1440x874", "ogg-details-inspector", INSPECTOR),
    ("n55-after-menu-undo-1440x874", "history-after-menu-undo-dock", DOCK),
    ("n57-context-menu-1440x874", "context-menu-hierarchy", MENU),
    ("m11-pixel-camera-narrow-1000x650", "pixel-camera-narrow-inspector", NARROW),
    ("m13-dumbbell-narrow-1000x650", "dumbbell-narrow-inspector", NARROW),
]

for source, name in FULL:
    shutil.copyfile(raw / f"{source}.png", out / f"{name}.png")
for source, name, (x, y, w, h) in CROPS:
    subprocess.run(
        [str(helper), "crop", str(raw / f"{source}.png"), str(out / f"{name}.png"), str(x), str(y), str(w), str(h)],
        check=True,
        capture_output=True,
    )

lines = []
for path in sorted(out.glob("*.png")):
    lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  screenshots/{path.name}")
for name in ("actions.jsonl", "fixture-before.sha256", "fixture-after.sha256", "native-review.json"):
    path = handoff / "evidence" / name
    if path.exists():
        lines.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  evidence/{name}")
(handoff / "evidence").mkdir(exist_ok=True)
(handoff / "evidence" / "screenshots.sha256").write_text("\n".join(lines) + "\n")
print(json.dumps({"committed": len(lines), "full": len(FULL), "crops": len(CROPS)}))
