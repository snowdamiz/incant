"""One trusted Core C-PGO functional experiment; never a public export mode."""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import shutil
import time

import export_native
import native_process
import native_profiling
from native_export_session import copy_authoring, inventory, managed_compile
from native_package import read, require, ulid_hex
from native_wasm import selection
from shipping_session import FreshSession
import quickjs_pgo as protocol

ROOT = Path(__file__).resolve().parents[1]
ROUTES = {'win': 2156, 'loss': 900, 'restart': 3056}


def request(path):
    value = json.loads(read(path, 256 * 1024))
    require(isinstance(value, dict) and set(value) == {'project', 'scene', 'camera', 'tools', 'routes'},
            'Closed trusted Core request')
    project = Path(value['project'])
    require(project.is_absolute() and project.resolve(strict=True) == project, 'Canonical Core project')
    document = json.loads(read(project, 64 * 1024**2))
    require(document['settings']['physics_mode'] == 'fast' and document['settings']['tick_rate'] == 60,
            'Preserve ordinary fast scalar Core')
    entities = document['scenes'][value['scene']]['entities']
    require(len(entities) == 10045 and sum('Script' in e['components'] for e in entities.values()) == 10000,
            'Existing Core10k authored workload required')
    require(selection(project, value['scene']) is None, 'QuickJS Core only')
    ulid_hex(value['scene']); ulid_hex(value['camera'])
    require(value['routes'].keys() == ROUTES.keys(), 'Exact win/loss/restart routes')
    for name, row in value['routes'].items():
        require(set(row) == {'input', 'observations'} and
                json.loads(read(row['input'], 4 * 1024**2))['ticks'] == ROUTES[name], 'Route input bound')
        read(row['observations'], 4 * 1024**2)
    tools = value['tools']
    require(set(tools) == {'compiler', 'archiver', 'profdata_tool', 'profile_runtime',
                          'sdk', 'resource_directory', 'tool_inputs'}, 'Closed tool inputs')
    for row in [tools[k] for k in ('compiler', 'archiver', 'profdata_tool', 'profile_runtime')] + tools['tool_inputs']:
        require(protocol.file_identity(Path(row['path'])) == row, 'Tool bytes changed')
    require(os.environ.get('SDKROOT') == tools['sdk'], 'Explicit selected SDKROOT required')
    paths = [Path(path), *[Path(row[key]) for row in value['routes'].values()
                          for key in ('input', 'observations')]]
    records = [protocol.file_identity(p, 4 * 1024**2) for p in paths]
    records += [tools[key] for key in ('compiler', 'archiver', 'profdata_tool', 'profile_runtime')]
    records += tools['tool_inputs']
    return value, {'project': project, 'scene': value['scene'], 'camera': value['camera'],
                   'inventory': inventory(project.parent), 'immutable_inputs': records}


def check_inputs(records):
    for row in records:
        require(protocol.file_identity(Path(row['path'])) == row, 'Experiment input changed')


def environment(values):
    for name, value in values.items():
        forbidden = (name.startswith(('CFLAGS', 'CPPFLAGS', 'LLVM_PROFILE_', 'INCANT_QUICKJS_', 'CARGO_PROFILE_'))
                     or name.endswith(('_CFLAGS', '_CPPFLAGS', '_RUSTFLAGS', '_CC', '_AR'))
                     or name.startswith(('CC_', 'AR_')) or name in ('CC', 'AR', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
                        'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTC',
                        'CARGO_BUILD_RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER'))
        require(not forbidden or not value, 'Unset ambient PGO/compiler override: ' + name)


