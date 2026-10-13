"""Reviewer inspection aid, CPU only: side-by-side magnified crops, one panel per run
(magenta separators). The crops it writes are for viewing and are not committed.

  python3 -I -B crops.py <output dir> <name>:<on.png>:<off.png>:x0,y0,x1,y1:<scale> <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
from pathlib import Path
out = Path(sys.argv[1]); spec = sys.argv[2]  # name:onfile:x0,y0,x1,y1:scale
pngio, runs = setup(sys.argv[3:])
name, onfile, offfile, box, scale = spec.split(":")
x0, y0, x1, y1 = map(int, box.split(",")); scale = int(scale)
panels = []
for label, d in runs.items():
    file = offfile if label == "off" else onfile
    w, h, rows = pngio.read(str(find(d, file)))
    panel = []
    for y in range(y0, y1):
        line = bytearray()
        for x in range(x0, x1):
            line += rows[y][x*4:x*4+4] * scale
        panel.extend([bytes(line)] * scale)
    panels.append(panel)
rows = [b"".join((bytes([255, 0, 255, 255]) * 4 if i else b"") + p[y] for i, p in enumerate(panels)) for y in range(len(panels[0]))]
pngio.write(str(out / f"{name}.png"), len(rows[0]) // 4, len(rows), rows)
print(name, len(rows[0]) // 4, "x", len(rows))
