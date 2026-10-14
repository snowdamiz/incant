#!/usr/bin/env python3
"""Actual public profile exports and explicit local Tracy capture; no GUI/device.

Authoring is private and uses public commands. Existing-proof repeats only the
frozen packages after checking their identities; it never recooks the game.
"""
import argparse
import contextlib
import csv
import hashlib
import io
import json
import math
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import export_native
SCRIPTS = ['00000000000000000000000001', '00000000000000000000000002']
ENTITIES = ['00000000000000000000000003', '00000000000000000000000004']
TICKS = 3600
# Each source gets the full native bulk world; the two source writes accumulate.
SPEED = 6 + 12


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda: f.read(65536), b''):
            h.update(b)
    return h.hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n')


def inventory(root):
    return {str(p.relative_to(root)): digest(p) for p in sorted(root.rglob('*')) if p.is_file()}


class Runner:
    def __init__(self, root):
        self.root, self.commands = root, []
        (root / 'commands').mkdir()

    def record(self, name, value):
        write(self.root / 'commands' / (name + '.json'), value)
        self.commands.append(value)
        return value

    @contextlib.contextmanager
    def child(self, name, argv, *, receipt=None, **kwargs):
        """Trusted leaf tools only. Exporter's separate child sessions stay in-process."""
        argv = list(map(str, argv)); started = time.time_ns()
        stdout_path = self.root / 'commands' / (name + '.stdout')
        stderr_path = self.root / 'commands' / (name + '.stderr')
        failure = None
        process = None
        try:
            with stdout_path.open('x') as stdout, stderr_path.open('x') as stderr:
                process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, **kwargs)
                try:
                    yield process
                    require(process.poll() is not None, 'Owned leaf process not awaited')
                except BaseException as error:
                    failure = type(error).__name__
                    raise
                finally:
                    if process.poll() is None:
                        process.kill()
                    process.wait()
        finally:
            if process is not None:
                self.record(name, {'kind': 'process', 'argv': argv, 'pid': process.pid,
                    'exit': process.returncode, 'started_ns': started, 'ended_ns': time.time_ns(),
                    'stdout': stdout_path.read_text(), 'stderr': stderr_path.read_text(),
                    'stdout_sha256': digest(stdout_path), 'stderr_sha256': digest(stderr_path),
                    'exception': failure, **(receipt or {})})

    def run(self, name, argv, *, timeout=60, **kwargs):
        with self.child(name, argv, **kwargs) as process:
            process.wait(timeout=timeout)
        return self.commands[-1]

    def export(self, mode, project, output):
        # No outer child/timeout: each exporter child is already owned by
        # native_process. Interrupts unwind that cleanup before temporary files.
        name = 'export-' + mode; started = time.time_ns(); failure = None
        stdout_path = self.root / 'commands' / (name + '.stdout')
        stderr_path = self.root / 'commands' / (name + '.stderr')
        try:
            with stdout_path.open('x') as stdout, stderr_path.open('x') as stderr:
                with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                    return export_native.export(project, output, profiling=mode)
        except BaseException as error:
            failure = {'type': type(error).__name__, 'message': str(error)}
            raise
        finally:
            self.record(name, {'kind': 'in_process_export', 'entrypoint': 'export_native.export',
                'kwargs': {'project': str(project), 'output': str(output), 'profiling': mode},
                'host_pid': os.getpid(), 'exit': 0 if failure is None else 1, 'exception': failure,
                'started_ns': started, 'ended_ns': time.time_ns(),
                'stdout': stdout_path.read_text(), 'stderr': stderr_path.read_text(),
                'stdout_sha256': digest(stdout_path), 'stderr_sha256': digest(stderr_path),
                'ownership': 'Exporter native_process owns, drains and reaps each build/tool session; no outer timeout'})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--capture', type=Path, required=True)
    parser.add_argument('--csv', type=Path, required=True)
    parser.add_argument('--existing-proof', type=Path)
    args = parser.parse_args()
    out = args.output.resolve(); out.mkdir(parents=True, exist_ok=False)
    proof = args.existing_proof.resolve(strict=True) if args.existing_proof else out
    runner = Runner(out)
    capture, csv_tool = args.capture.resolve(strict=True), args.csv.resolve(strict=True)
    version = runner.run('csv-version', [csv_tool, '--version'])
    require(version['exit'] == 0 and version['stdout'].startswith('tracy-csvexport 0.14.0 / '), 'Wrong capture protocol')
    tools = {str(p): digest(p) for p in [capture, csv_tool]}
    fixture_path = ROOT / 'tools/probes/native-cpu-profile/behavior.ts'
    sources = [fixture_path.read_text(), fixture_path.read_text().replace('const speed = 6;', 'const speed = 12;')]
    if not args.existing_proof:
        require(args.binary is not None, 'Authoring binary required')
        binary = args.binary.resolve(strict=True)
        authoring = out / 'authoring'; authoring.mkdir()
        project = authoring / 'project.json'; journal = authoring / 'journal.jsonl'
        init = runner.run('init', [binary, 'init', project, '--entities', '0'])
        require(init['exit'] == 0, init['stderr'])
        original = json.loads(project.read_text()); scene = next(iter(original['scenes']))
        rate = original['settings']['tick_rate']
        rpc_type = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))['Rpc']
        rpc = rpc_type(binary, project, journal); calls = []
        def call(method, params=None):
            row = {'method': method, 'params': params}; calls.append(row)
            try:
                value = rpc.call(method, params)
                row['result'] = value; return value
            except BaseException as error:
                row['error'] = {'type': type(error).__name__, 'message': str(error)}
                raise
        try:
            for index, (script, entity, source) in enumerate(zip(SCRIPTS, ENTITIES, sources)):
                commands = [
                    {'op': 'write_script', 'script_id': script, 'expected_source': {'state': 'missing'}, 'source': source},
                    {'op': 'upsert_script', 'script': {'id': script, 'name': 'Numerical source', 'path': f'source-{index}.ts'}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': entity, 'name': 'Numerical entity',
                     'parent': None, 'provenance': None, 'components': {'Transform': {'translation': [0, 0, 0],
                     'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}, 'Script': {'source': script, 'props': {}}}}}]
                result = call('command.execute', {'expected_revision': index, 'description': 'Author one numerical source and attached entity', 'commands': commands})
                require(result['revision'] == index + 1, 'Source/entity transaction was not atomic')
            accepted = call('project.read')
            play = call('play.run', {'seconds': 20 / rate})
            require(play['passed'] and not play['state']['gpu_submitted'], 'Native CPU play failed')
            for entity in ENTITIES:
                actual = call('play.state_query', {'path': f'/entities/{entity}/transform/translation'})['untrusted_runtime_data']
                require(abs(actual[0] - SPEED * 20 / rate) < 1e-9, 'Native numerical motion differs')
            require(call('project.read') == accepted, 'Play mutated authoring')
            require(call('project.save')['saved'], 'Authoring checkpoint failed')
        finally:
            try:
                rpc.close()
            finally:
                if rpc.process.poll() is None:
                    rpc.process.kill()
                rpc.process.wait()
                write(out / 'authoring-calls.json', {'argv': list(map(str, rpc.process.args)),
                      'pid': rpc.process.pid, 'exit': rpc.process.returncode, 'calls': calls})
        authored = {'binary_sha256': digest(binary), 'project_sha256': digest(project), 'rate': rate,
                    'source_sha256': [hashlib.sha256(s.encode()).hexdigest() for s in sources]}
        write(out / 'authoring.json', authored)
        for mode in ['summary', 'tracy']:
            runner.export(mode, project, out / ('package-' + mode))
            exported = json.loads((out / ('package-' + mode) / 'export.json').read_text())
            require(exported['project_sha256'] == authored['project_sha256'], 'Exporter changed project')
        write(out / 'deleted-authoring.json', inventory(authoring)); shutil.rmtree(authoring)
        remove = runpy.run_path(str(ROOT / 'tools/probes/native-ui-export.py'))['remove_debug_source_maps']
        write(out / 'deleted-maps.json', {mode: remove(out / ('package-' + mode)) for mode in ['summary', 'tracy']})
    authored = json.loads((proof / 'authoring.json').read_text()); rate = authored['rate']
    require(authored['source_sha256'] == [hashlib.sha256(s.encode()).hexdigest() for s in sources], 'Probe source changed')
    require(not (proof / 'authoring').exists(), 'Source authoring survived')
    deleted = json.loads((proof / 'deleted-authoring.json').read_text())
    require(deleted['project.json'] == authored['project_sha256'], 'Wrong deleted project')
    require(all(deleted[f'source-{i}.ts'] == h for i, h in enumerate(authored['source_sha256'])), 'Wrong deleted sources')
    plans = {'format': 'incant-native-observation', 'version': 1, 'entities': ENTITIES,
             'checks': [{'name': f'motion-{i}', 'tick': TICKS, 'expect': {'type': 'vector', 'entity': entity,
                         'property': 'translation', 'value': [SPEED*TICKS/rate, 0, 0], 'tolerance': 1e-7}}
                        for i, entity in enumerate(ENTITIES)]}
    write(out / 'observe.json', plans)
    packages, players = {}, {}
    for mode in ['summary', 'tracy']:
        package = proof / ('package-' + mode); exported = json.loads((package / 'export.json').read_text())
        player = package / 'player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
        require(digest(player) == exported['player_sha256'], 'Player identity changed')
        require(exported['profiling']['mode'] == mode and exported['project_sha256'] == authored['project_sha256'], 'Export binding mismatch')
        require(exported['build_storage_removed_before_startup'] and not list(package.rglob('*.map')), 'Build/source maps survived')
        packages[mode] = inventory(package); players[mode] = player
    base_report, base_pcm = None, None
    summaries, listener_rows = {}, []

    def run_player(name, mode, flags, *, trace=False, on_listener=None):
        nonlocal base_report, base_pcm
        player = players[mode]; pcm = out / (name + '.pcm')
        argv = [player, '--package', player.parent / 'data', '--ticks', str(TICKS), '--observe', out / 'observe.json', '--output', pcm, *flags]
        argv = list(map(str, argv)); env = {'PATH': str(player.parent), 'TMPDIR': str(out), 'TEMP': str(out), 'TMP': str(out)}
        queries, network = 0, []
        details = {}; receiver_started = False
        with runner.child(name, argv, cwd=player.parent, env=env, receipt=details) as process:
            deadline = time.monotonic() + 60
            while process.poll() is None:
                require(time.monotonic() < deadline, 'Player exceeded bounded capture time')
                seen = subprocess.run(['/usr/sbin/lsof', '-nP', '-a', '-p', str(process.pid), '-i'], capture_output=True, text=True, timeout=5)
                require(seen.returncode in (0, 1), 'Listener inspection failed')
                queries += 1
                if seen.stdout: network.append(seen.stdout)
                if on_listener is not None and not receiver_started and '127.0.0.1:8086 (LISTEN)' in seen.stdout:
                    on_listener()
                    receiver_started = True
                details.update(network_queries=queries, network=network, capture_started_after_owned_listener=receiver_started)
                time.sleep(0.02)
        row = runner.commands[-1]; stdout, stderr = row['stdout'], row['stderr']
        require(row['exit'] == 0 and not stderr, row)
        require(queries > 0, 'No live process ownership check')
        if trace:
            require(network and any('127.0.0.1:8086 (LISTEN)' in s for s in network), 'No owned loopback listener observed')
            for s in network:
                for line in s.splitlines()[1:]:
                    require(' TCP ' in line and '127.0.0.1:' in line and '*:' not in line, 'Unexpected network endpoint')
            listener_rows.extend(network)
        else:
            require(not network, 'Noncapturing invocation created network endpoints')
        report = json.loads(stdout); profile = report.pop('profile', None)
        require(report['observations']['passed'] and report['tick'] == TICKS, 'Wrong completed run')
        require(digest(pcm) == report['pcm_sha256'] and pcm.stat().st_size == report['frames']*8, 'PCM binding failed')
        if base_report is None:
            base_report, base_pcm = report, digest(pcm)
        require(report == base_report and digest(pcm) == base_pcm, 'Profiling changed logical state or PCM')
        if flags:
            require(profile and profile['completed_frames'] == TICKS and profile['failed_frames'] == 0, 'Wrong profile frame counts')
            require(profile['tracy_process_transport'] is trace, 'Wrong transport state')
            require(profile['quickjs_heap']['samples'] == TICKS and profile['quickjs_heap']['latest']['estimated_used_bytes'] > 0, 'Missing VM heap samples')
            require(profile['rust_allocations']['installed'] and not profile['rust_allocations']['saturated'], 'Missing Rust allocation counter')
            require(profile['dropped_source_samples'] == 0 and len(profile['scripts']) == 2, 'Wrong bounded source table')
            require(all(s['totals']['calls'] == TICKS for s in profile['scripts']), 'Wrong source-group count')
            require({tuple(s['source']) for s in profile['scripts']} == {tuple(i.to_bytes(16,'big')) for i in (1,2)}, 'Source identity mismatch')
            require('PRIVATE-authored-marker' not in json.dumps(profile), 'Authored data leaked to profiler')
            summaries[name] = profile
        else:
            require(profile is None, 'Implicit profile reporting')
        return report

    for mode in ['summary', 'tracy']:
        run_player(mode + '-baseline', mode, [])
        run_player(mode + '-summary', mode, ['--profile', '--profile-heap-detail'])
    refusals = []
    for name, argv, diagnostic in [
        ('duplicate', [players['tracy'], '--profile', '--profile-tracy'], 'Error: Options\n'),
        ('summary-transport', [players['summary'], '--profile-tracy'], 'Error: TracyUnavailable\n'),
        ('unsupported', [players['tracy'], '--profile', '--build-info'], 'Error: UnsupportedRoute\n')]:
        result = runner.run('refusal-' + name, argv)
        require(result['exit'] != 0 and not result['stdout'] and result['stderr'] == diagnostic, result)
        refusals.append(name)
    failure_input = out / 'failure-input.json'
    write(failure_input, {'format': 'incant-input', 'version': 1, 'tick_rate': rate, 'start_tick': 0,
                         'ticks': 3, 'frames': [{'tick': 2, 'events': [{'type': 'key', 'code': 'F1', 'down': True}]}]})
    failed_diagnostic = None
    for index, flags in enumerate([[], ['--profile', '--profile-heap-detail']]):
        pcm = out / f'failed-{index}.pcm'; player = players['summary']
        result = runner.run(f'refusal-guest-{index}', [player, '--package', player.parent / 'data', '--ticks', '3',
                            '--input-recording', failure_input, '--output', pcm, *flags])
        require(result['exit'] != 0 and not result['stdout'] and not pcm.exists(), result)
        require('cpu-profiler deliberate trap' in result['stderr'], 'Wrong guest failure')
        if failed_diagnostic is None:
            failed_diagnostic = result['stderr']
        require(result['stderr'] == failed_diagnostic, 'Profiling changed the guest diagnostic')
        refusals.append('guest-' + str(index))
    require(not list(out.glob('.incant-pcm-*')), 'Failed guest left staging output')
    port = subprocess.run(['/usr/sbin/lsof','-nP','-iTCP:8086','-sTCP:LISTEN'], capture_output=True, text=True)
    require(port.returncode == 1 and not port.stdout, 'Refusing to use an occupied capture port')
    trace = out / 'capture.tracy'; cap_argv = list(map(str,[capture,'-a','127.0.0.1','-p','8086','-s','15','-m','1','-o',trace]))
    # Starting before the server creates a one-second reconnect race for short
    # offline games. Launch on the actual owned listener, without changing game
    # execution or inserting synthetic pauses into its measured work.
    with contextlib.ExitStack() as children:
        cap = None
        def start_receiver():
            nonlocal cap
            cap = children.enter_context(runner.child('capture', cap_argv))
        run_player('tracy-captured', 'tracy', ['--profile-tracy', '--profile-heap-detail'], trace=True, on_listener=start_receiver)
        require(cap is not None, 'No receiver launch against owned listener')
        cap.wait(timeout=30)
    result = runner.commands[-1]
    require(result['exit'] == 0 and trace.stat().st_size > 0, result)
    stats = runner.run('trace-zones', [csv_tool, trace]); require(stats['exit'] == 0, stats)
    events = runner.run('trace-events-plots', [csv_tool,'-u','-p',trace]); require(events['exit'] == 0, events)
    require('PRIVATE-authored-marker' not in stats['stdout']+events['stdout'], 'Authored string leaked to capture')
    zones = list(csv.DictReader(io.StringIO(stats['stdout'])))
    required = {'player.frame','native.tick','world.step','ecs.integrate','script.tick','script.source','game.audio_render'}
    require(required <= {r['name'] for r in zones if int(r['counts']) > 0}, 'Missing actual captured zones')
    rows = list(csv.DictReader(io.StringIO(events['stdout'])))
    plots = {name: [float(r['value']) for r in rows if r['name'] == name and not r['src_file']]
             for name in ['QuickJS sampled estimated used bytes','QuickJS sampled logical allocated bytes','Rust requested live bytes','Rust allocation calls']}
    require(all(values and all(math.isfinite(v) and v > 0 for v in values) for values in plots.values()), 'Missing/invalid actual plot samples')
    for mode in packages:
        require(inventory(proof / ('package-' + mode)) == packages[mode], 'Package changed during execution')
    write(out / 'result.json', {'passed': True, 'package_root': str(proof), 'source': authored, 'packages': packages, 'tools': tools,
        'commands': len(runner.commands), 'summaries': summaries, 'capture_sha256': digest(trace), 'zones': zones,
        'plot_samples': {k:len(v) for k,v in plots.items()}, 'owned_listener_samples': len(listener_rows),
        'refusals': refusals, 'guest_diagnostic': failed_diagnostic,
        'all_processes_reaped': all(r['exit'] is not None for r in runner.commands),
        'limits': ['desktop CPU functional diagnostics only', 'on-demand trace may begin after early frames',
                   'no GUI, GPU, phone, network-device capture or performance-gain claim']})


if __name__ == '__main__':
    main()