class Session(FreshSession):
    def __init__(self, *args):
        super().__init__(*args)
        self.mode, self.directory, self.training = None, None, []
        self.last_stdout, self.last_stderr = '', ''

    def build(self, command, *, cwd, env, execute=None):
        require(self.mode in protocol.MODES and self.directory is not None, 'Unselected PGO phase')
        command = list(command)
        require(command[1] == 'build', 'Expected ordinary public build recipe')
        command[1] = 'rustc'
        command += ['--', '--print=link-args=' + str(self.directory / 'link-args.txt')]
        if self.mode == 'generate':
            command += ['-C', 'link-dead-code=yes']
        result, audited = super().build(command, cwd=cwd, env=env, execute=execute)
        self.last_stdout, self.last_stderr = result.stdout, result.stderr
        return result, audited

    def report(self):
        value = super().report()
        # Never publish the ordinary ledger's stronger before-any-player claim:
        # the explicit instrumented training child necessarily ran earlier.
        del value['build_storage_removed_before_any_player']
        del value['authoring_inputs_removed_before_any_player']
        value.update(build_storage_removed_before_baseline_and_use_validation=True,
                     authoring_inputs_removed_before_baseline_and_use_validation=True,
                     training_before_deletion=self.training,
                     scope='One bounded diagnostic session; nonshipping training precedes deletion; no performance gate')
        return value


@contextmanager
def phase(mode):
    features, symbols = native_profiling.features, export_native.runtime_symbol_audit

    def selected(profiling, route):
        require((profiling, route) == ('summary', 'headless'), 'CPU summary recipe only')
        return ['profile'] + ([] if mode == 'ordinary' else [protocol.FEATURES[mode]])

    def training(executable, kind, llvm, link_map):
        require(mode == 'generate' and kind == 'dsym', 'Training-only Mac symbol exception')
        raw = export_native.execute([llvm / 'llvm-nm', '--defined-only', executable]).stdout
        return protocol.training_symbols(raw)

    native_profiling.features = selected
    if mode == 'generate':
        export_native.runtime_symbol_audit = training
    try:
        yield
    finally:
        native_profiling.features = features
        export_native.runtime_symbol_audit = symbols


def role_receipts(messages, target, expected, directory, producer):
    """Require real sys and script build outputs in both Cargo dependency roles."""
    target = Path(target).resolve()
    found, build_ids = {}, set()
    expected_id = protocol.identity(expected)
    for row in messages:
        if row.get('reason') != 'build-script-executed':
            continue
        package = next((p for p, directory, version in
                        (('incant_script', 'crates', '0.0.1'), ('rquickjs-sys', 'third_party', '0.14.0'))
                        if row['package_id'] == 'path+' + (ROOT / directory / p).as_uri() + '#' + version), None)
        if package is None:
            continue
        out = Path(row['out_dir']).resolve()
        if out.is_relative_to(target / protocol.TARGET / 'shipping'):
            role = 'runtime'
        else:
            require(out.is_relative_to(target / 'shipping'), 'Unknown role output')
            role = 'cooker'
        data = json.loads(read(out / 'quickjs-pgo.json', 64 * 1024))
        require(data['format'] == 'incant-quickjs-pgo-build-v1' and data['package'] == package
                and data['mode'] == expected['mode'] and data['identity'] == expected_id
                and data['target'] == protocol.TARGET and data['default_c_flags'] is False
                and data['source_free_claim'] is False
                and data['out_dir'] == str(out) and data['compiler'] == expected['compiler']
                and data['descriptor_sha256'] == protocol.digest(directory / 'descriptor.json')
                and data['recipe'] == expected['recipe'] and data['profile'] == expected['profile']
                and data['actual_args'] == (None if package == 'incant_script' else
                    protocol.insert_flags(expected['recipe']['base_args'], protocol.additions(expected))),
                'Actual role/descriptor identity mismatch')
        key = (package, role)
        require(key not in found, 'Duplicate role receipt')
        found[key] = data
        if package == 'incant_script':
            env = dict(row['env'])
            require(env['INCANT_SCRIPT_DEFAULT_C_FLAGS'] == '0', 'Diagnostic ABI falsely marked default')
            build_ids.add(env['INCANT_SCRIPT_BUILD_ID'])
        protocol.create_json(directory / (package + '-' + role + '.json'), data)
    require(set(found) == {(p, r) for p in ('incant_script', 'rquickjs-sys') for r in ('runtime', 'cooker')}
            and len(build_ids) == 1 and producer['script']['build_id'] in build_ids
            and producer['script']['bytecode_abi']['default_c_flags'] is False,
            'Fresh host cook and linked runtime must have the same mode build identity')
    return {'descriptor_identity': expected_id, 'actual_build_id': next(iter(build_ids)),
            'roles': [{'package': p, 'role': r} for p, r in sorted(found)]}


