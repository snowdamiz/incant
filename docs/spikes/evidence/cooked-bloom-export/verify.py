#!/usr/bin/env python3
"""Replay retained public-export evidence; never run a compiler, player or device."""
import argparse
import base64
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess


def require(ok, message):
    if not ok:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def file_digest(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='Also hash retained original records and large raw logs')
    parser.add_argument('--binaries', action='store_true', help='Also hash the external package, authoring and installed compiler inputs')
    parser.add_argument('--git-source', help='Require the exact producing source inventory in this Git revision')
    parser.add_argument('--source-root', type=Path, help='Additionally require unchanged live producing inputs')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    repo = here.parents[3]
    manifest = json.loads((here / 'manifest.json').read_text())
    archive = (here / 'records.json.gz').read_bytes()
    require(digest(archive) == manifest['archive_sha256'], 'Archive differs')
    with gzip.open(here / 'records.json.gz', 'rb') as stream:
        payload = stream.read(24 * 1024**2 + 1)
    require(len(payload) <= 24 * 1024**2, 'Archive exceeds bound')
    encoded = json.loads(payload)
    rows = manifest['records']
    require(len({r['key'] for r in rows}) == len(rows) and set(encoded) == {r['key'] for r in rows}, 'Record membership differs')
    raw = {}
    for row in rows:
        value = base64.b64decode(encoded[row['key']], validate=True)
        require(len(value) == row['bytes'] and digest(value) == row['sha256'], 'Record differs: ' + row['key'])
        raw[row['key']] = value
        if args.originals:
            require(Path(row['original']).stat().st_size == len(value) and file_digest(row['original']) == row['sha256'],
                    'Original differs: ' + row['key'])
    external = {r['path']: r for r in manifest['external_raw_logs']}
    if args.originals:
        for row in external.values():
            require(Path(row['path']).stat().st_size == row['bytes'] and file_digest(row['path']) == row['sha256'], 'External raw log differs')

    def data(key):
        return json.loads(raw[key])

    source = manifest['source']
    require(source == '340acb7cfe7da8cc7c959f60730feb937b0a9d31', 'Producing revision differs')
    freeze = data('shipping-source-002/freeze.json')
    require(freeze['source'] == source and len(freeze['records']) == 4490, 'Source inventory differs')
    source_rows = freeze['records']
    require(len({r['path'] for r in source_rows}) == len(source_rows), 'Repeated source path')
    if args.source_root or args.originals:
        for row in source_rows:
            if args.source_root:
                require(file_digest(args.source_root / row['path']) == row['sha256'], 'Live source differs: ' + row['path'])
            if args.originals:
                require(Path(row['copy']).stat().st_size == row['bytes'] and file_digest(row['copy']) == row['sha256'],
                        'Frozen source copy differs: ' + row['path'])
    if args.git_source:
        queries = ''.join(args.git_source + ':' + r['path'] + '\n' for r in source_rows).encode()
        result = subprocess.run(['git', 'cat-file', '--batch'], cwd=repo, input=queries, capture_output=True, check=True)
        offset = 0
        for row in source_rows:
            end = result.stdout.index(b'\n', offset)
            header = result.stdout[offset:end].split()
            require(len(header) == 3 and header[1] == b'blob', 'Missing Git source: ' + row['path'])
            size = int(header[2]); offset = end + 1
            blob = result.stdout[offset:offset + size]; offset += size + 1
            require(size == row['bytes'] and digest(blob) == row['sha256'], 'Git source differs: ' + row['path'])
        require(offset == len(result.stdout), 'Unexpected Git batch output')

    calls_by_run = {}
    for run, count, passed in [('shipping-run-002', 9, False), ('shipping-run-003', 18, True),
                               ('node-setup-run-001', 1, False), ('node-setup-run-002', 2, True)]:
        receipt = data(run + '/result.json')
        calls = calls_by_run[run] = data(run + '/managed-calls.json')
        require(receipt['passed'] is passed and len(calls) == count and receipt['calls_finished'], 'Attempt chronology differs: ' + run)
        for call in calls:
            require(call['call_finished'] and call['cleanup_confirmed'] and not call['cleanup_failures']
                    and call['cleanup_records'] and all(r['original_stop_returned'] for r in call['cleanup_records']),
                    'Cleanup is unconfirmed: ' + run)
            for name in ('stdout', 'stderr'):
                item = call.get(name)
                if not item:
                    continue
                if item.get('merged_into') == 'stdout':
                    require(name == 'stderr' and 'path' in call['stdout'], 'Invalid merged stream')
                    continue
                key = run + '/' + Path(item['path']).name
                if key in raw:
                    require(len(raw[key]) == item['bytes'] and digest(raw[key]) == item['sha256'], 'Managed stream differs')
                else:
                    require(external.get(item['path'], {}).get('sha256') == item['sha256']
                            and external[item['path']]['bytes'] == item['bytes'], 'External stream binding differs')
    require(calls_by_run['shipping-run-002'][-1]['exit_code'] == 101
            and b"authored TypeScript requires the development checkout's pinned Node dependencies"
            in raw['shipping-run-002/call-008.stderr.raw.log'], 'Missing dependency failure lost')
    require(b'ENOTCACHED' in raw['node-setup-run-001/call-000.stderr.raw.log'], 'Offline setup failure lost')
    require(data('node-setup-partial-001.json') == [], 'No-child absent-directory observation differs')
    setup = data('node-setup-run-002/result.json')
    require(setup['compiler_smoke'] == {'typescript': '5.9.3', 'swc': '1.16.13', 'sourcemap': '1.6.0',
                                      'sourceTransform': True, 'projectCodeExecuted': False}, 'Actual compiler imports differ')
    install = calls_by_run['node-setup-run-002'][0]['argv']
    require(install[-5:] == ['ci', '--ignore-scripts', '--no-audit', '--no-fund', '--workspaces=false']
            and '--offline' not in install, 'Authorized pinned setup differs')
    node_inputs = data('node-setup-run-002/installed-inputs.json')
    require(len(node_inputs) == 7620 and digest(raw['node-setup-run-002/installed-inputs.json']) == setup['installed_inputs_sha256'], 'Compiler inventory differs')

    final = data('shipping-run-003/result.json')
    plan = data('shipping-proposal-003.json')
    require(final['source_sha'] == source and final['proposal_sha256'] == digest(raw['shipping-proposal-003.json'])
            and final['supervisor_sha256'] == digest(raw['shipping-supervisor-003.py']), 'Final execution binding differs')
    require(b'Ran 10 tests' in raw['shipping-controls-003.log'] and b'\nOK' in raw['shipping-controls-003.log'],
            'Managed lifecycle/cleanup controls differ')
    require(plan['source_freeze_sha256'] == digest(raw['shipping-source-002/freeze.json'])
            and plan['jobs'] == 2 and plan['Rayon_threads'] == 2 and plan['Cargo_timeout_seconds'] == 2400, 'Source/tool bounds differ')
    require(plan['bounds'] == {'seconds': 3000, 'floor': 6 * 1024**3, 'scratch_cap': 8 * 1024**3,
                              'log_cap': 128 * 1024**2, 'file_cap': 64 * 1024**2, 'output_cap': 512 * 1024**2}, 'Resource envelope differs')
    require(all(c['exit_code'] == 0 for c in calls_by_run['shipping-run-003']), 'Final child failure')
    storage = data('shipping-run-003/storage.json')
    require(max(s['scratch_bytes'] for s in storage) == 4324769042
            and max(s['scratch_bytes'] for s in storage) <= plan['bounds']['scratch_cap']
            and min(s['available_bytes'] for s in storage) >= plan['bounds']['floor']
            and max(s['log_bytes'] for s in storage) <= plan['bounds']['log_cap'], 'Observed resource bound differs')
    close = data('shipping-run-003/closeout.json')
    require(close['supervisor_terminal_reaped'] and close['CPU_builder_released'] and close['matching_live_pid_or_group'] == []
            and close['private_scratch_entries'] == [] and close['cleanup_confirmed_calls'] == 18
            and close['result_sha256'] == digest(raw['shipping-run-003/result.json']), 'Final closeout differs')
    require(final['remaining_private_entries'] == [] and final['authoring_unchanged']
            and close['source_and_frozen_copies_unchanged'] == 4490 and close['installed_compiler_entries_unchanged'] == 7620
            and close['authoring_files_unchanged'] == 74, 'Input immutability differs')
    export = data('public-cooked-window-export-003/export.json')
    require(final['export_receipt_sha256'] == digest(raw['public-cooked-window-export-003/export.json'])
            and export['player_sha256'] == final['player_sha256'] == manifest['qualification']['player_sha256'], 'Published identity differs')
    require(export['target'] == 'aarch64-apple-darwin' and export['mode'] == 'window' and export['physics_mode'] == 'fast'
            and export['build_storage_removed_before_startup'] and export['startup_source_parsing'] is False, 'Export admission differs')
    startup = export['startup']
    native_calls = [c for c in calls_by_run['shipping-run-003'] if Path(c['argv'][0]).name == 'incant_game_player']
    require(len(native_calls) == 1 and '--window' not in native_calls[0]['argv']
            and native_calls[0]['argv'][native_calls[0]['argv'].index('--ticks') + 1] == '1'
            and startup == json.loads(raw['shipping-run-003/call-017.stdout.raw.log'])
            and startup['tick'] == 1 and startup['frames'] == 800
            and startup['render_input']['gpu_submitted'] is False and startup['render_input']['mode'] == 'cpu-only', 'CPU-only startup differs')
    graph = raw['public-cooked-window-export-003/normal-dependencies.txt']
    require(digest(graph) == final['selected_normal_graph_sha256'], 'Selected graph differs')
    forbidden = {'incant_doc', 'incant_cmd', 'incant_agent', 'incant_core', 'incant_cook', 'incant_shader_cook',
                 'incant_source', 'incant_play', 'incant_play_legacy', 'incant_wasm_cook', 'incant_wasm_dev', 'loro', 'automerge'}
    names = []
    for line in graph.decode().splitlines():
        match = re.fullmatch(r'([A-Za-z0-9_-]+) v[^|]+\|([^|]*)', line.removesuffix(' (*)'))
        require(match is not None, 'Malformed normal graph')
        name = match[1]; names.append(name)
        require(name not in forbidden and not name.startswith(('wasmtime', 'cranelift')), 'Authoring/JIT normal dependency')
    require(names[0] == 'incant_game_player' and sorted(set(names)) == export['normal_dependencies'], 'Selected dependency set differs')
    compiler_lines = [line for line in raw['shipping-run-003/call-008.stderr.raw.log'].decode().splitlines()
                      if 'Running `' in line and '--target aarch64-apple-darwin' in line]
    for name in ('incant_runtime', 'incant_script', 'incant_game_player_library', 'incant_game_player'):
        expected = {'opt-level': '3', 'codegen-units': '1', 'panic': 'abort', 'debuginfo': '2', 'split-debuginfo': 'packed'}
        if name == 'incant_game_player':
            expected['lto'] = 'fat'
        require(export['codegen'][name] == expected, 'Shipping codegen differs: ' + name)
        crate = 'incant_game_player' if name == 'incant_game_player_library' else name
        kind = 'bin' if name == 'incant_game_player' else 'lib'
        lines = [line for line in compiler_lines if re.search(r'--crate-name ' + crate + r'\b', line)
                 and re.search(r'--crate-type ' + kind + r'(?:\s|$)', line)]
        require(len(lines) == 1 and '--test' not in lines[0] and 'feature="compiler"' not in lines[0],
                'Unique shipping rustc invocation missing: ' + name)
        flags = {key: value.strip('"\'') for key, value in re.findall(r'(?:^|\s)-C\s*([a-z-]+)=([^\s`]+)', lines[0])}
        require(all(flags.get(k) == v for k, v in expected.items()) and flags.get('target-cpu') != 'native',
                'Raw rustc flags differ: ' + name)

    shaders = export['shaders']
    expected_families = {'output', 'diagnostic', 'shadow', 'material', 'sprite', 'ui', 'text', 'cluster',
                         'bloom-prefilter', 'bloom-downsample', 'bloom-upsample', 'bloom-display'}
    require(set(shaders['families']) == expected_families and shaders['mode'] == 'cooked' and shaders['profile'] == 'shipping'
            and shaders['runtime_wgsl_in_selected_ordinary_catalog'] is False, 'Cooked selection differs')
    require(shaders['compiler_features']['incant_render'] == plan['expected_renderer_features']
            and sorted(shaders['compiler_features']['incant_game_player']) == ['cooked-native-shaders', 'native-window', 'project', 'render-gpu', 'render-input'],
            'Actual target features differ')
    require(shaders['families'] == data('cpu-005/actual-shaders.json')['families'], 'Actual fresh audits differ from retained CPU shader bytes')
    prior_manifest = data('prior-cpu-packet-manifest.json')
    prior_path = here / manifest['prior_shader_bytes']['manifest']
    require(prior_path.read_bytes() == raw['prior-cpu-packet-manifest.json'], 'Prior CPU packet changed')
    prior_archive = (prior_path.parent / 'records.json.gz').read_bytes()
    require(digest(prior_archive) == prior_manifest['archive_sha256'] == manifest['prior_shader_bytes']['archive_sha256'], 'Prior shader archive differs')
    with gzip.open(prior_path.parent / 'records.json.gz', 'rb') as stream:
        prior_payload = stream.read(32 * 1024**2 + 1)
    require(len(prior_payload) <= 32 * 1024**2, 'Prior archive exceeds bound')
    prior = json.loads(prior_payload)
    count = total = 0
    for family, value in shaders['families'].items():
        prefix = 'cpu-005/generated/' + family + '-shaders/'
        for item in value['artifacts']:
            shader = base64.b64decode(prior[prefix + item['file']], validate=True)
            require(len(shader) == item['bytes'] and digest(shader) == item['sha256'], 'Retained shader differs')
            count += 1; total += len(shader)
        for filename, field in [('audit.json', 'audit_sha256'), ('manifest.rs', 'manifest_sha256')]:
            require(digest(base64.b64decode(prior[prefix + filename], validate=True)) == value[field], 'Shader audit/manifest differs')
    require(count == 69 and total == 268438, 'Artifact totals differ')
    if args.binaries:
        package = Path(plan['package'])
        inventory = {str(p.relative_to(package)): file_digest(p) for p in package.rglob('*') if p.is_file()}
        require(inventory == final['package_inventory'] and len(inventory) == 38
                and sum((package / p).stat().st_size for p in inventory) == final['package_bytes'], 'External package differs')
        for row in manifest['binaries']:
            require(Path(row['path']).stat().st_size == row['bytes'] and file_digest(row['path']) == row['sha256'], 'External player differs')
        author = Path(plan['project']).parent
        require({str(p.relative_to(author)): file_digest(p) for p in author.rglob('*') if p.is_file()}
                == data('authoring-freeze.json')['authoring_files'], 'Authoring differs')
        for row in node_inputs:
            path = Path(freeze['worktree']) / 'node_modules' / row['path']
            if 'symlink' in row:
                require(path.is_symlink() and os.readlink(path) == row['symlink'], 'Installed compiler symlink differs')
            else:
                require(not path.is_symlink() and path.stat().st_size == row['bytes'] and file_digest(path) == row['sha256'], 'Installed compiler input differs')
    print(f'PASS {len(rows)} records; public export; 12 families/69 shaders; CPU tick1/800 PCM sample frames; 18 cleanups; failures retained')


if __name__ == '__main__':
    main()
