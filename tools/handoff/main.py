#!/usr/bin/env python3
"""An ACP stdio client for isolated, reviewable visual handoffs. No credential IO."""
import argparse
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parents[2]

class HandoffError(Exception):
    pass

def packet_id(value):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}', value):
        raise HandoffError('Packet ID must contain only lowercase letters, numbers and hyphens')
    return value

def within(root, path):
    root = Path(root).resolve()
    path = Path(path)
    if not path.is_absolute():
        raise HandoffError('ACP filesystem paths must be absolute')
    path = path.resolve()
    if not path.is_relative_to(root):
        raise HandoffError('ACP filesystem access outside the handoff worktree refused')
    return path

def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args], text=True).strip()

def write_status(packet, status, detail):
    # Never persist adapter output, environment, prompts, or authentication responses.
    (packet / 'status.json').write_text(json.dumps({
        'status': status, 'detail': detail, 'updated_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
    }, indent=2) + '\n')

class AcpClient:
    def __init__(self, command, cwd, timeout=600):
        env = os.environ.copy()
        # Use the logged-in subscription; never accidentally bill a configured API key.
        for name in ('ANTHROPIC_API_KEY', 'ANTHROPIC_AUTH_TOKEN', 'CLAUDE_CODE_OAUTH_TOKEN'):
            env.pop(name, None)
        self.proc = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                     stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        self.root, self.timeout, self.counter = Path(cwd), timeout, 0
        self.print_updates = True
        self.messages = queue.Queue()
        def reader():
            try:
                for line in self.proc.stdout:
                    try:
                        self.messages.put(json.loads(line))
                    except json.JSONDecodeError:
                        self.messages.put(HandoffError('Adapter emitted invalid ACP JSON'))
                        return
            finally:
                self.messages.put(HandoffError('ACP adapter disconnected'))
        threading.Thread(target=reader, daemon=True).start()

    def send(self, value):
        self.proc.stdin.write(json.dumps(value) + '\n')
        self.proc.stdin.flush()

    def handle(self, msg):
        if 'id' not in msg:
            update = msg.get('params', {}).get('update', {})
            if self.print_updates and update.get('sessionUpdate') == 'agent_message_chunk':
                print(update.get('content', {}).get('text', ''), end='', flush=True)
            return
        method, params = msg.get('method'), msg.get('params', {})
        try:
            if method == 'fs/read_text_file':
                content = within(self.root, params['path']).read_text()
                lines = content.splitlines(keepends=True)
                start = max(0, params.get('line', 1) - 1)
                result = {'content': ''.join(lines[start:start + params.get('limit', len(lines))])}
            elif method == 'fs/write_text_file':
                path = within(self.root, params['path'])
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(params['content'])
                result = {}
            elif method == 'session/request_permission':
                options = params.get('options', [])
                result = {'outcome': {'outcome': 'cancelled'}}
                if sys.stdin.isatty():
                    call = params.get('toolCall', {})
                    # Print a reviewable command/path without duplicating large
                    # file content embedded in both rawInput and ACP diff blocks.
                    print('\nClaude requests:', json.dumps({
                        'name': call.get('name'), 'kind': call.get('kind'),
                        'title': call.get('title'),
                        'command': call.get('rawInput', {}).get('command'),
                        'path': call.get('rawInput', {}).get('file_path'),
                    }, indent=2))
                    for index, option in enumerate(options):
                        print(f'{index + 1}: {option["name"]}')
                    choice = input('Choose an option (Enter cancels): ')
                    if choice.isdecimal() and 0 < int(choice) <= len(options):
                        result = {'outcome': {'outcome': 'selected', 'optionId': options[int(choice)-1]['optionId']}}
            else:
                self.send({'jsonrpc': '2.0', 'id': msg['id'], 'error': {'code': -32601, 'message': 'Unsupported client method'}})
                return
            self.send({'jsonrpc': '2.0', 'id': msg['id'], 'result': result})
        except (OSError, KeyError, HandoffError):
            self.send({'jsonrpc': '2.0', 'id': msg['id'], 'error': {'code': -32603, 'message': 'Scoped filesystem operation failed'}})

    def request(self, method, params):
        self.counter += 1
        ident = self.counter
        self.send({'jsonrpc': '2.0', 'id': ident, 'method': method, 'params': params})
        deadline = time.monotonic() + self.timeout
        while True:
            try:
                msg = self.messages.get(timeout=max(0.001, deadline - time.monotonic()))
            except queue.Empty:
                raise HandoffError(f'ACP request timed out: {method}') from None
            if isinstance(msg, Exception):
                raise msg
            # A long handoff may keep making progress for hours. This is an
            # inactivity timeout, not a deadline that kills an active agent.
            deadline = time.monotonic() + self.timeout
            if msg.get('id') == ident and ('result' in msg or 'error' in msg):
                if 'error' in msg:
                    # Error bodies may include sensitive account details; expose only the code.
                    raise HandoffError(f'ACP {method} failed, code {msg["error"].get("code")}')
                return msg['result']
            self.handle(msg)

    def close(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()
        for stream in (self.proc.stdin, self.proc.stdout):
            stream.close()

def run(ident):
    packet = ROOT / 'handoffs' / packet_id(ident)
    if not (packet / 'brief.md').is_file():
        raise HandoffError('Missing handoffs/<id>/brief.md')
    config = json.loads((ROOT / 'tools/acp/config.json').read_text())
    claude = ROOT / config['claude']
    if not claude.exists():
        raise HandoffError('Run npm ci to install the pinned ACP adapter and Claude CLI')
    auth = subprocess.run([str(claude), 'auth', 'status'], capture_output=True, text=True)
    try:
        logged_in = json.loads(auth.stdout).get('loggedIn', False)
    except json.JSONDecodeError:
        logged_in = False
    if not logged_in:
        write_status(packet, 'blocked', 'Claude Code is not logged in. Run node_modules/.bin/claude auth login, then retry.')
        raise HandoffError('Claude Code is not logged in; packet preserved for manual execution')
    worktree = ROOT / '.worktrees' / ident
    branch = f'handoff/{ident}'
    if not worktree.exists():
        if git('status', '--porcelain', '--untracked-files=normal'):
            raise HandoffError('Commit the reviewed working tree before creating the handoff worktree')
        git('worktree', 'add', '-b', branch, str(worktree), 'HEAD')
    elif git('-C', str(worktree), 'branch', '--show-current') != branch:
        raise HandoffError('Existing worktree is on an unexpected branch')
    client = AcpClient([str(ROOT / config['adapter'])], worktree, config['request_timeout_seconds'])
    try:
        init = client.request('initialize', {'protocolVersion': config['protocol_version'],
            'clientInfo': {'name': 'incant-handoff', 'version': '0.1.0'},
            'clientCapabilities': {'fs': {'readTextFile': True, 'writeTextFile': True}, 'terminal': False}})
        if init.get('protocolVersion') != config['protocol_version']:
            raise HandoffError('ACP protocol version mismatch')
        session = None
        capabilities = init.get('agentCapabilities', {})
        if capabilities.get('loadSession') and 'list' in capabilities.get('sessionCapabilities', {}):
            listed = client.request('session/list', {'cwd': str(worktree)})
            # Some adapters return other directories despite a cwd filter.
            matches = [s for s in listed.get('sessions', []) if s.get('cwd') == str(worktree)]
            if matches:
                previous = max(matches, key=lambda s: s.get('updatedAt', ''))
                client.print_updates = False
                session = client.request('session/load', {'sessionId': previous['sessionId'], 'cwd': str(worktree), 'mcpServers': []})
                session['sessionId'] = previous['sessionId']
                client.print_updates = True
                print('Resumed the latest ACP session scoped to this handoff worktree.', flush=True)
        if session is None:
            session = client.request('session/new', {'cwd': str(worktree), 'mcpServers': []})
        model_config = next((x for x in session.get('configOptions', []) if x.get('category') == 'model'), None)
        if model_config:
            selected = next((x for x in model_config.get('options', [])
                             if x.get('name') == config['model_display_name']), None)
            if not selected:
                raise HandoffError('Required Claude 5.5 model is unavailable; no model substitution performed')
            client.request('session/set_config_option', {'sessionId': session['sessionId'],
                'configId': model_config['id'], 'value': selected['value']})
        else:
            models = session.get('models', {}).get('availableModels', [])
            matching = next((m for m in models if m.get('modelId') == config['model']), None)
            if not matching:
                raise HandoffError('Required Claude 5.5 model is unavailable; no model substitution performed')
            client.request('session/set_model', {'sessionId': session['sessionId'], 'modelId': matching['modelId']})
        result = client.request('session/prompt', {'sessionId': session['sessionId'], 'prompt': [{
            'type': 'text', 'text': f'Read CLAUDE.md and handoffs/{ident}/brief.md. Implement that packet in this worktree. '
            f'If files already exist from an interrupted attempt, inspect and finish them. '
            f'Return handoffs/{ident}/result.md with the exact model, evidence, screenshots, and limitations. '
            'Do not publish, merge, read credentials, change external accounts, or edit outside this worktree.'}]})
        result_path = worktree / 'handoffs' / ident / 'result.md'
        if result.get('stopReason') != 'end_turn' or not result_path.is_file():
            raise HandoffError('Handoff ended without a complete result packet')
        write_status(packet, 'review_required', f'Result is in .worktrees/{ident}/handoffs/{ident}/result.md. Review before integration.')
        print(f'\nReview: git diff --no-index handoffs/{ident} .worktrees/{ident}/handoffs/{ident}')
    except HandoffError:
        write_status(packet, 'blocked', 'ACP handoff did not complete. See terminal diagnosis; no credentials logged.')
        raise
    finally:
        client.close()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('run').add_argument('id', type=packet_id)
    args = parser.parse_args()
    try:
        run(args.id)
    except (HandoffError, subprocess.CalledProcessError) as exc:
        print(f'Handoff blocked: {exc}', file=sys.stderr)
        return 1
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
