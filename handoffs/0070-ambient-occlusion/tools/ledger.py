"""Durable launch ledger and exclusive non-waiting locks. Standard library only.

Every record is one JSON line, flushed and fsynced before the call returns.
Budgets are charged by `reserve` records only and are never released: a failed,
timed-out or interrupted child keeps its full reservation.

Lock order (runner.py): the shared native GPU lock first, then this packet's
lock. A lock is released by closing its descriptor, never by LOCK_UN: device
children inherit the shared-lock descriptor, so the lock stays held while any
of them is alive -- including an orphan that outlived a SIGKILLed runner.

Derived state:
- blocked: a reservation whose process group was never verified absent (no
  completion because the runner was lost, a completion with survivors or an
  unverified group, or a failed reconciliation). Every child refuses until
  `reconcile` proves ownership by exact identity and verifies the group absent.
- failed: a device child whose completion is not an explicit success, or that
  never completed. A failure latches: further device work refuses except a
  root-receipted precise retry.
"""

import datetime
import fcntl
import json
import os
import time
from contextlib import contextmanager
from pathlib import Path

DEVICE_KINDS = ("listing", "suite", "numeric", "appearance", "retry")


class Refused(Exception):
    """A gate refused before any child started."""


def now_iso():
    return datetime.datetime.now(datetime.UTC).isoformat()


@contextmanager
def exclusive(path, what):
    """Exclusive flock(2) on `path`, never waiting. Yields the descriptor."""
    try:
        fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o600)
    except OSError as error:
        raise Refused(f"cannot open {what} at {path}: {error}") from error
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise Refused(f"{what} ({path}) is held by another runner or its child; refusing without waiting") \
                from error
        yield fd
    finally:
        os.close(fd)


class Ledger:
    def __init__(self, evidence):
        self.evidence = Path(evidence)
        self.path = self.evidence / "ledger.jsonl"
        self.lock_path = self.evidence / "runner.lock"
        self._held = False

    @contextmanager
    def locked(self):
        """This packet's lock: a second runner refuses instead of queueing."""
        with exclusive(self.lock_path, "the packet lock"):
            self._held = True
            try:
                yield self
            finally:
                self._held = False

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

    def children(self):
        """Per label: reservation, spawned/member records, completion, reconciliations."""
        out = {}
        for r in self.records():
            kind, label = r.get("type"), r.get("label")
            if label is None or kind not in ("reserve", "spawned", "member", "complete", "reconcile"):
                continue
            entry = out.setdefault(label, {"reserve": None, "spawned": [], "member": [], "complete": None,
                                           "reconcile": []})
            if kind in ("reserve", "complete"):
                entry[kind] = r
            else:
                entry[kind].append(r)
        return {label: e for label, e in out.items() if e["reserve"] is not None}

    @staticmethod
    def group_verified(entry):
        if any(r.get("resolved") is True for r in entry["reconcile"]):
            return True
        done = entry["complete"]
        if done is None:
            return False
        outcome = done.get("outcome") or {}
        return (outcome.get("group_absent") is True and not outcome.get("survivors")
                and not outcome.get("leader_unreaped"))

    def blocked(self):
        """Labels whose process group was never verified absent."""
        return {label: e for label, e in self.children().items() if not self.group_verified(e)}

    def failures(self):
        """Latched device failures: anything but an explicit success."""
        out = {}
        for label, entry in self.children().items():
            if entry["reserve"].get("kind") not in DEVICE_KINDS:
                continue
            done = entry["complete"]
            if done is None:
                out[label] = ["no completion record: the runner was lost before the child finished"]
            elif done.get("success") is not True:
                out[label] = done.get("failure_reasons") or ["completion without an explicit success"]
        return out

    def retained_identities(self, entry):
        """Every exact identity this packet recorded for one child."""
        found = []
        for r in entry["spawned"]:
            found.append(r.get("identity"))
        for r in entry["member"]:
            found.append(r.get("identity"))
        outcome = (entry["complete"] or {}).get("outcome") or {}
        found.append(outcome.get("leader"))
        found.extend(outcome.get("members_seen") or [])
        found.extend(outcome.get("survivors") or [])
        for r in entry["reconcile"]:
            found.extend(r.get("retained") or [])
        unique = []
        for ident in found:
            if isinstance(ident, dict) and "start" in ident and ident not in unique:
                unique.append(ident)
        return unique

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
