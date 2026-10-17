#!/usr/bin/env python3
"""Check retained native hierarchy records without starting an app or build."""
import base64
import gzip
import hashlib
import io
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(packet):
    data = {}
    for record in packet['records']:
        raw = base64.b64decode(record['base64'], validate=True)
        require(len(raw) == record['bytes'] and sha(raw) == record['sha256'], 'Record integrity')
        require(record['name'] not in data, 'Unique record')
        data[record['name']] = raw
    read = lambda name: json.loads(data[name])
    build, plan, result = read('build/result.json'), read('window/plan.json'), read('window/result.json')
    require(build['passed'] is True and result['passed'] is True and result['app_exit'] == 0, 'Successful build and native app')
    require(build['source'] == plan['source'] == packet['source'], 'Producing source')
    require(sha(data['build/plan.json']) == build['plan_sha256'], 'Build plan')
    require(sha(data['build/run.py']) == build['runner_sha256'], 'Build runner')
    require([stage['exit'] for stage in build['stages']] == [0, 0], 'Frontend and native builds')
    editor = next(a for a in build['artifacts'] if a['target']['name'] == 'incant_editor')
    require(editor['copied_sha256'] == plan['editor_sha256'], 'Native editor identity')
    require(result['preflight']['console_unlocked'] and result['preflight']['own_console_login_complete'], 'Unlocked desktop')
    require(0 < result['seconds'] <= plan['window_seconds'], 'Bounded native run')
    require(len(result['absence']) == 7, 'Seven owned native processes')
    pids = [r['pid'] for r in result['absence']]
    require(all(type(pid) is int and pid > 0 for pid in pids) and len(set(pids)) == 7, 'Seven distinct process identities')
    require(all(r['pid_absent'] is True and r['group_absent'] is True for r in result['absence']), 'Owned process cleanup')
    processes = read('window/setup-processes.json')
    require(len(processes) == 6 and all(p['exit'] == 0 for p in processes), 'Setup and fresh reopen')
    require(all(p['argv'][0] == plan['headless'] for p in processes), 'One headless producer')
    require(processes[0]['argv'][1] == 'init', 'Ordinary project creation')
    requests = [json.loads(p['input']) for p in processes[1:]]
    require([r['method'] for r in requests] == ['project.read', 'command.execute', 'project.save', 'project.read', 'project.read'], 'Shared command and persistence route')
    require(requests[1]['params']['expected_revision'] == 0, 'Initial transaction revision')
    require([c['op'] for c in requests[1]['params']['commands']] == ['create_entity'] * 3, 'Fixture authored through command bus')
    require(all('--journal' in p['argv'] for p in processes[1:]), 'Journal-backed RPC processes')
    before, after = read('window/before.json'), read('window/after.json')
    require(json.loads(processes[-2]['stdout'])['result'] == before, 'Recorded initial state')
    require(json.loads(processes[-1]['stdout'])['result'] == after, 'Recorded fresh-process state')
    require((before['revision'], after['revision']) == (1, 8), 'Native history revisions')
    require(read('window/project.json') == after['project'], 'Saved file and reopened state equality')
    old, new = before['project'], after['project']
    require(len(old['scenes']) == len(new['scenes']) == 1 and old['scenes'].keys() == new['scenes'].keys(), 'Same scene')
    require({k: v for k, v in old.items() if k != 'scenes'} == {k: v for k, v in new.items() if k != 'scenes'}, 'Unrelated project fields')
    old_scene, new_scene = next(iter(old['scenes'].values())), next(iter(new['scenes'].values()))
    require({k: v for k, v in old_scene.items() if k != 'entities'} == {k: v for k, v in new_scene.items() if k != 'entities'}, 'Scene metadata')
    original, final = old_scene['entities'], new_scene['entities']
    require(len(original) == 3 and len(final) == 5, 'One retained subtree copy')
    require(all(final[k] == v for k, v in original.items()), 'Original subtree and sibling unchanged')
    parent, child = '01M519KGFE7VGM7HH0MDED1Q8N', '01M519KGFEJZT9QTY1HKF44T2N'
    require(set(final) - set(original) == {parent, child}, 'Exact duplicate identities survive')
    require(final[parent]['parent'] is None and final[child]['parent'] == parent, 'Copied parent relation')
    for copied, original_id in [(parent, '00000000000000000000000010'), (child, '00000000000000000000000011')]:
        require(final[copied]['id'] == copied and final[copied]['name'] == original[original_id]['name'], 'Copied identity and name')
        require(final[copied]['components'] == original[original_id]['components'], 'Copied components')
    require(final[parent]['provenance'] == final[child]['provenance'], 'One duplicate transaction')
    require(final[parent]['provenance']['transaction'] != original['00000000000000000000000010']['provenance']['transaction'], 'New transaction provenance')
    notes = read('observations.json')
    require(notes['source'] == packet['source'] and notes['packet'] == plan['output'], 'Observation provenance')
    require(notes['no_screenshot'] is True and len(notes['steps']) == 9, 'Observation scope')
    return len(data)


def main():
    with (HERE / 'manifest.json').open('rb') as source:
        raw = source.read(65537)
    require(len(raw) <= 65536, 'Bounded manifest')
    manifest = json.loads(raw)
    with (HERE / 'records.json.gz').open('rb') as source:
        packed = source.read(1024 * 1024 + 1)
    require(len(packed) <= 1024 * 1024 and len(packed) == manifest['compressed_bytes'] and sha(packed) == manifest['sha256'], 'Bounded archive')
    with gzip.GzipFile(fileobj=io.BytesIO(packed)) as source:
        raw = source.read(1024 * 1024 + 1)
    require(len(raw) <= 1024 * 1024 and len(raw) == manifest['uncompressed_bytes'], 'Bounded contents')
    packet = json.loads(raw)
    require(all(packet[k] == manifest[k] for k in ('format', 'source', 'tree')), 'Packet identity')
    require(replay(packet) == manifest['records'], 'Record count')
    print(json.dumps({'passed': True, 'records': manifest['records'], 'native_executed': False}))


if __name__ == '__main__':
    main()
