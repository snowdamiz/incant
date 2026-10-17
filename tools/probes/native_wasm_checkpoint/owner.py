"""Existing managed group lifecycle, with an independent observed PID journal.

Trusted Cargo/Clang descendants must retain the managed process group. Polling
does not prove every transient descendant was observed and is not a sandbox.
"""
import json
import os
import time

from ..native_wasm_restart.owner import Owner, process_rows
from ..native_wasm_restart.verify import require


class CheckpointOwner(Owner):
    def __init__(self, output, target):
        super().__init__(output, target, 12 * 1024**3)
        self.deadline = self.start + 5400
        self.identities = {}
        self.discover(os.getpid(), os.getpgrp(), 'wrapper')

    def discover(self, pid, group, source):
        if pid in self.identities:
            require(self.identities[pid]['pgid'] == group, 'Observed PID changed process group')
            return
        require(len(self.identities) < 10000, 'Observed process inventory bound')
        row = {'pid': pid, 'pgid': group, 'source': source}
        # Independent append-only sampling record, never reconstructed from a
        # final all-absent claim. Closed on every append for terminal recovery.
        with (self.output / 'process-inventory.jsonl').open('a') as file:
            file.write(json.dumps(row, sort_keys=True) + '\n')
        self.identities[pid] = row

    def save(self):
        for group in self.groups:
            self.discover(group['pid'], group['pid'], 'managed-leader')
        super().save()

    def check(self, force=False):
        if force or time.monotonic() - self.last_sample >= 1:
            for pid, group, _ in process_rows():
                if group in self.live_groups:
                    self.discover(pid, group, 'managed-leader' if pid == group else 'sampled-descendant')
        super().check(force)

    def absent_children(self):
        rows = process_rows()
        return [{'pid': pid, 'pid_and_group_absent': not any(pid in (p, g) for p, g, _ in rows)}
                for pid in sorted(self.identities) if pid != os.getpid()]
