"""Shared helpers for the 0072 three-run inspection (CPU only; reads published or
captured PNGs by name, searching each run directory recursively).
"""
import sys
from pathlib import Path
sys.dont_write_bytecode = True

def setup(argv):
    """argv: <dir holding 0067 pngio.py> label=dir [label=dir ...]; dirs are searched
    recursively for each PNG name, so published folders and capture trees both work."""
    sys.path.insert(0, argv[0])
    import pngio
    runs = {}
    for pair in argv[1:]:
        label, directory = pair.split("=", 1)
        runs[label] = Path(directory)
    return pngio, runs

def find(directory, name):
    hits = sorted(Path(directory).rglob(name))
    if len(hits) != 1:
        raise SystemExit(f"{name}: expected one file under {directory}, found {len(hits)}")
    return hits[0]

def load(pngio, directory, name):
    w, h, rows = pngio.read(str(find(directory, name)))
    return w, h, b"".join(rows)
