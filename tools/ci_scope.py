#!/usr/bin/env python3
"""Select only the explicitly reviewed documentation-only general-check path."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = frozenset({
    'PLAN.md', 'STATUS.md', 'AGENTS.md', 'CLAUDE.md',
    'docs/conventions/shared.md', 'docs/conventions/astra.md',
    'docs/conventions/claude.md',
    'docs/adr/0012-accounts.md', 'docs/adr/0014-ci.md',
})
SHA = re.compile(r'[0-9a-f]{40}')


def revision(value):
    if not isinstance(value, str) or not SHA.fullmatch(value) or value == '0' * 40:
        raise ValueError('missing or invalid revision')
    return value


def git(*args, root=ROOT):
    return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.PIPE)


def changes(raw):
    """Parse unambiguous, non-renamed raw Git records, including both file modes."""
    fields = raw.split(b'\0')
    if not fields or fields[-1] != b'' or len(fields) % 2 != 1:
        raise ValueError('invalid raw diff')
    result = []
    for index in range(0, len(fields) - 1, 2):
        metadata = fields[index].decode('ascii').split()
        if len(metadata) != 5 or not metadata[0].startswith(':'):
            raise ValueError('invalid diff metadata')
        before, after, old, new, status = metadata
        if not re.fullmatch(r':[0-7]{6}', before) or not re.fullmatch(r'[0-7]{6}', after):
            raise ValueError('invalid file mode')
        if not SHA.fullmatch(old) or not SHA.fullmatch(new):
            raise ValueError('invalid object identity')
        result.append((fields[index + 1].decode('utf-8'), status, before[1:], after))
    return result


def classify(event_name, event, root=ROOT):
    if event_name == 'pull_request':
        base = revision(event['pull_request']['base']['sha'])
        head = revision(event['pull_request']['head']['sha'])
        base = revision(git('merge-base', base, head, root=root).decode().strip())
    elif event_name == 'push':
        base, head = revision(event['before']), revision(event['after'])
        git('merge-base', '--is-ancestor', base, head, root=root)
    else:
        raise ValueError('event requires full validation')
    raw = git('diff', '--raw', '-z', '--no-abbrev', '--no-renames', '--no-ext-diff', base, head, root=root)
    records = changes(raw)
    safe = bool(records) and all(
        path in DOCUMENTS and new == '100644' and
        ((status == 'M' and old == '100644') or (status == 'A' and old == '000000'))
        for path, status, old, new in records
    )
    return {'scope': 'docs' if safe else 'full', 'base': base, 'head': head,
            'reason': 'explicit documentation inputs only' if safe else 'empty, executable, unknown or unsupported change',
            'changed': [path for path, _, _, _ in records]}


def select(event_name, event, root=ROOT):
    try:
        return classify(event_name, event, root)
    except (KeyError, TypeError, ValueError, UnicodeError, OSError, subprocess.SubprocessError):
        # Do not print potentially sensitive subprocess stderr or event contents.
        return {'scope': 'full', 'base': '', 'head': '', 'reason': 'classification unavailable; full validation required', 'changed': []}


def validate_documents(base, head, root=ROOT):
    base, head = revision(base), revision(head)
    for name in DOCUMENTS:
        path = root / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f'required regular document missing: {name}')
        text = path.read_text(encoding='utf-8')
        if not text.strip() or '\0' in text:
            raise ValueError(f'invalid document text: {name}')
    # Keep whitespace validation tied to exactly the classified change set.
    git('diff', '--check', base, head, root=root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    select_parser = sub.add_parser('select')
    select_parser.add_argument('--full', action='store_true', help='Always execute the full release-checkpoint suite')
    check = sub.add_parser('validate-docs')
    check.add_argument('--base', required=True)
    check.add_argument('--head', required=True)
    args = parser.parse_args()
    if args.command == 'validate-docs':
        validate_documents(args.base, args.head)
        return
    try:
        event = json.loads(Path(os.environ['GITHUB_EVENT_PATH']).read_text(encoding='utf-8'))
        result = select(os.environ.get('GITHUB_EVENT_NAME', ''), event)
    except (KeyError, ValueError, OSError):
        result = select('', {})
    if args.full:
        result['scope'] = 'full'
        result['reason'] = 'release checkpoint requires comprehensive validation'
    # Fixed scope literals and validated hexadecimal revisions only enter outputs.
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
        for name in ['scope', 'base', 'head']:
            output.write(f'{name}={result[name]}\n')
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
