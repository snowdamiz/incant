#!/usr/bin/env python3
"""Publicly author matched TS/Wasm projects. Runs tools only when explicitly invoked.

The Rust guest is built with the fixed generated SDK helper; both copies start
from one public init. Project/source mutations use incant_cmd with undo/redo and
reopen proof. No shipping export or timing run is implicit.
"""
import argparse
import json
from pathlib import Path
import shutil
import sys

from cost_workload import (HERE, TOOLS, COUNT, BATCH, SCRIPT, TICKS, owner, entity, recording,
                    observation, json_hash, file_hash, read, require, run, validate_documents, write_json)
from cost_rpc import Session


def transact(binary, project, output, batches, *, check_script=False):
    calls, transactions, hashes = [], [], []
    session = Session(binary, project, project.parent / 'authoring.journal.jsonl', calls, output)
    try:
        hashes.append(json_hash(session.call('project.read')['project']))
        for description, commands in batches:
            transactions.append(session.execute(description, commands))
            hashes.append(json_hash(session.call('project.read')['project']))
        for expected in reversed(hashes[:-1]):
            session.call('history.undo')
            require(json_hash(session.call('project.read')['project']) == expected, 'Undo changed project data')
        if check_script:
            require(not (project.parent / 'behavior.ts').exists(), 'Source undo left behavior behind')
        for expected in hashes[1:]:
            session.call('history.redo')
            require(json_hash(session.call('project.read')['project']) == expected, 'Redo changed project data')
        session.call('project.save')
        saved = session.call('project.read')
    finally:
        session.close()
        write_json(output.with_suffix('.transactions.json'), {'calls': calls, 'transactions': transactions})
    reopened = Session(binary, project, project.parent / 'authoring.journal.jsonl', [],
                       output.with_name(output.name + '-reopen'))
    try:
        require(reopened.call('project.read') == saved, 'Saved reopen differs')
        if check_script:
            require(reopened.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source']
                    == (HERE / 'behavior.ts').read_text(), 'Public source read differs')
    finally:
        reopened.close()
    return saved['project']


def prepare(output, binary):
    binary = binary.resolve(strict=True)
    output = output.absolute()
    require(not output.exists() and output.parent.is_dir(), 'Choose a new output directory')
    output.mkdir()
    base = output / 'base'; base.mkdir()
    run([binary, 'init', base / 'project.json', '--entities', '0'], output / 'init')
    initial = json.loads(read(base / 'project.json', 1024**2))
    require(len(initial['scenes']) == 1, 'Expected one initialized scene')
    scene = next(iter(initial['scenes']))
    batches = [('Author matched input', [{'op': 'set_input_actions', 'actions': {
        'right': {'kind': 'button', 'bindings': [{'type': 'key', 'code': 'ArrowRight'}]}}}])]
    for start in range(0, COUNT, BATCH):
        batches.append((f'Author rows {start}..{start + BATCH - 1}', [
            {'op': 'create_entity', 'scene_id': scene, 'entity': entity(n)} for n in range(start, start + BATCH)]))
    transact(binary, base / 'project.json', output / 'base-author', batches)
    # Copy saved public data only, not replay journals. Both copies retain IDs.
    for kind in ('typescript', 'wasm'):
        destination = output / kind; destination.mkdir()
        shutil.copy2(base / 'project.json', destination / 'project.json')
    ts_project = output / 'typescript/project.json'
    # Attach the callback to an existing row, so the runtime scene population and
    # archetype columns remain equal. Script is authoring metadata, not a hot ECS component.
    ts = transact(binary, ts_project, output / 'ts-author', [('Register TypeScript system', [
        {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'},
         'source': (HERE / 'behavior.ts').read_text()},
        {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Matched transform system', 'path': 'behavior.ts'}},
        {'op': 'set_component', 'scene_id': scene, 'entity_id': owner(0), 'component': 'Script',
         'value': {'source': SCRIPT, 'props': {}}}])], check_script=True)
    build = output / 'guest-build'; build.mkdir()
    guest = build / 'behavior.wasm'
    source = build / 'behavior.rs'; shutil.copy2(HERE / 'behavior.rs', source)
    compiled = json.loads(run([sys.executable, TOOLS / 'rust_guest.py', source, guest], output / 'guest-compile'))
    require(compiled['wasm_sha256'] == file_hash(guest), 'Guest producer identity differs')
    shutil.copy2(guest, output / 'wasm/behavior.wasm')
    wasm_project = output / 'wasm/project.json'
    imported = json.loads(run([binary, 'import', wasm_project, 'behavior.wasm'], output / 'import'))
    require(imported['asset']['kind'] == 'wasm' and imported['asset']['sha256'] == file_hash(guest),
            'Public import identity differs')
    wasm = transact(binary, wasm_project, output / 'wasm-author', [('Select authored Wasm', [
        {'op': 'set_scene_program', 'scene_id': scene, 'program': {'kind': 'wasm', 'module': imported['asset']['id']}}])])
    validate_documents(ts, wasm)
    write_json(output / 'input.json', recording())
    for page in range(COUNT // BATCH):
        write_json(output / f'observe-{page}.json', observation(list(range(page * BATCH, (page + 1) * BATCH))))
    write_json(output / 'observe.json', observation([0, COUNT // 2, COUNT - 1], population=True))
    paths = ['input.json', 'observe.json', *(f'observe-{i}.json' for i in range(4)),
             'typescript/project.json', 'typescript/behavior.ts', 'wasm/project.json', 'wasm/behavior.wasm']
    receipt = {'format': 'incant-wasm-ts-workload', 'version': 1, 'entities': COUNT, 'ticks': TICKS,
               'project_id': ts['id'], 'scene_id': scene, 'binary_sha256': file_hash(binary, 512 * 1024**2),
               'files': {path: file_hash(output / path) for path in paths}, 'guest': compiled,
               'source': {name: file_hash(HERE / name) for name in ('behavior.rs', 'behavior.ts')},
               'public_undo_redo_reopen': True, 'measured': False, 'source_root': str(HERE)}
    write_json(output / 'prepared.json', receipt)
    return receipt


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.output, args.binary), indent=2))