def run_player(player, row, ticks, output, cancel, *, training=False):
    output.mkdir()
    pcm = output / 'audio.pcm'
    env = {'PATH': str(player.parent), 'TMPDIR': str(output), 'RAYON_NUM_THREADS': '2'}
    if training:
        raw = output / 'raw'
        raw.mkdir()
        env['LLVM_PROFILE_FILE'] = str(raw / 'training-%p.profraw')
    command = [player, '--package', player.parent / 'data', '--ticks', str(ticks), '--output', pcm,
               '--input-recording', row['input'], '--observe', row['observations'], '--ui-viewport', '640x480']
    with (output / 'stdout.json').open('x') as stdout, (output / 'stderr.log').open('x') as stderr:
        child = native_process.run(list(map(str, command)), cwd=player.parent, env=env,
                                   timeout=90, stdout=stdout, stderr=stderr, cancel=cancel)
    require(child.returncode == 0, 'Full Core replay failed')
    report = json.loads(read(output / 'stdout.json', 4 * 1024**2))
    require(report['observations']['passed'] is True
            and report['render_input']['gpu_submitted'] is False, 'Actual CPU Core route must pass')
    require(pcm.stat().st_size == ticks * 6400, 'Whole 48kHz stereo f32 PCM required')
    require(report['frames'] == ticks * 800 and report['pcm_sha256'] == protocol.digest(pcm),
            'Actual PCM sample frames/hash')
    require(report['tick'] == (2156 if ticks == 3056 else ticks), 'World tick chronology')
    if ticks == 3056:
        require(report['session'] == {'generation': 2, 'host_tick': 3056, 'simulation_tick': 2156,
            'restarts': [{'host_step': 900, 'completed_generation': 1, 'simulation_tick': 900, 'output_frames': '720000'}]},
            'Actual reset/host chronology')
    else:
        require('session' not in report, 'Unexpected session replacement')
    return report


def raw_training(directory):
    files = list(directory.iterdir())
    require(len(files) == 1 and re.fullmatch(r'training-[0-9]+\.profraw', files[0].name),
            'Exactly one owned training profile, no wildcard merge')
    return [protocol.file_identity(files[0], 64 * 1024**2)]


def run_tool(command, output, name, timeout, cancel):
    with (output / (name + '.stdout')).open('x') as stdout, (output / (name + '.stderr')).open('x') as stderr:
        result = native_process.run(list(map(str, command)), timeout=timeout, stdout=stdout, stderr=stderr, cancel=cancel)
    require(result.returncode == 0, 'Failed owned tool: ' + name)
    return read(output / (name + '.stdout'), 8 * 1024**2).decode()


