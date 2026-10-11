#!/usr/bin/env python3
"""Start only Incant's isolated local service fixtures; never hosted resources."""
from __future__ import annotations
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import stat
import shutil
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
LOCAL = ROOT / 'services/local'
STATE_HOME = Path.home() / '.incant-local-services'
PORTS = {'PG': 15432, 'S3': 18333, 'OIDC': 18080, 'SMTP': 11025, 'MAIL': 18025}


def private_write(path: Path, content: str) -> None:
    with os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_NOFOLLOW', 0), 0o600), 'w', encoding='utf-8') as f:
        f.write(content)


def image_policy() -> dict:
    lock = json.loads((LOCAL / 'images.lock.json').read_text())
    if lock.get('schema') != 1 or set(lock['images']) != {'postgres', 'objects', 'oidc', 'mail', 'gateway'}:
        raise ValueError('Unknown local image manifest schema/service')
    for name, item in lock['images'].items():
        if not re.fullmatch(r'[a-z0-9._/-]+@sha256:[a-f0-9]{64}', item['image']):
            raise ValueError('Every local image must use an exact SHA-256 digest')
        if item['license'] not in lock['license_policy'].get(name, []):
            raise ValueError('Image license is outside this service-specific policy')
        license_file = LOCAL / item['license_file']
        if not license_file.resolve().is_relative_to((LOCAL / 'licenses').resolve()):
            raise ValueError('License must be retained within services/local/licenses')
        if hashlib.sha256(license_file.read_bytes()).hexdigest() != item['license_sha256']:
            raise ValueError('Retained license text does not match the lock')
    for relative, expected in lock['retained_notices'].items():
        path = LOCAL / relative
        if not path.resolve().is_relative_to((LOCAL / 'licenses').resolve()) or hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Retained notice is missing or changed')
    return lock


