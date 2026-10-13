"""CPU controls for real Cargo unit-test/ordinary-library artifact differences."""
import copy
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from build import library_linkage

unit = {"target": "incant_render", "kind": ["lib"], "test": True,
        "executable": "/unit-test", "features": ["default", "physics-deterministic"],
        "package_id": "path+same-source#incant_render", "filenames": {"/unit-test": "test-hash"}}
lib = {**unit, "test": False, "executable": None, "filenames": {"/lib.rlib": "lib-hash"}}
r = library_linkage([], [lib], [unit])
assert r["valid"] and r["same_ordinary_library"] is None
assert library_linkage([lib], [lib], [unit])["valid"]
wrong = copy.deepcopy(lib)
wrong["filenames"]["/lib.rlib"] = "different"
assert not library_linkage([lib], [wrong], [unit])["valid"]
wrong = {**lib, "features": ["different"]}
assert not library_linkage([], [wrong], [unit])["valid"]
assert not library_linkage([], [{**lib, "package_id": "other-source"}], [unit])["valid"]
assert not library_linkage([], [lib], [])["valid"]
assert not library_linkage([], [lib], [unit, unit])["valid"]
assert not library_linkage([], [], [unit])["valid"]
print("8 linkage controls passed; no build or GPU")
