"""fixture.py: read-only hash snapshots of the retained v2 review fixture, for 0053.

  fixture.py snap <label> <out.json>   every file under the fixture root: size, sha256, mtime
  fixture.py compare <a.json> <b.json> what changed between two snapshots
  fixture.py verify <snap.json>        that snapshot against the fixture manifest named in
                                       build.json: the same files, no extras, equal hashes

Copied from 0052, plus `verify`. The fixture is project data, so it is only read here: no file
is opened for writing, and the three files the packet names (project, journal and behaviour
source) are listed first.
"""

import datetime
import hashlib
import json
import os
import sys

ROOT = '/tmp/incant-animation-native-review-v2'
MANIFEST = '/tmp/incant-animation-native-review-v2-manifest.json'
MANIFEST_SHA256 = '069789bef712fa54c17ffab6a19415581f06bc18fb85ea7d84bc1449913056f2'
NAMED = ('project.json', 'project.journal.jsonl', 'animation.ts')


def snap(label, out):
    files = {}
    for directory, _, names in os.walk(ROOT):
        for name in names:
            path = os.path.join(directory, name)
            with open(path, 'rb') as handle:
                digest = hashlib.sha256(handle.read()).hexdigest()
            info = os.stat(path)
            files[os.path.relpath(path, ROOT)] = {
                'sha256': digest, 'bytes': info.st_size, 'mtime_ns': info.st_mtime_ns,
            }
    record = {
        'label': label,
        'at': datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='milliseconds'),
        'root': ROOT,
        'named': {name: files[name]['sha256'] for name in NAMED},
        'files': dict(sorted(files.items())),
    }
    with open(out, 'w', encoding='utf-8') as handle:
        json.dump(record, handle, indent=1)
        handle.write('\n')
    print(json.dumps({'label': label, 'files': len(files), 'named': record['named']}))


def compare(a_path, b_path):
    a = json.load(open(a_path, encoding='utf-8'))
    b = json.load(open(b_path, encoding='utf-8'))
    added = sorted(set(b['files']) - set(a['files']))
    removed = sorted(set(a['files']) - set(b['files']))
    changed = sorted(k for k in set(a['files']) & set(b['files']) if a['files'][k]['sha256'] != b['files'][k]['sha256'])
    touched = sorted(k for k in set(a['files']) & set(b['files'])
                     if a['files'][k]['sha256'] == b['files'][k]['sha256'] and a['files'][k]['mtime_ns'] != b['files'][k]['mtime_ns'])
    print(json.dumps({'from': a['label'], 'to': b['label'], 'named_equal': a['named'] == b['named'],
                      'added': added, 'removed': removed, 'changed': changed, 'mtime_only': touched}, indent=1))


def verify(snap_path):
    with open(MANIFEST, 'rb') as handle:
        raw = handle.read()
    manifest_sha = hashlib.sha256(raw).hexdigest()
    expected = json.loads(raw)['files']
    actual = {k: v['sha256'] for k, v in json.load(open(snap_path, encoding='utf-8'))['files'].items()}
    result = {
        'manifest_sha256_ok': manifest_sha == MANIFEST_SHA256,
        'manifest_files': len(expected),
        'snapshot_files': len(actual),
        'missing': sorted(set(expected) - set(actual)),
        'extra': sorted(set(actual) - set(expected)),
        'differ': sorted(k for k in set(expected) & set(actual) if expected[k] != actual[k]),
    }
    result['ok'] = result['manifest_sha256_ok'] and not (result['missing'] or result['extra'] or result['differ'])
    print(json.dumps(result))
    if not result['ok']:
        raise SystemExit(1)


if __name__ == '__main__':
    if sys.argv[1:2] == ['snap'] and len(sys.argv) == 4:
        snap(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ['compare'] and len(sys.argv) == 4:
        compare(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ['verify'] and len(sys.argv) == 3:
        verify(sys.argv[2])
    else:
        raise SystemExit(__doc__)
