"""Reviewer inspection, CPU only: are the temporal-off reference PNGs byte-identical
across runs?

  python3 -I -B references.py <dir holding 0067 pngio.py> off=<dir> label=<dir> ...
"""
import hashlib, sys
sys.dont_write_bytecode = True
sys.path.insert(0, __import__("os").path.dirname(__file__))
from common_runs import setup, find
pngio, runs = setup(sys.argv[1:])
for name in ("stationary-off.png", "motion-object-off-f8.png", "motion-pan-off-f6.png", "excluded-off.png", "skinned-off-f6.png"):
    digests = {label: hashlib.sha256(find(d, name).read_bytes()).hexdigest()[:16] for label, d in runs.items()}
    print(name, digests, "identical" if len(set(digests.values())) == 1 else "DIFFER")
