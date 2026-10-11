#!/usr/bin/env python3
"""Astra-owned, per-call gated native capture runner. Do not edit or bypass."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / 'artifacts/tools'
BUDGET = ROOT / 'artifacts/flipbook-capture-budget.json'


def within(value):
    path = Path(value).resolve()
    if not path.is_relative_to(ROOT):
        raise ValueError('Capture inputs/output must remain inside this handoff worktree')
    return path


def main():
    if not (TOOLS / 'gpu-review-ready').is_file():
        raise ValueError('GPU capture gate is absent; CPU preparation only')
    if len(sys.argv) != 7:
        raise ValueError('usage: capture.py SCENE MODELS TEXTURES CAMERA_HEX REQUEST_JSON NEW_OUTPUT_DIR')
    scene, models, textures, camera, request_path, output = sys.argv[1:]
    scene, models, textures, request_path, output = map(within, (scene, models, textures, request_path, output))
    if output.exists() or not output.is_relative_to(ROOT / 'artifacts'):
        raise ValueError('Capture output must be a new artifacts subdirectory')
    with request_path.open('rb') as source:
        data = source.read(65537)
    if len(data) > 65536:
        raise ValueError('Request exceeds 64 KiB')
    request = json.loads(data)
    width, height, ticks = request['width'], request['height'], request['ticks']
    if (type(width) is not int or type(height) is not int or not 16 <= width <= 1024
            or not 16 <= height <= 1024 or not isinstance(ticks, list)
            or not 1 <= len(ticks) <= 64 or any(type(t) is not int or not 0 <= t <= 36000 for t in ticks)
            or any(a >= b for a, b in zip(ticks, ticks[1:]))):
        raise ValueError('Capture dimensions/clock invalid')
    byte_count = width * height * 4 * len(ticks)
    manifest = json.loads((TOOLS / 'binary.json').read_text())
    tool = TOOLS / 'native_frames'
    expected = manifest['tools']['native_frames']['sha256']
    if hashlib.sha256(tool.read_bytes()).hexdigest() != expected:
        raise ValueError('Frozen native_frames executable hash mismatch')
    with (TOOLS / 'capture.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        if not (TOOLS / 'gpu-review-ready').is_file():
            raise ValueError('GPU capture gate was withdrawn')
        budget = json.loads(BUDGET.read_text()) if BUDGET.exists() else {'calls': 0, 'captures': 0, 'decoded_bytes': 0}
        budget = {'calls': budget['calls'] + 1, 'captures': budget['captures'] + len(ticks),
                  'decoded_bytes': budget['decoded_bytes'] + byte_count}
        if budget['calls'] > 40 or budget['captures'] > 96 or budget['decoded_bytes'] > 256 * 1024 * 1024:
            raise ValueError('Packet capture budget exhausted; ask Astra for a bounded follow-up')
        # Reserve before spawn; a failed/timed-out invocation still consumes budget.
        BUDGET.write_text(json.dumps(budget, indent=2) + '\n')
        command = [str(tool), str(scene), str(models), str(textures), camera, str(request_path), str(output)]
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        try:
            stdout, stderr = process.communicate(timeout=60)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.communicate()
            raise ValueError('Native capture exceeded 60 seconds; process group killed and reaped') from None
        finally:
            # This trusted native helper creates no subprocesses. Kill any
            # unexpected survivors in its private group before returning.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
        if hashlib.sha256(tool.read_bytes()).hexdigest() != expected:
            raise ValueError('Frozen executable changed during capture')
        if process.returncode:
            raise ValueError('Native capture failed: ' + stderr.decode('utf-8', errors='replace')[:4096])
        report = json.loads(stdout)
        if [r['tick'] for r in report['captures']] != ticks:
            raise ValueError('Native capture report clock mismatch')
        print(json.dumps({'passed': True, 'output': str(output.relative_to(ROOT)),
                          'captures': len(ticks), 'budget_used': budget, 'adapter': report['adapter']}))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(2)
