"""Review aids for filter-result.md, derived after GPU release from retained
RGBA readbacks only (no new render). Uses the committed build003 review tool
(../../appearance-review/pngtool.py) read-only. Writes create-new PNGs and a
manifest with every source path and hash.

  /opt/homebrew/bin/python3 -I -B make_review.py
"""

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / "appearance-review"))
import pngtool  # noqa: E402

B3 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-003/outputs")
F1 = Path("/Users/andreyyurlov/.codex/incant-evidence/0070-ssao-filter-001/outputs")
SIZE = (321, 193)


def rgba(path):
    data = path.read_bytes()
    assert len(data) == SIZE[0] * SIZE[1] * 4, path
    return data


def side(a, b, gap=6):
    """Horizontal concatenation of two equal-size RGBA images with a white gap."""
    (w, h), da, db = a[0], a[1], b[1]
    out = bytearray()
    for y in range(h):
        out += da[y * w * 4:(y + 1) * w * 4] + bytes((255, 255, 255, 255)) * gap + db[y * w * 4:(y + 1) * w * 4]
    return (2 * w + gap, h), bytes(out)


def darkening(on, off, gain):
    """Gray map of (off - on) scaled by gain; brighter-than-off would show red."""
    out = bytearray()
    for i in range(0, len(on), 4):
        d = sum(off[i + c] - on[i + c] for c in range(3)) / 3
        out += bytes((255, 0, 0, 255)) if d < 0 else bytes((min(255, int(d * gain)),) * 3 + (255,))
    return bytes(out)


SOURCES = {
    "b3_p_ao_white": B3 / "case-ssao_core/p_ao_white.rgba", "f1_p_ao_white": F1 / "case-ssao_core/p_ao_white.rgba",
    "p_off_white": F1 / "case-ssao_core/p_off_white.rgba",
    "b3_p_ao_edge": B3 / "case-ssao_core/p_ao_edge.rgba", "f1_p_ao_edge": F1 / "case-ssao_core/p_ao_edge.rgba",
    "b3_o_ao_white": B3 / "case-ssao_core/o_ao_white.rgba", "f1_o_ao_white": F1 / "case-ssao_core/o_ao_white.rgba",
    "o_off_white": F1 / "case-ssao_core/o_off_white.rgba",
    "f1_p_ao_stripes": F1 / "case-ssao_quality/p_ao_stripes.rgba", "f1_p_off_stripes": F1 / "case-ssao_quality/p_off_stripes.rgba",
    "f1_o_ao_stripes": F1 / "case-ssao_quality/o_ao_stripes.rgba", "f1_o_off_stripes": F1 / "case-ssao_quality/o_off_stripes.rgba",
    "f1_p_off_sphere": F1 / "case-ssao_quality/p_off_sphere.rgba", "f1_p_ao_sphere": F1 / "case-ssao_quality/p_ao_sphere.rgba",
}
OUTPUTS = {
    # name: (description, builder)
    "slab_edge_crop_build003_vs_filter001.png": (
        "white_slab perspective, receiver left of the slab edge, x 110..199 y 40..149, x3: build003 (left) vs filter-001",
        lambda s: side(pngtool.crop(SIZE, s["b3_p_ao_white"], (110, 40, 90, 110), 3),
                       pngtool.crop(SIZE, s["f1_p_ao_white"], (110, 40, 90, 110), 3))),
    "darkening_white_slab_p_build003_vs_filter001.png": (
        "white_slab perspective darkening (off - on) x6, full frame: build003 (left) vs filter-001",
        lambda s: side((SIZE, darkening(s["b3_p_ao_white"], s["p_off_white"], 6)),
                       (SIZE, darkening(s["f1_p_ao_white"], s["p_off_white"], 6)))),
    "darkening_white_slab_o_build003_vs_filter001.png": (
        "white_slab orthographic darkening (off - on) x6: build003 (left) vs filter-001",
        lambda s: side((SIZE, darkening(s["b3_o_ao_white"], s["o_off_white"], 6)),
                       (SIZE, darkening(s["f1_o_ao_white"], s["o_off_white"], 6)))),
    "darkening_edge_slab_p_build003_vs_filter001.png": (
        "edge_slab perspective darkening (off - on) x6: build003 (left) vs filter-001",
        lambda s: side((SIZE, darkening(s["b3_p_ao_edge"], s["p_off_white"], 6)),
                       (SIZE, darkening(s["f1_p_ao_edge"], s["p_off_white"], 6)))),
    "darkening_stripes_p_vs_o_filter001.png": (
        "N8 stripes darkening (off - on) x6, filter-001: perspective (left) vs orthographic",
        lambda s: side((SIZE, darkening(s["f1_p_ao_stripes"], s["f1_p_off_stripes"], 6)),
                       (SIZE, darkening(s["f1_o_ao_stripes"], s["f1_o_off_stripes"], 6)))),
    "stripes_p_roi_crop_off_vs_on.png": (
        "N8 perspective ROI area x 120..189 y 100..159, x4, filter-001: off (left) vs on; ROI row 124, x 148..158",
        lambda s: side(pngtool.crop(SIZE, s["f1_p_off_stripes"], (120, 100, 70, 60), 4),
                       pngtool.crop(SIZE, s["f1_p_ao_stripes"], (120, 100, 70, 60), 4))),
    "sphere_p_off_vs_on.png": (
        "isolated 24x48 sphere perspective, x 118..189 y 62..133, x3, filter-001: off (left) vs on (byte-identical)",
        lambda s: side(pngtool.crop(SIZE, s["f1_p_off_sphere"], (118, 62, 72, 72), 3),
                       pngtool.crop(SIZE, s["f1_p_ao_sphere"], (118, 62, 72, 72), 3))),
}


def main():
    sources = {k: rgba(p) for k, p in SOURCES.items()}
    manifest = {"format": "incant-0070-ssao-filter-001-review-v1",
                "note": "Derived after GPU release from retained RGBA readbacks; not renders and not acceptance evidence.",
                "tool": {"make_review.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                         "pngtool.py": hashlib.sha256((HERE.parent.parent / "appearance-review" / "pngtool.py").read_bytes()).hexdigest()},
                "sources": {k: {"path": str(p), "rgba_sha256": hashlib.sha256(sources[k]).hexdigest()}
                            for k, p in SOURCES.items()},
                "outputs": {}}
    for name, (description, build) in OUTPUTS.items():
        size, data = build(sources)
        path = HERE / name
        if path.exists():
            raise SystemExit(f"{path} exists; review aids are never replaced")
        pngtool.encode(path, size, data)
        manifest["outputs"][name] = {"description": description, "size": list(size),
                                     "png_sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    with open(HERE / "manifest.json", "x") as out:
        json.dump(manifest, out, indent=1, sort_keys=True)
        out.write("\n")
    print(json.dumps({k: v["size"] for k, v in manifest["outputs"].items()}, indent=1))


if __name__ == "__main__":
    main()
