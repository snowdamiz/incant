"""Read-only poll of the approval card's readiness hold (handoff 0048, 0049 behaviour).

Records, with timestamps, the Agent header, whether the hold sentence is shown, and the Allow
button's state, as fast as winctl's accessibility reads allow. Sends no input.

Usage: python3 -I poll_hold.py <pid> <out.jsonl> <seconds>
"""
import json
import subprocess
import sys
import time

WINCTL = "/tmp/incant0048/winctl"
HOLD = "Waiting for the agent to update its session"


def main():
    pid, out, seconds = sys.argv[1], sys.argv[2], float(sys.argv[3])
    start = time.time()
    with open(out, "a") as log:
        while time.time() - start < seconds:
            lines = subprocess.run([WINCTL, pid, "ax", "dump"], capture_output=True, text=True, timeout=20).stdout.splitlines()
            nodes = [json.loads(line) for line in lines if line.startswith("{")]
            header = next((n.get("title", "") for n in nodes if n["role"] == "AXHeading" and (n.get("title") or "").startswith("Agent")), "")
            allow = next((n for n in nodes if n["role"] == "AXButton" and n.get("title") in ("Allow", "Allowing…")), None)
            held = [n.get("value") for n in nodes if n["role"] == "AXStaticText" and HOLD in (n.get("value") or "")]
            decided = [n.get("value") for n in nodes if n["role"] == "AXStaticText" and (n.get("value") or "") in ("Allowed", "Denied")]
            record = {
                "t": round(time.time() - start, 3),
                "header": header.replace("\n", " "),
                "allow": None if allow is None else {"title": allow.get("title"), "enabled": allow.get("enabled", True)},
                "hold": held[0] if held else None,
                "decided": decided,
            }
            log.write(json.dumps(record, ensure_ascii=False) + "\n")
            log.flush()


if __name__ == "__main__":
    main()
