"""fixture.py: read-only hash snapshots of the 0052 review fixture.

  fixture.py snap <label> <out.json>   every file under the fixture root: size, sha256, mtime
  fixture.py compare <a.json> <b.json> what changed between two snapshots

The fixture is project data, so it is only read here: no file is opened for writing, and the
three files the packet names (project, journal and behaviour source) are listed first.
"""

import datetime
import hashlib
import json
import os
import sys

ROOT = '/tmp/incant-animation-native-review-v2'
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


if __name__ == '__main__':
    if sys.argv[1:2] == ['snap'] and len(sys.argv) == 4:
        snap(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ['compare'] and len(sys.argv) == 4:
        compare(sys.argv[2], sys.argv[3])
    else:
        raise SystemExit(__doc__)
