#!/usr/bin/env python3
"""Compile retained, build-generated fixed temporal artifacts without a GPU.

MSL compilation/metallib linking and SPIR-V validation do not establish driver
execution. HLSL is retained; DXIL is explicitly unverified when DXC is absent.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import time
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("build_out", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--native-metalfx", action="store_true", help="Require both native boundary program families")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    records = []
    artifacts = []
    report = {"format": "incant-cooked-temporal-offline", "version": 1,
              "build_out": str(args.build_out.resolve()), "commands": records,
              "artifacts": artifacts, "gpu_execution": False,
              "dxil": "not run: DXC unavailable"}

    def run(name, command):
        start = time.time()
        stdout, stderr = output / (name + ".stdout"), output / (name + ".stderr")
        with stdout.open("xb") as out, stderr.open("xb") as err:
            child = subprocess.Popen(command, stdout=out, stderr=err)
            try:
                code = child.wait(timeout=120)
            except BaseException:
                child.kill()
                code = child.wait()
                records.append({"name": name, "command": command, "exit_code": code,
                                "exception": True, "started": start,
                                "stdout_sha256": digest(stdout), "stderr_sha256": digest(stderr)})
                raise
        records.append({"name": name, "command": command, "exit_code": code,
                        "started": start, "ended": time.time(),
                        "stdout_sha256": digest(stdout), "stderr_sha256": digest(stderr)})
        if code:
            raise RuntimeError(f"{name} failed: {code}; retained {stderr}")

    try:
        run("metal-version", ["xcrun", "-sdk", "macosx", "metal", "--version"])
        run("spirv-version", ["spirv-val", "--version"])
        run("sdk-version", ["xcrun", "-sdk", "macosx", "--show-sdk-version"])
        for family in ("motion", "depth", "resolve", "copy", "upscale", "composite", "cluster-shaders") + (("metalfx-pack", "metalfx-normalize") if args.native_metalfx else ()):
            directory = "temporal-" + family
            original = args.build_out / directory
            target = output / directory
            shutil.copytree(original, target)
            stages = ("Compute",) if family == "cluster-shaders" else ("Vertex", "Fragment")
            for stage in stages:
                label = f"{family}-{stage}"
                metal = target / f"Metal-{stage}.shader"
                air, library = target / f"{stage}.air", target / f"{stage}.metallib"
                run(label + "-metal", ["xcrun", "-sdk", "macosx", "metal", "-std=macos-metal2.4",
                                       "-x", "metal", "-c", str(metal), "-o", str(air)])
                run(label + "-metallib", ["xcrun", "-sdk", "macosx", "metallib", str(air), "-o", str(library)])
                run(label + "-spirv", ["spirv-val", "--target-env", "vulkan1.0", str(target / f"Vulkan-{stage}.shader")])
        report["complete"] = True
    finally:
        for path in sorted(output.rglob("*")):
            if path.is_file():
                artifacts.append({"path": str(path.relative_to(output)), "bytes": path.stat().st_size,
                                  "sha256": digest(path)})
        (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
