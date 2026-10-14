#!/usr/bin/env python3
"""SOURCE-REVIEWED PRIVATE BUILDER ONLY. A separate root grant precedes execution.

No player launch and no export.json. Normal export/cook/package/codegen helpers
remain unchanged. The exact extra diagnostic feature is admitted only here.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import tempfile
import time

from common import (ROOT, TARGET, FEATURES, authored, checked_file, compiler_features,
                    graph_delta, load, plan, source_check)
import export_native as ordinary
import native_process
import native_profiling
import physics_export
from physics_features import selected_graph_errors, validate_artifacts
from native_package import data_package, read, require, select_camera, sha256, write_json
from shipping_symbols import package


def build(plan_path, output, scratch_parent):
    data, plan_hash = plan(plan_path)
    source = source_check(data)
    project = authored(data)
    baseline = Path(data['baseline_package']).absolute()
    _, baseline_raw = checked_file(data['baseline_export'], 16 * 1024**2)
    require(data['baseline_export']['path'] == str(baseline / 'export.json'), 'Baseline path differs')
    base = json.loads(baseline_raw)
    base_info = load(baseline / 'project-build.json')
    _, baseline_graph = checked_file(data['baseline_graph'], 4 * 1024**2)
    require(sha256(read(baseline / 'project-build.json', 16 * 1024**2))
            == data['baseline_project_build_sha256'], 'Baseline producer changed')
    require(base['physics_mode'] == 'fast' and base['program'] is None
            and base['target'] == TARGET and base_info['tick_rate'] == 60, 'Baseline mode differs')
    require(physics_export.project_mode(read(project, 64 * 1024**2)) == 'fast', 'Core mode changed')
    expected_features = physics_export.cargo_features('fast', FEATURES)
    require(expected_features == FEATURES, 'Fast4 capability selection changed')
    native_profiling.environment('summary', os.environ)
    for name, value in os.environ.items():
        if (name in ('RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'RUSTC_WRAPPER',
                     'RUSTC_WORKSPACE_WRAPPER') or name.startswith('CARGO_PROFILE_')
                or name.endswith('_RUSTFLAGS')):
            require(not value, 'Unexpected compiler override: ' + name)
    compiler_bin = Path(data['toolchain_bin']).absolute()
    rustc, cargo = compiler_bin / 'rustc', compiler_bin / 'cargo'
    node, _ = checked_file(data['node'], 256 * 1024**2)
    require(node.name == 'node' and node.resolve(strict=True) == node and os.access(node, os.X_OK),
            'Expected the reviewed real Node executable')
    require(not os.environ.get('NODE_OPTIONS') and not os.environ.get('NODE_PATH'),
            'Unexpected Node injection environment')
    node_version = ordinary.execute([node, '--version'], timeout=10).stdout.strip()
    require(node_version == data['node']['version'] == 'v24.21.0', 'Node version differs')
    compiler = ordinary.execute([rustc, '-vV']).stdout
    require(compiler == base['compiler'] and re.search(r'^release: 1\.99\.0$', compiler, re.M)
            and re.search(r'^host: aarch64-apple-darwin$', compiler, re.M), 'Compiler differs')
    llvm = Path(ordinary.execute([rustc, '--print', 'sysroot']).stdout.strip()) / 'lib/rustlib' / TARGET / 'bin'
    output, scratch_parent = output.absolute(), scratch_parent.resolve(strict=True)
    require(str(output) == data['build_output'] and str(scratch_parent) == data['scratch_parent'],
            'Output/temporary storage differs from reviewed plan')
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(), 'Output exists')
    require(ROOT not in output.parents and project.parent not in output.parents, 'Output overlaps inputs')
    output.mkdir()
    state = {'format': 'incant-script-attribution-private-build-attempt', 'version': 1,
             'started_ns': time.time_ns(), 'plan_sha256': plan_hash, 'source': source,
             'status': 'running', 'player_launches': 0, 'eligible_for_performance_gate': False}
    write_json(output / 'attempt.json', state)
    env = {k: v for k, v in os.environ.items()
           if not k.startswith(('INCANT_', 'CARGO_', 'RUSTUP_', 'RAYON_'))}
    env.update(PATH=os.pathsep.join((str(compiler_bin), str(node.parent), os.environ.get('PATH', ''))),
               INCANT_NATIVE_PROJECT=str(project), RAYON_NUM_THREADS='2', CARGO_BUILD_JOBS='2',
               CARGO_NET_OFFLINE='true')
    require(shutil.which('node', path=env['PATH']) == str(node), 'Cook would select another Node')
    try:
        with tempfile.TemporaryDirectory(prefix='incant-script-attribution-', dir=scratch_parent) as temporary:
            storage = Path(temporary).resolve()
            target = storage / 'target'
            args = ['--locked', '--offline', '--no-default-features', '-p', ordinary.NAME,
                    '--features', ','.join(FEATURES), '--target', TARGET]
            graph = ordinary.execute([cargo, 'tree', *args, '-e', 'normal', '--prefix', 'none',
                                      '--format', '{p}|{f}'], env=env).stdout
            graph_change = graph_delta(graph, baseline_graph.decode())
            names = ordinary.normal_dependencies(graph)
            require(not selected_graph_errors(graph, 'fast', TARGET), 'Fast4 graph differs')
            (output / 'normal-dependencies.txt').write_text(graph)
            argv = [cargo, 'build', *args, '--bin', ordinary.NAME, '--profile', 'shipping',
                    '--target-dir', target, '-j2', '--config', 'profile.shipping.split-debuginfo="packed"',
                    '--message-format=json', '-vv']
            write_json(output / 'command.json', {'argv': list(map(str, argv)), 'cwd': str(ROOT),
                       'timeout_seconds': 2400, 'target': str(target), 'source': source,
                       'environment': {'RAYON_NUM_THREADS': '2', 'CARGO_BUILD_JOBS': '2',
                                       'CARGO_NET_OFFLINE': 'true', 'INCANT_NATIVE_PROJECT': str(project),
                                       'PATH': env['PATH']}, 'node': data['node']})
            with (output / 'cargo.jsonl').open('xb') as out, (output / 'build.log').open('xb') as err:
                result = native_process.run(list(map(str, argv)), cwd=ROOT, env=env,
                                            stdout=out, stderr=err, timeout=2400)
            state['build_exit'] = result.returncode
            require(result.returncode == 0, 'Build failed; attempt and raw logs retained')
            flags = ordinary.verify_codegen(read(output / 'build.log', 64 * 1024**2).decode(), TARGET, 'packed')
            require(flags == base['codegen'], 'Shipping codegen differs from baseline')
            messages = ordinary.cargo_messages(output / 'cargo.jsonl')
            actual_features = compiler_features(messages, target, base['profiling']['compiler']['features'])
            physics = validate_artifacts(messages, 'fast', target_dir=target, target=TARGET, profile='shipping')
            require(physics['features'] == base['physics']['features'], 'Fast4 compiler units differ')
            executables = [m for m in messages if m.get('reason') == 'compiler-artifact'
                           and m['target']['name'] == ordinary.NAME and m.get('executable')]
            require(len(executables) == 1, 'Expected unique fresh player')
            artifact = executables[0]
            producers = [Path(m['out_dir']) for m in messages if m.get('reason') == 'build-script-executed'
                         and m['package_id'] == artifact['package_id']
                         and (Path(m['out_dir']) / 'project-export.json').is_file()]
            require(len(producers) == 1, 'Missing unique actual cook output')
            producer = producers[0]
            info = load(producer / 'project-export.json')
            require(info['project_sha256'] == base['project_sha256'] == sha256(read(project, 64 * 1024**2))
                    and info['target'] == TARGET and info['startup_source_parsing'] is False
                    and info['program'] is None, 'Fresh cook identity changed')
            physics_export.verify_producer(info, 'fast')
            # Conservative build ID must remain genuine; every bytecode/source
            # field and all asset/scene identities stay equal to the baseline.
            before, after = dict(base_info['script']), dict(info['script'])
            old_id, new_id = before.pop('build_id'), after.pop('build_id')
            require(before == after, 'Compiler output, bytecode, bindings, map or ABI changed')
            require(len(new_id) == 64, 'Missing actual compatibility fingerprint')
            old_info, new_info = dict(base_info), dict(info)
            old_info.pop('script'); new_info.pop('script')
            require(old_info == new_info, 'Cooked scene/assets/producer identity changed')
            chosen = select_camera(info, None, False)
            symbol_audit = ordinary.runtime_symbol_audit(Path(artifact['executable']), 'dsym', llvm,
                                                        output / 'unused-link.map')
            player, symbols = package(Path(artifact['executable']), output, 'dsym', llvm)
            packaged = data_package(project.parent, producer, info, player.parent / 'data', chosen)
            require(packaged == base['data'], 'Exact cooked package data changed')
            proof = output / 'producer'; proof.mkdir()
            for name in ('project-export.json', 'script-symbols.json', 'behavior.js.map', 'behavior.rs'):
                p = producer / name
                if p.exists():
                    shutil.copy2(p, proof / name)
            generated = read(producer / 'behavior.rs', 16 * 1024**2).decode()
            arrays = re.findall(r'bytecode: &(\[[0-9, ]+\]), hardening: &(\[[0-9, ]+\]), bindings:', generated)
            require(len(arrays) == 1, 'Unexpected trusted compiler output layout')
            payload_sizes = 0
            for field, text in zip(('bytecode', 'hardening'), arrays[0]):
                values = json.loads(text)
                require(all(type(n) is int and 0 <= n <= 255 for n in values), 'Invalid compiler bytes')
                payload = bytes(values); payload_sizes += len(payload)
                require(sha256(payload) == base_info['script'][field + '_sha256'],
                        'Actual compiled payload changed: ' + field)
                (proof / (field + '.bin')).write_bytes(payload)
            require(payload_sizes == info['script']['bytecode_bytes'], 'Compiler byte count differs')
            require(sha256(read(proof / 'behavior.js.map', 16 * 1024**2))
                    == base_info['script']['source_map_sha256'], 'Actual compiler map changed')
            write_json(output / 'project-build.json', info)
            report = {'format': 'incant-script-attribution-private-build', 'version': 1,
                      'diagnostic_only': True, 'eligible_for_distribution': False,
                      'eligible_for_performance_gate': False, 'profile_version': 4,
                      'plan_sha256': plan_hash, 'source': source, 'compiler': compiler,
                      'node': data['node'], 'node_version_observed': node_version,
                      'target': TARGET, 'features': actual_features, 'graph_delta': graph_change,
                      'codegen': flags, 'physics': physics, 'normal_dependencies': names,
                      'player_sha256': sha256(read(player, 256 * 1024**2)), 'data': packaged,
                      'symbols': symbols, 'symbol_audit': symbol_audit, 'player_launches': 0,
                      'baseline_export_sha256': sha256(baseline_raw),
                      'compatibility': {'baseline_build_id': old_id, 'diagnostic_build_id': new_id,
                                        'all_other_script_metadata_equal': True},
                      'limits': ['Instrumented v4 is ineligible for normal v3 gates or distribution',
                                 'No player startup, functional execution or timing by this builder']}
        require(not storage.exists(), 'Owned temporary target not retired')
        require(authored(data) == project and source_check(data) == source, 'Inputs changed during build')
        checked_file(data['node'], 256 * 1024**2)
        report['build_storage_removed'] = True
        artifact_paths = [output / name for name in ('cargo.jsonl', 'build.log', 'normal-dependencies.txt', 'project-build.json')]
        artifact_paths += sorted((output / 'producer').iterdir())
        report['artifacts'] = {p.relative_to(output).as_posix(): sha256(read(p, 64 * 1024**2)) for p in artifact_paths}
        write_json(output / 'diagnostic-build.json', report)  # Last: no export.json ever.
        state['status'] = 'complete'
    except BaseException as error:
        state.update(status='failed', error=type(error).__name__ + ': ' + str(error))
        raise
    finally:
        state['ended_ns'] = time.time_ns()
        write_json(output / 'attempt.json', state)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('plan', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('scratch_parent', type=Path)
    args = parser.parse_args()
    build(args.plan, args.output, args.scratch_parent)