def prepare(request_path, output, scratch, *, seconds=5400, cancel=lambda: None):
    require(type(seconds) is int and 60 <= seconds <= 7200, 'Finite Core experiment bound')
    environment(os.environ)
    spec, job = request(request_path)
    output, scratch = Path(output).absolute(), Path(scratch).absolute()
    require(all(not p.exists() and not p.is_symlink() and p.parent.is_dir() for p in (output, scratch))
            and not output.is_relative_to(scratch) and not scratch.is_relative_to(output)
            and all(not p.is_relative_to(job['project'].parent) for p in (output, scratch)), 'Fresh separate owned roots')
    began = time.monotonic()
    external_cancel = cancel

    def check():
        external_cancel()
        require(time.monotonic() - began < seconds, 'Core PGO deadline')

    output.mkdir(); scratch.mkdir()
    protocol.create_json(output / 'partial.json', {'complete': False, 'training_is_nonshipping': True})
    original_selector = os.environ.get(protocol.SELECTOR)
    try:
        rustc = Path(export_native.execute(['rustup', 'which', 'rustc']).stdout.strip())
        require('host: ' + protocol.TARGET in export_native.execute([rustc, '-Vv']).stdout, 'Native Mac only')
        sysroot = Path(export_native.execute([rustc, '--print', 'sysroot']).stdout.strip())
        llvm = sysroot / 'lib/rustlib' / protocol.TARGET / 'bin'
        # rustc adds its linker-tools bin; tools/cargo and export_native.build
        # each prepend the selected Rust bin. Bind the complete reported PATH,
        # not an ambient lookup of the recorded `env ... cc` command.
        link_environment = {'LC_ALL': 'C', 'PATH': os.pathsep.join(
            (str(llvm), str(rustc.parent), str(rustc.parent), os.environ['PATH'])),
            'SDKROOT': spec['tools']['sdk'], 'VSLANG': '1033', 'ZERO_AR_DATE': '1'}
        storage = scratch / 'build'; storage.mkdir()
        session = Session(ROOT, storage / 'target', rustc, protocol.TARGET, 'packed')
        copied = copy_authoring(job, scratch / 'authoring', check)
        session.input_directory(copied.parent)
        vendor = ROOT / 'third_party/rquickjs-sys'
        built, descriptors = {}, {}
        for index, mode in enumerate(protocol.MODES):
            check()
            check_inputs(job['immutable_inputs'])
            if index:
                session.begin_next_player()
            directory = output / mode; directory.mkdir()
            session.mode, session.directory = mode, directory
            if mode == 'ordinary':
                os.environ.pop(protocol.SELECTOR, None)
            else:
                fallback = scratch / 'compile-discard'; fallback.mkdir(exist_ok=True)
                descriptors[mode] = protocol.descriptor(mode, spec['tools'], base_args, vendor,
                    raw_directory=fallback if mode == 'generate' else None,
                    profile=profile if mode == 'use' else None)
                descriptor_path = scratch / (mode + '.json')
                protocol.create_json(descriptor_path, descriptors[mode])
                os.environ[protocol.SELECTOR] = str(descriptor_path)
                protocol.create_json(directory / 'descriptor.json', descriptors[mode])
            with phase(mode):
                player, report = export_native.build(copied, directory, storage,
                    spec['scene'], spec['camera'], 'headless', profiling='summary', session=session,
                    session_execute=managed_compile(directory, check))
            require(inventory(copied.parent, check) == job['inventory'], 'Build changed authoring input')
            require(report['physics_mode'] == 'fast' and report['program'] is None, 'Unchanged scalar fast bytecode route')
            if built:
                baseline = built['ordinary'][1]
                require(all(report[key] == baseline[key] for key in ('data', 'compiler', 'codegen', 'physics_mode'))
                        and report['physics']['features'] == baseline['physics']['features'],
                        'Ordinary package/compiler/physics contract changed')
            session.packaged_player(player)
            messages = [json.loads(line) for line in session.last_stdout.splitlines() if line.startswith('{')]
            report['pgo_feature_graph'] = protocol.artifact_features(messages, mode, session.path, ROOT)
            # Cargo prints build-script debug output in stdout, while rustc
            # invocations are in stderr. Retain and inspect both actual streams.
            commands = protocol.c_commands(session.last_stdout + '\n' + session.last_stderr, session.path, vendor,
                expected=descriptors.get(mode))
            protocol.create_json(directory / 'actual-c-roles.json', commands)
            if mode == 'ordinary':
                target = [c for c in commands if c['role'] == 'runtime']
                require(all(c['compiler'] == spec['tools']['compiler']['path'] for c in commands), 'Actual baseline tool differs')
                base_args = target[0]['args']
                require('-O3' in base_args and any(a in base_args for a in ('-g', '-g2', '-gdwarf-2')),
                        'Actual ordinary shipping optimization/debug recipe')
            else:
                producer = json.loads(read(directory / 'project-build.json', 4 * 1024**2))
                report['pgo_roles'] = role_receipts(messages, session.path, descriptors[mode], directory, producer)
            link = read(directory / 'link-args.txt', 1024**2).decode()
            report['actual_final_link'] = protocol.final_link(link, mode, spec['tools']['profile_runtime']['path'],
                compiler=spec['tools']['compiler']['path'], target=session.path, environment=link_environment)
            symbols = run_tool([llvm / 'llvm-nm', '--defined-only', player], directory, 'actual-symbols', 30, check)
            sections = run_tool([llvm / 'llvm-readobj', '--sections', player], directory, 'actual-sections', 30, check)
            report['instrumentation'] = protocol.instrumentation(mode, symbols, sections)
            report.update(training_is_nonshipping=mode == 'generate', eligible_for_distribution=False,
                          source_free_claim=mode != 'generate', performance_claim=False)
            protocol.create_json(directory / 'build-receipt.json', report)
            built[mode] = (player, report)
            if mode == 'generate':
                training = output / 'training'
                trained = run_player(player, spec['routes']['win'], ROUTES['win'], training, check, training=True)
                rows = raw_training(training / 'raw')
                session.training.append({'mode': mode, 'player_sha256': report['player_sha256'], 'raw_inputs': rows,
                                         'logical_report_sha256': protocol.digest(training / 'stdout.json'),
                                         'pcm_sha256': protocol.digest(training / 'audio.pcm')})
                profile = output / 'training.profdata'
                run_tool([spec['tools']['profdata_tool']['path'], 'merge', '--failure-mode=any', '-j', '2',
                    '-o', str(profile), *[r['path'] for r in rows]], output, 'profile-merge', 30, check)
                raw_names = run_tool([spec['tools']['profdata_tool']['path'], 'show', '--all-functions', '--counts', '--covered',
                    str(profile)], output, 'profile-covered', 30, check)
                covered = protocol.covered_names(raw_names)
                protocol.create_json(output / 'profile-inventory.json', {'inputs': rows, 'profile': protocol.file_identity(profile, 64*1024**2),
                    'covered': covered, 'compile_discard_not_merged': True, 'heldout_not_merged': True})
        require(protocol.file_identity(profile, 64 * 1024**2) == descriptors['use']['profile'],
                'Actual profile changed after use compilation')
        check_inputs(job['immutable_inputs'])
        shutil.rmtree(copied.parent)
        session.seal()
        protocol.create_json(output / 'session.json', session.report())
        # Preserve existing session's independent shared-unit origins, then retire
        # every remaining build/config/discard profile byte before validation.
        for path in storage.glob('build-*'):
            shutil.copy2(path, output / path.name)
        shutil.rmtree(scratch)
        require(not scratch.exists(), 'Complete deletion barrier')
        for name, ticks in ROUTES.items():
            reports = {}
            for mode in ('ordinary', 'use'):
                check(); session.assert_ready()
                reports[mode] = run_player(built[mode][0], spec['routes'][name], ticks,
                    output / (mode + '-' + name), check)
            require(reports['ordinary'] == reports['use'], 'Exact whole logical report parity')
            a, b = (output / (m + '-' + name) / 'audio.pcm' for m in ('ordinary', 'use'))
            require(read(a, ticks * 6400) == read(b, ticks * 6400), 'Exact entire PCM parity')
            if name == 'win':
                require(trained == reports['ordinary'] and read(output / 'training/audio.pcm', ticks * 6400) == read(a, ticks * 6400),
                        'Actual training semantics differ from ordinary Core')
        require(inventory(job['project'].parent, check) == job['inventory'], 'Original Core changed')
        check_inputs(job['immutable_inputs'])
        require(protocol.file_identity(profile, 64 * 1024**2) == descriptors['use']['profile'],
                'Actual profile changed after standalone validation')
        for mode, (player, report) in built.items():
            require(protocol.digest(player) == report['player_sha256'], 'Immutable player changed')
        result = {'complete': True, 'source': session.head.decode(), 'target': protocol.TARGET,
                  'routes': ROUTES, 'training_before_deletion': session.training,
                  'profile': descriptors['use']['profile'], 'immutable_inputs': job['immutable_inputs'],
                  'build_receipts': {mode: protocol.file_identity(output / mode / 'build-receipt.json')
                                     for mode in protocol.MODES},
                  'session': protocol.file_identity(output / 'session.json'),
                  'full_logical_and_PCM_parity': ['ordinary/use win', 'ordinary/use loss',
                                                 'ordinary/use restart', 'training/ordinary win'],
                  'no_performance_claim': True,
                  'engine_implementation': True, 'GPU_window_capture_executions': 0}
        protocol.create_json(output / 'manifest.json', result)
        return result
    except BaseException as error:
        protocol.create_json(output / 'failure.json', {'complete': False, 'error': repr(error), 'scratch_retained': scratch.exists()})
        raise
    finally:
        if original_selector is None:
            os.environ.pop(protocol.SELECTOR, None)
        else:
            os.environ[protocol.SELECTOR] = original_selector
