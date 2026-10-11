#!/usr/bin/env python3
"""Run fresh local protocols and restart persistence, then remove only this fixture."""
from __future__ import annotations
import argparse
import hashlib
import json
import secrets
import shutil
import time

from local_services import Stack
from local_services_probe import require, run, signed_s3


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port-offset', type=int, default=3000)
    args = parser.parse_args()
    stack = Stack('proof-' + secrets.token_hex(6), args.port_offset)
    started = time.monotonic()
    stack.prepare()
    try:
        stack.validate_config()
        stack.compose('up', '-d', '--wait', '--wait-timeout', '180')
        health = stack.health()
        first = run(stack)
        require(signed_s3(stack, 'PUT', '/restart-proof')[0] == 200, 'restart bucket')
        payload = secrets.token_bytes(256)
        require(signed_s3(stack, 'PUT', '/restart-proof/bytes', payload)[0] == 200, 'restart object')
        stack.compose('exec', '-T', 'postgres', 'sh', '-c',
            'PGPASSWORD="$POSTGRES_PASSWORD" psql -h gateway -p 15432 -U incant -d incant -v ON_ERROR_STOP=1 -At',
            input="CREATE TABLE restart_proof(value text); INSERT INTO restart_proof VALUES ('persisted');")
        state_hash = hashlib.sha256((stack.state / 'settings.json').read_bytes()).digest()
        stack.compose('down')
        stack.prepare()
        require(hashlib.sha256((stack.state / 'settings.json').read_bytes()).digest() == state_hash, 'restart retained private state')
        stack.compose('up', '-d', '--wait', '--wait-timeout', '180')
        require(stack.health()['oidc_issuer'] == health['oidc_issuer'], 'restart issuer stable')
        output = stack.compose('exec', '-T', 'postgres', 'sh', '-c',
            'PGPASSWORD="$POSTGRES_PASSWORD" psql -h gateway -p 15432 -U incant -d incant -v ON_ERROR_STOP=1 -At',
            input='SELECT value FROM restart_proof;')
        require(output.strip() == 'persisted', 'Postgres named-volume persistence')
        status, _, data = signed_s3(stack, 'GET', '/restart-proof/bytes')
        require(status == 200 and data == payload, 'S3 named-volume exact-byte persistence')
        second = run(stack)
        require(first['checks'] == second['checks'], 'same protocols after restart')
        print(json.dumps({'scope': 'fresh local containers only', 'protocols_before': first,
            'protocols_after': second, 'restart_checks': ['private_state', 'oidc_issuer', 'postgres_row', 's3_bytes'],
            'elapsed_seconds': round(time.monotonic()-started, 2), 'hosted_gates_satisfied': False}, sort_keys=True))
    finally:
        # This name was freshly generated here; never run against a developer instance.
        stack.compose('down', '--volumes', '--remove-orphans')
        shutil.rmtree(stack.state)


if __name__ == '__main__':
    main()
