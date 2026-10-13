"""Verify the lossless publication packaging without writing reconstructed files."""
from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parents[4]
directory = Path(__file__).resolve().parent
manifest = json.loads((directory / "artifact-parts.json").read_text())
assert manifest["format"] == "ordered-byte-parts-v1"
whole = hashlib.sha256()
size = 0
for part in manifest["parts"]:
    path = root / part["path"]
    digest = hashlib.sha256()
    part_size = 0
    with path.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
            whole.update(block)
            part_size += len(block)
    assert part_size == part["bytes"], path
    assert digest.hexdigest() == part["sha256"], path
    assert part_size <= 16 * 1024 * 1024, path
    size += part_size
assert size == manifest["reconstructed_bytes"]
assert whole.hexdigest() == manifest["reconstructed_sha256"]
reviewed = json.loads((directory / "reviewed-manifest.json").read_text())
assert whole.hexdigest() == reviewed["archive"]["sha256"]
print(json.dumps({"passed": True, "parts": len(manifest["parts"]),
                  "bytes": size, "sha256": whole.hexdigest()}))
