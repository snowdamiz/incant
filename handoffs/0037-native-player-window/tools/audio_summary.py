"""Summarise a gamewin app-audio log against its run timeline.

Usage: python3 -I audio_summary.py RUN_DIR AUDIO_LOG_NAME

Prints per-phase mean RMS for the left and right channels, where phases are
the intervals between timeline steps that change focus, minimisation or keys.
The figures describe the game's captured application audio, not speakers.
"""

import json
import pathlib
import sys

MARKERS = {"audio-start", "down", "up", "finder-activate", "game-front", "minimize", "restore", "audio-stop"}


def load(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def main():
    run = pathlib.Path(sys.argv[1])
    buffers = [b for b in load(run / f"{sys.argv[2]}.jsonl") if "rms" in b]
    steps = [s for s in load(run / "timeline.jsonl") if s.get("step") in MARKERS]
    name = f"{sys.argv[2]}.jsonl"
    start = next(s for s in steps if s["step"] == "audio-start" and s.get("path") == name)
    stop = next(s for s in steps if s["step"] == "audio-stop" and s["epoch"] > start["epoch"])
    steps = [s for s in steps if start["epoch"] <= s["epoch"] <= stop["epoch"]]
    phases = []
    for current, following in zip(steps, steps[1:]):
        label = current["step"] + (f" {current['key']}" if "key" in current else "")
        phases.append((label, current["epoch"], following["epoch"]))
    report = []
    for label, begin, end in phases:
        inside = [b for b in buffers if begin <= b["epoch"] < end]
        # Skip the first 100 ms after each transition so the mean reflects the settled state.
        settled = [b for b in inside if b["epoch"] >= begin + 0.1]
        def mean(channel, items):
            return sum(b["rms"][channel] for b in items) / len(items) if items else None
        report.append({
            "phase": label,
            "start_ms": round((begin - start["epoch"]) * 1000),
            "duration_ms": round((end - begin) * 1000),
            "buffers": len(inside),
            "settled_buffers": len(settled),
            "rms_left": mean(0, settled),
            "rms_right": mean(1, settled),
            "silent_settled_buffers": sum(1 for b in settled if max(b["rms"]) < 1e-6),
        })
    print(json.dumps({"run": str(run), "audio": sys.argv[2], "buffers": len(buffers), "phases": report}, indent=1))


if __name__ == "__main__":
    main()
