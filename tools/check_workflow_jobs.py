#!/usr/bin/env python3
"""Require an exact set of GitHub Actions dependencies to actually succeed."""
import json
import os
import sys


def validate(needs, required):
    if not required or len(required) != len(set(required)):
        raise ValueError('Expected nonempty, unique required job names')
    if not isinstance(needs, dict) or set(needs) != set(required):
        raise ValueError('Workflow dependency set does not match required jobs')
    for name in required:
        job = needs[name]
        if not isinstance(job, dict) or job.get('result') != 'success':
            raise ValueError(f'Required job did not succeed: {name}')


def main():
    try:
        validate(json.loads(os.environ.get('INCANT_WORKFLOW_NEEDS', 'null')), sys.argv[1:])
    except (ValueError, TypeError) as error:
        raise SystemExit(str(error)) from error
    print('Every required dependency completed successfully.')


if __name__ == '__main__':
    main()
