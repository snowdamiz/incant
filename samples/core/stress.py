#!/usr/bin/env python3
"""Add exactly 9,999 invisible bulk movers to a fresh copy of the authored Core.

All project/source changes use public incant_cmd transactions. No export/build or
performance run happens here. Input and approved Core content remain unchanged.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil

import bindings
from art import ids
from stress_rpc import Session

HERE = Path(__file__).resolve().parent
COUNT = 9999
SCRIPT = ids.make('script', 2)
FIRST = 100_000
BATCH = 256

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def hash_json(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()

def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')

def inventory(root):
    result = {}
    for path in sorted(root.rglob('*')):
        if path.is_symlink():
            raise ValueError('Core stress input must not contain symbolic links')
        if path.is_file():
            result[str(path.relative_to(root))] = digest(path)
    return result

def owner(ordinal):
    if type(ordinal) is not int or not 0 <= ordinal < COUNT:
        raise ValueError('Core stress ordinal out of range')
    return ids.entity(FIRST + ordinal)

def position(ordinal, tick):
    owner(ordinal)
    if type(tick) is not int or not 1 <= tick <= 3600:
        raise ValueError('Core stress oracle tick out of bounded player range')
    phase = (tick % 64 - 32) / 64
    return [ordinal % 97 - 48 + phase, ordinal // 97 % 97 - 48, ordinal % 13 - 6 - phase]

def sentinel(tick):
    values = [position(n, tick) for n in range(COUNT)]
    return [COUNT, sum(p[0] for p in values) / COUNT, sum(p[2] for p in values) / COUNT]

def validate_base(document):
    if set(document['scenes']) != {bindings.SCENE}:
        raise ValueError('Core stress requires the actual single Core presentation scene')
    entities = document['scenes'][bindings.SCENE]['entities']
    attached = [(key, e['components']['Script']) for key, e in entities.items() if 'Script' in e['components']]
    if len(attached) != 1 or attached[0][0] != bindings.CONTROLLER or attached[0][1]['source'] != bindings.SCRIPT:
        raise ValueError('Core stress requires exactly one original gameplay controller')
    if set(document['scripts']) != {bindings.SCRIPT} or document['scripts'][bindings.SCRIPT]['path'] != 'gameplay.ts':
        raise ValueError('Core stress requires the unchanged Core gameplay registry')
    if any(owner(n) in entities for n in range(COUNT)):
        raise ValueError('Core stress entity ID collides with original content')
    bindings.authored_hud(document)
    props = attached[0][1]['props']
    if props.get('audio') != bindings.AUDIO or props.get('camera') != ids.entity(3):
        raise ValueError('Core stress requires production audio and camera bindings')
    return entities

def commands():
    source = (HERE / 'stress.ts').read_text()
    yield 'Author a separate bulk stress source', [
        {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': source},
        {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Core bulk stress', 'path': 'stress.ts'}}]
    for start in range(0, COUNT, BATCH):
        batch = []
        for n in range(start, min(COUNT, start + BATCH)):
            batch.append({'op': 'create_entity', 'scene_id': bindings.SCENE, 'entity': {
                'id': owner(n), 'name': f'Bulk stress {n}', 'parent': None, 'provenance': None,
                'components': {'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                               'Script': {'source': SCRIPT, 'props': {'ordinal': n}}}}})
        yield f'Author bulk stress ordinals {start}..{start + len(batch) - 1}', batch

def verify_extension(before, after):
    original = validate_base(before)
    entities = after['scenes'][bindings.SCENE]['entities']
    if len(entities) != len(original) + COUNT:
        raise ValueError('Core stress entity count differs')
    membership = []
    for n in range(COUNT):
        entity = entities[owner(n)]
        if set(entity['components']) != {'Transform', 'Script'} or entity['parent'] is not None:
            raise ValueError('Core stress must remain invisible unparented transforms only')
        if entity['components']['Transform'] != {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}:
            raise ValueError('Core stress initial transform differs')
        if entity['components']['Script'] != {'source': SCRIPT, 'props': {'ordinal': n}}:
            raise ValueError('Core stress attachment or ordinal mismatch')
        membership.append({'entity': owner(n), 'source': SCRIPT, 'ordinal': n})
    projected = copy.deepcopy(after)
    for n in range(COUNT):
        del projected['scenes'][bindings.SCENE]['entities'][owner(n)]
    del projected['scripts'][SCRIPT]
    if projected != before:
        raise ValueError('Core stress changed original project content')
    if sum('Script' in e['components'] for e in entities.values()) != 10_000:
        raise ValueError('Core stress requires exactly 10000 Script attachments')
    return membership

def observation(ticks, original_entities, removed_original=0):
    if type(removed_original) is not int or not 0 <= removed_original <= original_entities:
        raise ValueError('Invalid original-entity removal count')
    chosen = [0, 1, 96, 97, COUNT // 2, COUNT - 1]
    checks = [{'name': 'core-stress-population', 'tick': ticks, 'expect': {
        'type': 'entity_count', 'value': original_entities + COUNT - removed_original}}]
    for n in chosen:
        checks.append({'name': f'core-stress-{n}', 'tick': ticks, 'expect': {
            'type': 'vector', 'entity': owner(n), 'property': 'translation',
            'value': sentinel(ticks) if n == 0 else position(n, ticks), 'tolerance': 1e-9}})
    return {'format': 'incant-native-observation', 'version': 1, 'checks': checks, 'entities': [owner(n) for n in chosen]}

def prepare(project, output, binary, recordings):
    project = project.resolve(strict=True); binary = binary.resolve(strict=True)
    output = output.resolve()
    if output.is_relative_to(project.parent):
        raise ValueError('Output must be outside the protected input project')
    if output.exists():
        raise ValueError('Output already exists')
    protected = inventory(project.parent)
    if 'stress.ts' in protected or 'stress.journal.jsonl' in protected:
        raise ValueError('Input contains a prior stress extension')
    before = json.loads(project.read_text()); validate_base(before)
    if digest(project.parent / 'gameplay.ts') != digest(HERE / 'gameplay.ts'):
        raise ValueError('Core gameplay source differs from reviewed repository source')
    input_rows = {str(p.resolve(strict=True)): digest(p) for p in recordings}
    output.mkdir(parents=True)
    target = output / 'authoring'; shutil.copytree(project.parent, target)
    copied = target / project.name; calls = []; transactions = []; hashes = [hash_json(before)]
    session = Session(binary, copied, target / 'stress.journal.jsonl', calls, output / 'rpc-author')
    failure = None
    try:
        if session.call('project.read')['project'] != before:
            raise ValueError('Public project reopen changed original content')
        source = session.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data']['source']
        if source != (HERE / 'gameplay.ts').read_text():
            raise ValueError('Public original script identity mismatch')
        for description, batch in commands():
            transactions.append({'id': session.execute(description, batch), 'commands': len(batch), 'description': description})
            hashes.append(hash_json(session.call('project.read')['project']))
        after = session.call('project.read')['project']; membership = verify_extension(before, after)
        for expected in reversed(hashes[:-1]):
            session.call('history.undo')
            if hash_json(session.call('project.read')['project']) != expected:
                raise ValueError('Core stress undo mismatch')
        if (target / 'stress.ts').exists():
            raise ValueError('Source-writing transaction did not undo')
        for expected in hashes[1:]:
            session.call('history.redo')
            if hash_json(session.call('project.read')['project']) != expected:
                raise ValueError('Core stress redo mismatch')
        session.call('project.save'); expected = session.call('project.read')
    except BaseException as error:
        failure = repr(error); raise
    finally:
        try: session.close()
        finally: write(output / 'authoring.json', {'calls': calls, 'transactions': transactions, 'failure': failure})
    reopened = Session(binary, copied, target / 'stress.journal.jsonl', calls, output / 'rpc-reopen', timeout=180)
    try:
        if reopened.call('project.read') != expected:
            raise ValueError('Core stress saved reopen mismatch')
        if reopened.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] != (HERE / 'stress.ts').read_text():
            raise ValueError('Core stress source reopen mismatch')
    except BaseException as error:
        failure = repr(error); raise
    finally:
        try: reopened.close()
        finally: write(output / 'authoring.json', {'calls': calls, 'transactions': transactions, 'failure': failure})
    if inventory(project.parent) != protected:
        raise ValueError('Protected original project changed')
    for rel, sha in protected.items():
        if rel != project.name and not rel.startswith('.incant-source-transactions/') and digest(target / rel) != sha:
            raise ValueError('Copied original asset/source changed: ' + rel)
    write(output / 'membership.json', membership)
    receipt = {'format': 'incant-core-script-stress', 'version': 1, 'project': str(copied),
        'project_sha256': digest(copied), 'original_project_sha256': digest(project),
        'original_files': protected, 'authoring_files': inventory(target), 'input_recordings': input_rows,
        'script_sha256': {bindings.SCRIPT: digest(target / 'gameplay.ts'), SCRIPT: digest(target / 'stress.ts')},
        'membership_sha256': digest(output / 'membership.json'), 'scripted_entities': 10_000,
        'controller_instances': 1, 'stress_instances': COUNT, 'sources': [bindings.SCRIPT, SCRIPT],
        'original_entities': len(before['scenes'][bindings.SCENE]['entities']),
        'entities': len(after['scenes'][bindings.SCENE]['entities']), 'binary_sha256': digest(binary),
        'public_undo_redo_reopen': True, 'original_content_preserved': True,
        'performance_measured': False, 'gpu_submitted': False}
    write(output / 'receipt.json', receipt)
    return receipt

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path); parser.add_argument('output', type=Path)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--input', required=True, action='append', type=Path)
    args = parser.parse_args(); print(json.dumps(prepare(args.project, args.output, args.binary, args.input), indent=2))

if __name__ == '__main__':
    main()
