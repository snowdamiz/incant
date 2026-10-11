"""A separately gated 0040 tangent delta; original helpers/evidence stay immutable.

Only this entry point may launch the candidate for the delta. The subprocess
proxy rechecks the identity-specific grant immediately before EVERY Popen,
including imports/rpc, so withdrawing the gate stops the next process launch.
"""
import argparse
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import engine


class GuardedSubprocess:
    def __init__(self, gate, token, binary):
        self.gate, self.token, self.binary = Path(gate), token, str(binary)

    def __getattr__(self, name):
        return getattr(subprocess, name)

    def Popen(self, command, *args, **kwargs):
        if not isinstance(command, list) or str(command[0]) != self.binary:
            raise engine.EngineError("delta helper refuses a different executable")
        try:
            grant = self.gate.read_text().strip()
        except FileNotFoundError:
            raise engine.EngineError("delta capture slot is closed; wait for Astra") from None
        if grant != self.token:
            raise engine.EngineError("delta slot grant does not match the frozen candidate")
        return subprocess.Popen(command, *args, **kwargs)


def configure():
    metadata = engine.ROOT / "artifacts/tools/tangent-candidate.json"
    info = json.loads(metadata.read_text())
    engine.BINARY = engine.ROOT / "artifacts/tools/incant_headless-tangent"
    engine.BINARY_JSON = metadata
    engine.ART = engine.ROOT / "artifacts/0040-static-fbx-import/tangent-delta"
    engine.LEDGER = engine.ART / "ledger.jsonl"
    engine.MAX_CAPTURES = 20
    engine.ART.mkdir(parents=True, exist_ok=True)
    token = info["source_commit"] + ":" + info["sha256"]
    engine.subprocess = GuardedSubprocess(engine.ROOT / "artifacts/tools/gpu-tangent-ready", token, engine.BINARY)
    engine.check_binary()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["textures", "textures-twin"])
    parser.add_argument("run")
    args = parser.parse_args()
    configure()
    import review
    review.SOURCES = engine.ROOT / "artifacts/0040-static-fbx-import/sources"
    review.EVIDENCE = Path(__file__).resolve().parents[1] / "evidence/tangent-delta"
    command = {"textures": review.textures, "textures-twin": review.textures_twin}[args.mode]
    print(command(args.run))


if __name__ == "__main__":
    main()
