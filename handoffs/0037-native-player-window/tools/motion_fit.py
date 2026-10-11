"""Relate captured model positions to the key timeline of one gamewin plan.

Usage: python3 -I motion_fit.py RUN_DIR FIT_PREFIX [PREFIX ...]

Lists every capture whose name starts with FIT_PREFIX or another PREFIX, with
its time (ms after the plan's begin step) and measured triangle centroid/left
edge, interleaved with the D key events of that same plan. It then reports a
least-squares slope (px/s) over the FIT_PREFIX captures only. Positions come
from analysis.jsonl, times from timeline.jsonl. Coherence only: this makes no
frame-rate, smoothness or budget claim.
"""

import json
import pathlib
import sys


def records(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def plans(timeline):
    current = []
    for step in timeline:
        if step.get("step") == "begin" and current:
            yield current
            current = []
        current.append(step)
    if current:
        yield current


def main():
    run = pathlib.Path(sys.argv[1])
    fit, prefixes = sys.argv[2], sys.argv[2:]
    shapes = {pathlib.Path(r["path"]).name: r for r in records(run / "analysis.jsonl")}
    def wanted(step):
        if step.get("step") not in ("cap", "burst"):
            return False
        return any(pathlib.Path(step["path"]).name.startswith(prefix) for prefix in prefixes)
    plan = next(p for p in plans(records(run / "timeline.jsonl")) if any(wanted(s) for s in p))
    begin = plan[0]["epoch"]
    samples = []
    for step in plan:
        t = (step["epoch"] - begin) * 1000
        if step.get("step") in ("down", "up") and step.get("key") == "D":
            print(f"{t:8.0f} ms  D {step['step']}")
        elif step.get("step") in ("finder-activate", "game-front", "minimize", "restore"):
            print(f"{t:8.0f} ms  {step['step']}")
        elif wanted(step):
            name = pathlib.Path(step["path"]).name
            shape = shapes[name]
            if "centroid" in shape:
                print(f"{t:8.0f} ms  {name:28s} centroid_x={shape['centroid'][0]:7.1f} left_px={shape['bbox'][0]}")
                if name.startswith(fit):
                    samples.append((t / 1000, shape["centroid"][0]))
            else:
                print(f"{t:8.0f} ms  {name:28s} no foreground")
    if len(samples) >= 2:
        n = len(samples)
        mean_t = sum(t for t, _ in samples) / n
        mean_c = sum(c for _, c in samples) / n
        slope = sum((t - mean_t) * (c - mean_c) for t, c in samples) / sum((t - mean_t) ** 2 for t, _ in samples)
        print(f"slope over {n} '{fit}' captures: {slope:.1f} px/s")


if __name__ == "__main__":
    main()