class Stack:
    def __init__(self, instance: str, port_offset: int = 0):
        if not re.fullmatch(r'[a-z][a-z0-9-]{0,31}', instance):
            raise ValueError('Instance must be a short lowercase identifier')
        if not 0 <= port_offset <= 40000:
            raise ValueError('Port offset must be between 0 and 40000')
        self.instance = instance
        self.project = 'incant-local-' + instance
        self.state = STATE_HOME / instance
        self.port_offset = port_offset
        self.values: dict[str, str] = {}
        self.child_env = {k: v for k, v in os.environ.items() if not k.startswith('INCANT_LOCAL_')}

    def redact(self, text: str) -> str:
        for key, value in self.values.items():
            if any(x in key for x in ('PASSWORD', 'SECRET', 'ACCESS_KEY')):
                text = text.replace(value, '[local fixture value]')
        return text

    def command(self, args: list[str], *, input: str | None = None) -> str:
        result = subprocess.run(args, input=input, capture_output=True, text=True, encoding="utf-8",
                                env=self.child_env, timeout=300)
        if result.returncode:
            raise RuntimeError(self.redact(result.stderr[-2500:]) or 'Local container command failed')
        return result.stdout

    def check_runtime(self) -> None:
        context = os.environ.get('DOCKER_CONTEXT')
        endpoint = None if context else os.environ.get('DOCKER_HOST')
        if not endpoint:
            args = ['docker', 'context', 'inspect'] + ([context] if context else [])
            endpoint = json.loads(self.command(args + ['--format', '{{json .Endpoints.docker.Host}}']))
        if not (endpoint.startswith('unix:///') or endpoint.startswith('npipe:////./pipe/')):
            raise ValueError('Local stack refuses remote Docker endpoints')
        self.command(['docker', 'compose', 'version'])

    def prepare(self) -> None:
        image_policy()
        self.check_runtime()
        for directory in [STATE_HOME]:
            if directory.is_symlink():
                raise ValueError('Local service state parents cannot be symlinks')
            directory.mkdir(exist_ok=True, mode=0o700)
            if os.name != 'nt' and stat.S_IMODE(directory.stat().st_mode) & 0o077:
                raise ValueError('Local service state parents must remain private to their owner')
        if self.state.is_symlink():
            raise ValueError('Local service state cannot be a symlink')
        if self.state.exists():
            self.load_state()
            return
        self.state.mkdir(mode=0o700)
        try:
            self.create_state()
        except BaseException:
            shutil.rmtree(self.state)
            raise

    def load_state(self) -> None:
        for path in [self.state, self.state / 'bootstrap', self.state / 'bootstrap/masterkey',
                     self.state / 'settings.json', self.state / '.env', self.state / 's3.json']:
            if path.is_symlink() or not path.exists():
                raise ValueError('Local state is incomplete or contains a symlink')
            if os.name != 'nt' and stat.S_IMODE(path.stat().st_mode) & 0o077:
                raise ValueError('Local state must remain private to its owner')
        data = json.loads((self.state / 'settings.json').read_text())
        if data.get('schema') != 1 or data['port_offset'] != self.port_offset:
            raise ValueError('Existing local state uses a different schema or port offset')
        values = data['values']
        expected = {'DB_PASSWORD', 'OIDC_DB_PASSWORD', 'ADMIN_PASSWORD', 'ACCESS_KEY', 'SECRET_KEY',
                    'STATE', 'UID', 'GID', 'PAT_EXPIRY'} | {key + '_PORT' for key in PORTS}
        if set(values) != expected or any(not isinstance(v, str) or '\n' in v or '\r' in v for v in values.values()):
            raise ValueError('Unexpected local state fields')
        if values['STATE'] != str(self.state) or any(values[key + '_PORT'] != str(port + self.port_offset) for key, port in PORTS.items()):
            raise ValueError('Local state paths or port mapping changed')
        for key in ['DB_PASSWORD', 'OIDC_DB_PASSWORD', 'SECRET_KEY']:
            if not re.fullmatch('[a-f0-9]{48}', values[key]):
                raise ValueError('Local generated value has an invalid format')
        if (not re.fullmatch('Local1![a-f0-9]{40}', values['ADMIN_PASSWORD'])
                or not re.fullmatch('incant[a-f0-9]{16}', values['ACCESS_KEY'])
                or not values['UID'].isdigit() or not values['GID'].isdigit()
                or not re.fullmatch(r'\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z', values['PAT_EXPIRY'])):
            raise ValueError('Local bootstrap fields have an invalid format')
        self.values = values
        if (self.state / '.env').read_text() != self.env_text():
            raise ValueError('Local state and environment file differ')
        if (self.state / 's3.json').read_text() != self.s3_text():
            raise ValueError('Local state and object credentials differ')

    def env_text(self) -> str:
        return ''.join('INCANT_LOCAL_' + key + '=' + value + '\n' for key, value in self.values.items())

    def s3_text(self) -> str:
        return json.dumps({'identities': [{'name': 'incant-local',
            'credentials': [{'accessKey': self.values['ACCESS_KEY'], 'secretKey': self.values['SECRET_KEY']}],
            'actions': ['Admin', 'Read', 'Write', 'List', 'Tagging']}]}) + '\n'

    def create_state(self) -> None:
        bootstrap = self.state / 'bootstrap'
        bootstrap.mkdir(mode=0o700)
        private_write(bootstrap / 'masterkey', secrets.token_hex(16))
        self.values = {
            'DB_PASSWORD': secrets.token_hex(24), 'OIDC_DB_PASSWORD': secrets.token_hex(24),
            'ADMIN_PASSWORD': 'Local1!' + secrets.token_hex(20),
            'ACCESS_KEY': 'incant' + secrets.token_hex(8), 'SECRET_KEY': secrets.token_hex(24),
            'STATE': str(self.state), 'UID': str(os.getuid() if hasattr(os, 'getuid') else 1000),
            'GID': str(os.getgid() if hasattr(os, 'getgid') else 1000),
            'PAT_EXPIRY': (dt.datetime.now(dt.UTC) + dt.timedelta(days=7)).strftime('%Y-%m-%dT%H:%M:%SZ'),
            **{key + '_PORT': str(port + self.port_offset) for key, port in PORTS.items()},
        }
        private_write(self.state / 's3.json', self.s3_text())
        private_write(self.state / '.env', self.env_text())
        private_write(self.state / 'settings.json', json.dumps({'schema': 1, 'port_offset': self.port_offset, 'values': self.values}, indent=2) + '\n')

    def compose(self, *args: str, input: str | None = None) -> str:
        return self.command(['docker', 'compose', '--project-name', self.project,
            '--env-file', str(self.state / '.env'), '-f', str(LOCAL / 'compose.yaml'), *args], input=input)

    def startup_status(self) -> list[dict]:
        """Return only bounded metadata, never service logs, commands or credentials."""
        raw = self.compose('ps', '--all', '--format', 'json')
        jobs = json.loads(raw) if raw.lstrip().startswith('[') else [json.loads(line) for line in raw.splitlines() if line.strip()]
        states = {'created', 'running', 'paused', 'restarting', 'removing', 'exited', 'dead'}
        health = {'', 'starting', 'healthy', 'unhealthy'}
        result = []
        for job in jobs:
            if job.get('Service') not in {'postgres', 'objects', 'oidc', 'mail', 'gateway'}:
                continue
            code = job.get('ExitCode')
            result.append({'service': job['Service'],
                'state': job.get('State') if job.get('State') in states else 'unknown',
                'health': job.get('Health') if job.get('Health') in health else 'unknown',
                'exit_code': code if type(code) is int and 0 <= code <= 255 else None})
        return sorted(result, key=lambda item: item['service'])

    def up(self) -> None:
        try:
            self.compose('up', '-d', '--wait', '--wait-timeout', '180')
        except (OSError, RuntimeError, subprocess.SubprocessError):
            # Bootstrap output may contain values not yet known to the redactor.
            # Do not print raw logs or the original command exception on failure.
            try:
                status = json.dumps(self.startup_status(), sort_keys=True)
            except (OSError, ValueError, KeyError, TypeError, RuntimeError, subprocess.SubprocessError):
                status = 'unavailable'
            raise RuntimeError('Local service startup failed; bounded status: ' + status) from None

    def validate_config(self) -> dict:
        config = json.loads(self.compose('config', '--format', 'json'))
        lock = image_policy()
        if (set(config['services']) != set(lock['images']) or set(config['networks']) != {'local', 'edge'}
                or not config['networks']['local']['internal']):
            raise ValueError('Local stack service/network boundary changed')
        for name, service in config['services'].items():
            if service['image'] != lock['images'][name]['image'] or service.get('privileged') or 'build' in service:
                raise ValueError('Local service must use its locked unprivileged image')
            expected_networks = {'local', 'edge'} if name == 'gateway' else {'local'}
            if set(service.get('networks', {})) != expected_networks or service.get('network_mode'):
                raise ValueError('Only the fixed gateway can join the edge network')
            if name != 'gateway' and service.get('ports'):
                raise ValueError('Backend services cannot publish host ports')
            if service.get('devices') or service.get('cap_add') or service.get('extra_hosts'):
                raise ValueError('Local containers cannot gain host access')
            for mount in service.get('volumes', []):
                if 'docker.sock' in mount.get('source', ''):
                    raise ValueError('Docker socket access is forbidden')
            for port in service.get('ports', []):
                if port['host_ip'] != '127.0.0.1':
                    raise ValueError('Local service ports must bind IPv4 loopback only')
        return config

    def url(self, name: str) -> str:
        return 'http://127.0.0.1:' + self.values[name + '_PORT']

    def health(self) -> dict:
        raw = self.compose('ps', '--format', 'json')
        jobs = json.loads(raw) if raw.lstrip().startswith('[') else [json.loads(line) for line in raw.splitlines() if line.strip()]
        if {j['Service'] for j in jobs if j.get('Health') == 'healthy'} != {'postgres', 'objects', 'oidc', 'mail', 'gateway'}:
            raise RuntimeError('Not every local service is healthy')
        for job in jobs:
            # Inspect only networking metadata; never dump container environments.
            view = json.loads(self.command(['docker', 'inspect', '--format',
                '{"ports":{{json .NetworkSettings.Ports}},"networks":{{json .NetworkSettings.Networks}}}', job['ID']]))
            expected = {self.project + '_local'} | ({self.project + '_edge'} if job['Service'] == 'gateway' else set())
            if set(view['networks']) != expected:
                raise ValueError('Running service network boundary differs from Compose')
            bindings = [binding for values in view['ports'].values() if values for binding in values]
            if job['Service'] == 'gateway':
                if ({b['HostPort'] for b in bindings} != {self.values[key + '_PORT'] for key in PORTS}
                        or any(b['HostIp'] != '127.0.0.1' for b in bindings)):
                    raise ValueError('Running gateway ports are not exactly loopback-bound')
            elif bindings:
                raise ValueError('Running backend published a host port')
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with opener.open(self.url('OIDC') + '/.well-known/openid-configuration', timeout=10) as r:
            discovery = json.load(r)
        if discovery['issuer'] != self.url('OIDC'):
            raise ValueError('OIDC discovery advertised an unexpected issuer')
        return {'local_only': True, 'healthy': sorted(j['Service'] for j in jobs),
                'postgres': '127.0.0.1:' + self.values['PG_PORT'], 'objects': self.url('S3'),
                'oidc_issuer': discovery['issuer'], 'mail_capture': self.url('MAIL')}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['up', 'health', 'down', 'check', 'probe'])
    parser.add_argument('--instance', default='dev')
    parser.add_argument('--port-offset', type=int, default=0)
    args = parser.parse_args()
    try:
        stack = Stack(args.instance, args.port_offset)
        stack.prepare()
        stack.validate_config()
        if args.action == 'up':
            stack.up()
        if args.action in ('up', 'health'):
            print(json.dumps(stack.health(), sort_keys=True))
        elif args.action == 'probe':
            from local_services_probe import run
            print(json.dumps(run(stack), sort_keys=True))
        elif args.action == 'down':
            stack.compose('down')
            print('Local containers stopped; generated data volumes retained.')
        else:
            print('Locked images, retained licenses, local runtime and loopback/internal network verified.')
    except (OSError, ValueError, KeyError, RuntimeError, subprocess.SubprocessError) as error:
        print(stack.redact(str(error)) if 'stack' in locals() else str(error), file=sys.stderr)
        raise SystemExit(1) from None


if __name__ == '__main__':
    main()
