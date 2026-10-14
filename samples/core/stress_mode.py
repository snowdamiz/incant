#!/usr/bin/env python3
"""Prepare the separate Core fast-mode authoring input through one public edit."""
import argparse
import copy
import json
from pathlib import Path
import shutil

import stress
from stress_rpc import Session


def prepare(project, output, binary, stress_receipt=None):
    project = project.resolve(strict=True); binary = binary.resolve(strict=True); output = output.resolve()
    if output.exists() or output.is_relative_to(project.parent): raise ValueError('Output must be a fresh external directory')
    original_files = stress.inventory(project.parent)
    before = json.loads(project.read_text())
    inherited = None
    if stress_receipt is None:
        stress.validate_base(before)
    else:
        inherited = json.loads(stress_receipt.read_text())
        if (inherited.get('format') != 'incant-core-script-stress' or
                inherited.get('project_sha256') != stress.digest(project) or
                inherited.get('scripted_entities') != 10000 or
                not inherited.get('public_undo_redo_reopen')):
            raise ValueError('Deterministic stress receipt does not bind this complete input')
        projected = copy.deepcopy(before)
        for n in range(stress.COUNT):
            del projected['scenes'][stress.bindings.SCENE]['entities'][stress.owner(n)]
        del projected['scripts'][stress.SCRIPT]
        stress.verify_extension(projected, before)
        for source, name in ((stress.bindings.SCRIPT, 'gameplay.ts'), (stress.SCRIPT, 'stress.ts')):
            if inherited['script_sha256'][source] != stress.digest(project.parent / name):
                raise ValueError('Inherited script bytes differ')
    if before['settings'].get('physics_mode', 'deterministic') != 'deterministic':
        raise ValueError('Fast-mode reference must start with the original deterministic Core')
    expected = copy.deepcopy(before); expected['settings']['physics_mode'] = 'fast'
    output.mkdir(parents=True); target = output / 'authoring'
    # Keep project/assets/source bytes, not an earlier run's undo/recovery log.
    excluded = {rel for rel in original_files if rel.endswith('.journal.jsonl') or
                rel.startswith('.incant-source-transactions/')}
    def ignore(directory, names):
        relative = Path(directory).relative_to(project.parent)
        return [name for name in names if name == '.incant-source-transactions' or
                str(relative / name) in excluded]
    shutil.copytree(project.parent, target, ignore=ignore)
    copied = target / project.name; calls = []
    session = Session(binary, copied, target / 'mode.journal.jsonl', calls, output / 'rpc-mode')
    try:
        if session.call('project.read')['project'] != before: raise ValueError('Initial project mismatch')
        tx = session.execute('Select the separate Core fast physics reference', [{'op': 'set_physics_mode', 'mode': 'fast'}])
        if session.call('project.read')['project'] != expected: raise ValueError('Mode edit changed other project fields')
        session.call('history.undo')
        if session.call('project.read')['project'] != before: raise ValueError('Mode undo differs')
        session.call('history.redo')
        if session.call('project.read')['project'] != expected: raise ValueError('Mode redo differs')
        session.call('project.save'); saved = session.call('project.read')
    finally:
        try: session.close()
        finally: stress.write(output / 'calls.json', calls)
    session = Session(binary, copied, target / 'mode.journal.jsonl', calls, output / 'rpc-reopen')
    try:
        if session.call('project.read') != saved: raise ValueError('Mode reopen differs')
    finally:
        try: session.close()
        finally: stress.write(output / 'calls.json', calls)
    if stress.inventory(project.parent) != original_files: raise ValueError('Protected input changed')
    for rel, sha in original_files.items():
        if rel != project.name and rel not in excluded and stress.digest(target / rel) != sha: raise ValueError('Asset/source changed: ' + rel)
    receipt = {'format': 'incant-core-fast-input', 'version': 1, 'project': str(copied),
        'original_project_sha256': stress.digest(project), 'project_sha256': stress.digest(copied),
        'binary_sha256': stress.digest(binary), 'original_files': original_files,
        'authoring_files': stress.inventory(target), 'transaction': tx,
        'only_project_delta': '/settings/physics_mode = fast', 'undo_redo_reopen': True,
        'runtime_execution': False, 'performance_measured': False,
        'omitted_prior_authoring_history': sorted(excluded),
        'inherited_stress_receipt_sha256': stress.digest(stress_receipt) if stress_receipt else None,
        'scripted_entities': 10000 if inherited else 1}
    stress.write(output / 'receipt.json', receipt); return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path); parser.add_argument('output', type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--stress-receipt', type=Path)
    a = parser.parse_args(); print(json.dumps(prepare(a.project, a.output, a.binary, a.stress_receipt), indent=2))

if __name__ == '__main__': main()
