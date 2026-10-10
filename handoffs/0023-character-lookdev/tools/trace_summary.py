#!/usr/bin/env python3
"""Summarize the per-tick debug rows logged by character_course.ts.

Usage: python3 -I trace_summary.py RUN.logs.jsonl [LANE] [FIRST_TICK] [LAST_TICK]
       python3 -I trace_summary.py RUN.logs.jsonl --check

Without LANE, prints each lane's ungrounded tick spans, x/y extremes and final
pose. With LANE, prints that lane's rows for the given tick range. --check scans
every tick: spans whose +X progress falls below 95% of the walking input, ticks
whose sliding_down_slope flag is true, and airborne spans. Logged values are
rounded to 1 mm, so the threshold allows that rounding.
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


# +X input per tick: 1.6 m/s at 60 Hz; the wall lane walks along (1, -0.6).
EXPECTED_X = {"wall": 1.6 / 60 / (1 + 0.36) ** 0.5}


def spans(ticks):
    out = []
    for tick in ticks:
        if out and out[-1][1] == tick - 1:
            out[-1][1] = tick
        else:
            out.append([tick, tick])
    return out


def check(path):
    lanes = {}
    for tick, lane, x, y, z, grounded, sliding, delta, contacts in rows(path):
        info = lanes.setdefault(lane, {"slow": [], "flag": [], "air": [], "ticks": 0})
        info["ticks"] += 1
        if delta[0] < 0.95 * EXPECTED_X.get(lane, 1.6 / 60) - 0.0005:
            info["slow"].append((tick, delta[0]))
        if sliding:
            info["flag"].append(tick)
        if not grounded:
            info["air"].append(tick)
    for lane, info in lanes.items():
        slow = [[a, b, min(d for t, d in info["slow"] if a <= t <= b)]
                for a, b in spans([t for t, _ in info["slow"]])]
        print(lane, json.dumps({"ticks": info["ticks"], "slow_spans[first,last,min_dx]": slow,
                                "downhill_flag_spans": spans(info["flag"]), "air_spans": spans(info["air"])}))


def main():
    path = sys.argv[1]
    if sys.argv[2:] == ["--check"]:
        check(path)
        return
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
