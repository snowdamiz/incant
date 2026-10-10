#!/usr/bin/env python3
"""Summarize the per-tick debug rows logged by character_course.ts.

Usage: python3 -I trace_summary.py RUN.logs.jsonl [LANE] [FIRST_TICK] [LAST_TICK]

Without LANE, prints each lane's ungrounded tick spans, x/y extremes and final
pose. With LANE, prints that lane's rows for the given tick range.
"""

import json
import sys


def rows(path):
    for line in open(path, encoding="utf-8"):
        record = json.loads(line)
        if record["level"] == "debug":
            message = json.loads(record["message"])
            for name, x, y, z, grounded, sliding, delta, contacts in message["c"]:
                yield message["t"], name.split(" · ")[-1], x, y, z, grounded, sliding, delta, contacts


def main():
    path = sys.argv[1]
    if len(sys.argv) > 2:
        lane, first, last = sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
        for row in rows(path):
            if row[1] == lane and first <= row[0] <= last:
                print(*row)
        return
    lanes = {}
    for tick, lane, x, y, z, grounded, sliding, delta, contacts in rows(path):
        info = lanes.setdefault(lane, {"air": [], "slide": 0, "max_y": y, "min_y": y})
        if not grounded:
            if info["air"] and info["air"][-1][1] == tick - 1:
                info["air"][-1][1] = tick
            else:
                info["air"].append([tick, tick])
        info["slide"] += sliding
        info["max_y"], info["min_y"] = max(info["max_y"], y), min(info["min_y"], y)
        info["last"] = (tick, x, y, z, delta)
    for lane, info in lanes.items():
        print(lane, json.dumps(info))


if __name__ == "__main__":
    main()
