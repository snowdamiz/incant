"""Durable launch ledger and exclusive runner lock. Standard library only.

Every record is one JSON line, flushed and fsynced before the call returns.
Budgets are charged by `reserve` records only and are never released: a failed,
timed-out or interrupted child keeps its full reservation. A `reserve` without a
matching `complete` means a runner died uncatchably (SIGKILL); runner.py then
refuses new children until `reconcile` resolves it.
"""

import datetime
import fcntl
import json
import os
import time
from contextlib import contextmanager
from pathlib import Path


class Refused(Exception):
    """A gate refused before any child started."""


def now_iso():
    return datetime.datetime.now(datetime.UTC).isoformat()


class Ledger:
    def __init__(self, evidence):
        self.evidence = Path(evidence)
        self.path = self.evidence / "ledger.jsonl"
        self.lock_path = self.evidence / "runner.lock"
        self._held = False

    @contextmanager
    def locked(self):
        """Exclusive, non-waiting: a second runner refuses instead of queueing."""
        fd = os.open(self.lock_path, os.O_RDWR | os.O_CREAT, 0o600)
        try:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as error:
                raise Refused("another runner holds the packet lock") from error
            self._held = True
            try:
                yield self
            finally:
                self._held = False
                fcntl.flock(fd, fcntl.LOCK_UN)
        finally:
            os.close(fd)

    def records(self):
        if not self.path.exists():
            return []
        out = []
        for number, line in enumerate(self.path.read_text().splitlines(), 1):
            if not line.strip():
                continue
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError as error:
                raise Refused(f"ledger line {number} is not JSON; refusing") from error
        return out

    def append(self, record):
        if not self._held:
            raise RuntimeError("ledger writes require the packet lock")
        record = {"at": now_iso(), "at_epoch": time.time(), **record}
        line = json.dumps(record, sort_keys=True) + "\n"
        created = not self.path.exists()
        with open(self.path, "a") as handle:
            handle.write(line)
            handle.flush()
            os.fsync(handle.fileno())
        if created:
            directory = os.open(self.evidence, os.O_RDONLY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
        return record

    def totals(self):
        reserves = [r for r in self.records() if r.get("type") == "reserve"]
        return {
            "children": sum(1 for r in reserves if r.get("counts_as_child", True)),
            "existing_frames": sum(r.get("existing_frames", 0) for r in reserves),
            "new_frames": sum(r.get("new_frames", 0) for r in reserves),
            "pngs": sum(r.get("pngs", 0) for r in reserves),
            "retries": sum(1 for r in reserves if r.get("kind") == "retry"),
            "first_launch_epoch": min((r["at_epoch"] for r in reserves if r.get("counts_as_child", True)),
                                      default=None),
            "labels": [r["label"] for r in reserves],
        }

    def unresolved(self):
        """Reservations whose child has no completion or reconciliation record."""
        records = self.records()
        done = {r["label"] for r in records if r.get("type") in ("complete", "reconciled")}
        return [r for r in records if r.get("type") == "reserve" and r["label"] not in done]

    def check_and_reserve(self, budgets, request):
        """Charge `request` iff every ceiling still holds; otherwise refuse."""
        totals = self.totals()
        if request["label"] in totals["labels"]:
            raise Refused(f"label {request['label']} already used; outputs are never replaced")
        problems = []
        child = 1 if request.get("counts_as_child", True) else 0
        if totals["children"] + child > budgets["gpu_children_max"]:
            problems.append(f"children {totals['children']} + {child} > {budgets['gpu_children_max']}")
        existing = totals["existing_frames"] + request.get("existing_frames", 0)
        new = totals["new_frames"] + request.get("new_frames", 0)
        if existing > budgets["existing_frames_max"]:
            problems.append(f"existing frames {existing} > {budgets['existing_frames_max']}")
        if new > budgets["new_frames_max"]:
            problems.append(f"new frames {new} > {budgets['new_frames_max']}")
        if existing + new > budgets["combined_frames_max"]:
            problems.append(f"combined frames {existing + new} > {budgets['combined_frames_max']}")
        if totals["pngs"] + request.get("pngs", 0) > budgets["pngs_max"]:
            problems.append(f"PNGs {totals['pngs'] + request.get('pngs', 0)} > {budgets['pngs_max']}")
        if request.get("kind") == "retry" and totals["retries"] + 1 > budgets["children_planned"]["focused_retries_reserved"]:
            problems.append("focused retry budget exhausted")
        if child:
            start = totals["first_launch_epoch"] or time.time()
            elapsed = time.time() - start
            if elapsed + request["timeout_s"] > budgets["execution_seconds_max"]:
                problems.append(f"execution ceiling: {elapsed:.0f}s used + {request['timeout_s']}s "
                                f"watchdog > {budgets['execution_seconds_max']}s")
        if problems:
            raise Refused("; ".join(problems))
        return self.append({"type": "reserve", **request})
