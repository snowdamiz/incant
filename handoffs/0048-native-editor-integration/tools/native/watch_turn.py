"""Read-only watcher for the one granted agent turn (handoff 0048).

Polls the review PID's window through winctl's accessibility reads and captures that window only
when the Agent pane's state changes. It sends no input. It stops when an approval card waits
(an enabled "Allow" button), when the turn has ended (no agent Stop button and the header is no
longer working), or after the time limit.

Usage: python3 -I watch_turn.py <pid> <out-dir> <prefix> <seconds>
"""
import json
import os
import subprocess
import sys
import time

WINCTL = "/tmp/incant0048/winctl"
CAP = "/tmp/incant0048/cap.sh"


def dump(pid):
    out = subprocess.run([WINCTL, pid, "ax", "dump"], capture_output=True, text=True, timeout=20)
    return [json.loads(line) for line in out.stdout.splitlines() if line.startswith("{")]


def side(node, left):
    return node.get("x", 0) >= left


def state(nodes):
    """The Agent pane's observable state, from the window's accessibility tree."""
    agent = next((n for n in nodes if n.get("sub") == "AXLandmarkRegion" and n.get("title") == "Agent"), None)
    left = agent["x"] if agent else 1000
    top = agent["y"] if agent else 500
    header = next((n.get("title", "") for n in nodes if n["role"] == "AXHeading" and (n.get("title") or "").startswith("Agent")), "")
    buttons = [n for n in nodes if n["role"] == "AXButton" and side(n, left) and n.get("y", 0) >= top]
    names = [(n.get("title") or n.get("label") or "") for n in buttons]
    allow = next((n for n in buttons if (n.get("title") or "") in ("Allow", "Allowing…")), None)
    texts = [n.get("value") or "" for n in nodes if n["role"] == "AXStaticText" and side(n, left) and n.get("y", 0) >= top]
    return {
        "header": header.replace("\n", " "),
        "stop": "Stop" in names,
        "send": "Send" in names,
        "allow": None if allow is None else {"title": allow.get("title"), "enabled": allow.get("enabled", True)},
        "deny": "Deny" in names,
        "buttons": names,
        "texts": texts,
    }


def main():
    pid, out_dir, prefix, seconds = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4])
    os.makedirs(out_dir, exist_ok=True)
    log = open(os.path.join(out_dir, f"{prefix}-watch.jsonl"), "a")
    start = time.time()
    last = None
    shots = 0
    seen_work = False
    while time.time() - start < seconds:
        nodes = dump(pid)
        now = state(nodes)
        key = (now["header"], now["stop"], now["send"], json.dumps(now["allow"]), len(" ".join(now["texts"])) // 200)
        record = {"t": round(time.time() - start, 2), **{k: v for k, v in now.items() if k != "texts"},
                  "text_chars": len(" ".join(now["texts"]))}
        if key != last:
            shots += 1
            name = os.path.join(out_dir, f"{prefix}-{shots:02d}.png")
            subprocess.run(["sh", CAP, pid, name], capture_output=True)
            with open(os.path.join(out_dir, f"{prefix}-{shots:02d}-ax.jsonl"), "w") as f:
                f.write("\n".join(json.dumps(n, ensure_ascii=False) for n in nodes) + "\n")
            record["shot"] = os.path.basename(name)
            last = key
        log.write(json.dumps(record, ensure_ascii=False) + "\n")
        log.flush()
        print(json.dumps(record, ensure_ascii=False)[:300], flush=True)
        if now["stop"] or "Working" in now["header"] or "Starting" in now["header"]:
            seen_work = True
        if now["allow"] and now["allow"]["enabled"] and now["allow"]["title"] == "Allow":
            print("approval waiting", flush=True)
            return
        if seen_work and not now["stop"] and not now["allow"] and now["send"]:
            print("turn ended", flush=True)
            return
        time.sleep(0.4)
    print("time limit", flush=True)


if __name__ == "__main__":
    main()
