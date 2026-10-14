#!/usr/bin/env python3
"""Replay retained MSAA evidence offline; never runs a renderer, app or build."""
import argparse
import base64
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess

HERE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def replay(manifest, data):
    read = lambda name: json.loads(data[name])
    originals = {r['original']: r['name'] for r in manifest['records']}
    original = lambda path: data[originals[path]]
    sources = manifest['executed_sources']
    artifacts = {}
    for version, stages in [('001', 1), ('003', 2), ('005', 6), ('006', 6), ('007', 3)]:
        prefix = 'cpu/cpu-'
        receipt = read(prefix + 'source' + version + '-results.json')
        launch = read(prefix + 'launch-' + version + '.json')
        proposal_raw = original(launch['proposal'])
        proposal = json.loads(proposal_raw)
        require(receipt['source_sha'] == proposal['source_sha'] == sources['cpu' + version], 'CPU source identity')
        require(sha(proposal_raw) == launch['proposal_sha256']
                and sha(data[prefix + 'supervisor-' + version + '.py']) == launch['supervisor_sha256'], 'CPU protocol identities')
        require(not any(launch['ambient_override_values'].values()), 'Ambient override refusal')
        require('rustc 1.99.0' in launch['identities']['rustc']['stdout']
                and all(r['exit_code'] == 0 for r in launch['identities'].values()), 'Actual compiler/SDK capture')
        require(len(receipt['results']) == stages, 'Actual CPU stage count')
        for index, row in enumerate(receipt['results']):
            log = original(row['log'])
            require(sha(log) == row['log_sha256'] and row['reaped'] is True, 'CPU raw log and reaping')
            require(row['argv'] == proposal['commands'][index]['argv']
                    and row['timeout_seconds'] == 900, 'CPU exact command and bound')
            require(row['available_bytes_after'] >= 6 * 1024**3
                    and row['target_bytes_after'] - proposal['resource_bounds']['lease_target_bytes_before'] < 2 * 1024**3,
                    'CPU recorded resource bounds')
            expected_exit = 101 if version == '001' or (version == '003' and index == 1) else 0
            require(row['exit_code'] == expected_exit, 'CPU distinct original outcome')
        for row in receipt.get('copied_artifacts', []):
            artifacts[row['copy']] = row
        if version == '001':
            require(b'test result: FAILED. 5 passed; 3 failed' in original(receipt['results'][0]['log']),
                    'Original interpolation/observer failures retained')
        if version == '003':
            log = original(receipt['results'][0]['log'])
            require(sum(map(int, re.findall(rb'test result: ok\. (\d+) passed; 0 failed', log))) == 34,
                    'Original source003 cooker34')
            errors = []
            for line in original(receipt['results'][1]['log']).splitlines():
                try:
                    value = json.loads(line)
                except ValueError:
                    continue
                if value.get('reason') == 'compiler-message' and value['message']['level'] == 'error':
                    errors.append(value['message']['message'])
            require(any('`default`' in e and 'AmbientOcclusion' in e for e in errors), 'Original renderer compile refusal')
        if version in ('005', '006'):
            for row, count in zip(receipt['results'][:3], (175, 189, 178)):
                require(re.findall(rb'test result: ok\. (\d+) passed; 0 failed', original(row['log'])) == [str(count).encode()],
                        'Default/cooked/native renderer CPU counts')
        if version == '007':
            require(all('--no-run' in r['argv'] for r in receipt['results'][:2])
                    and receipt['results'][2]['argv'][-2:] == ['-D', 'warnings'], 'Source007 compile/lint scope')
    # Eleven actual children include the original failure and two superseded
    # positive timing controls. Final acceptance uses eight distinct controls.
    for version, count, passed in [('001', 5, False), ('002', 4, True), ('003', 2, True)]:
        prefix = 'gpu/gpu-'
        receipt = read(prefix + version + '/result.json')
        proposal = read(prefix + 'proposal-' + version + '.json')
        launch = read(prefix + version + '/launch.json')
        require(receipt['source_sha'] == proposal['source_sha'] == sources['gpu' + version]
                and receipt['passed'] is passed and len(receipt['results']) == count, 'GPU distinct source/outcome')
        require(sha(data[prefix + 'proposal-' + version + '.json']) == launch['proposal_sha256']
                and sha(data[prefix + 'supervisor-' + version + '.py']) == launch['supervisor_sha256'], 'GPU protocol identities')
        require(proposal['limits']['per_child_seconds'] == 120
                and proposal['limits']['sequence_execution_seconds'] == 300, 'GPU cooperative bounds')
        for index, row in enumerate(receipt['results']):
            arm = proposal['sequence'][index]
            log = data[prefix + version + '/' + row['name'] + '/output.log']
            require(row['argv'] == arm['argv'] and row['reaped'] is True
                    and row['exit_code'] == (101 if version == '001' and index == 4 else 0), 'GPU exact child and reaping')
            require(sha(log) == row['log_sha256'] and b'backend: Metal' in log
                    and b'Apple M5 Pro' in log, 'GPU actual backend/raw log')
            require(artifacts[arm['binary']]['sha256'] == row['binary_sha256'] == arm['binary_sha256']
                    and artifacts[arm['binary']]['source_sha'] == receipt['source_sha'], 'Copied qualified GPU artifact')
            for output in row['outputs']:
                require(sha(original(output['path'])) == output['sha256'], 'Actual GPU output identity')
            if row['name'].startswith('timing-order'):
                samples = [json.loads(line[len('MSAA timing '):]) for line in log.decode().splitlines()
                           if line.startswith('MSAA timing ')]
                require(len(samples) == 2 and [s['sequence'] for s in samples] == [1, 2], 'Actual supported timestamp samples')
                text, resolve, display = samples[0]['passes'][5:8]
                require(len(samples[0]['passes']) == 8 and text['end_ticks'] <= resolve['start_ticks']
                        <= resolve['end_ticks'] <= display['start_ticks'], 'Real post-text/pre-display resolve ordering')
                require(samples[1]['passes'][6] is None, 'Pixel-grid1x resolve absent')
        if version == '001':
            require(b'retained native diagnostic cubes are not implemented' in
                    data[prefix + version + '/ownership-default/output.log'], 'Original retained-fixture refusal')
    for name, raw in data.items():
        if name.startswith('gpu/') and name.endswith('.json'):
            value = json.loads(raw)
            if isinstance(value, dict) and 'rgba_sha256' in value:
                pixels = data[str(PurePosixPath(name).parent / value['raw_file'])]
                require(sha(pixels) == value['rgba_sha256'] and len(pixels) == value['rgba_bytes']
                        == value['size'][0] * value['size'][1] * 4, 'Raw bounded RGBA binding')
    palette = []
    for case, expected in [('one', (14683, [94, 53, 242, 181], 3)), ('four', (14899, [94, 52, 242, 181], 13))]:
        a = data['gpu/gpu-001/cube-default/raw/cube-' + case + '.rgba']
        b = data['gpu/gpu-001/cube-cooked/raw/cube-' + case + '.rgba']
        require(a == b, 'Actual standard/cooked cube pixels identical')
        rgba = list(zip(*[iter(a)] * 4))
        changed = [i for i, p in enumerate(rgba) if p != (20, 21, 25, 255)]
        bounds = [min(i % 320 for i in changed), min(i // 320 for i in changed),
                  max(i % 320 for i in changed) + 1, max(i // 320 for i in changed) + 1]
        values = len(Counter(rgba))
        require((len(changed), bounds, values) == expected, 'Supporting cube coverage/palette data')
        palette.append(values)
    for mode in ('default', 'cooked'):
        prefix = 'gpu/gpu-002/ownership-' + mode + '/raw/'
        require(read(prefix + 'cleared.json')['changed_from_clear_pixels'] == 0
                and data[prefix + 'viewport-0.rgba'] == data[prefix + 'viewport-2.rgba'], 'Clear and queued old attachment retention')
    # Root owns actual editor/CUA/pixel review; keep the original review wording.
    link = read('editor/editor-link-001/result.json')
    window = read('editor/editor-window-001/result.json')
    review = read('editor/editor-window-001/review.json')
    require(link['source'] == manifest['production_source'] and link['exit_code'] == window['exit_code'] == 0
            and link['managed_cleanup_returned'] and window['managed_cleanup_returned'], 'Actual editor link/window terminal')
    require(link['artifact']['sha256'] == window['binary_sha256'] == review['editor_executable_sha256'], 'Actual editor identity')
    require(sha(data['editor/editor-link-001/build.log']) == link['log_sha256']
            and sha(data['editor/editor-window-001/editor.png']) == review['editor_image_sha256']
            and sha(data['editor/editor-window-001/result.json']) == review['editor_window_result_sha256'], 'Root actual image/receipts')
    require(all(not data['editor/editor-window-001/' + r['file']] for r in window['logs']), 'Actual editor empty streams')
    require(review['decision'].startswith('Accept scoped ordinary editor cube silhouette improvement'), 'Scoped root pixel review')
    development = read('root-development/results.json')
    require(len(development) == 9 and all(r['returncode'] == 0 for r in development), 'Root local development checks')
    require(b'973 passed' in data['root-development/7.log'], 'Root973 UI tests')
    closure = read('source-closure.json')
    require(closure['production_to_final_delta'] == ['crates/incant_render/src/scene_msaa/device_tests.rs']
            and not closure['integration_native_input_delta'] and not closure['cooker_source003_to_final_delta'], 'Production/cooker/main-intake source relevance')
    inventory = read(manifest['compiler_inventory']['record'])
    lookup = {r['path']: r for r in inventory['records']}
    require(inventory['source'] == manifest['final_code_source'] and len(lookup) == manifest['compiler_inventory']['count'], 'Postexecution input inventory')
    for row in manifest['source_records']:
        require(sha(data[row['record']]) == row['sha256'] == lookup[row['path']]['sha256'], 'Changed source identity')
    return {'passed': True, 'records': len(data), 'cooker_source003': 34,
            'renderer_source005_and006': [175, 189, 178], 'source007_compile_lint': 3,
            'finalized_gpu_controls': 8, 'gpu_original_failure_retained': True,
            'cube_palette_supporting_data': palette, 'root_editor_review': 'scoped acceptance',
            'scope': 'Offline replay only; no engine, GPU, app, build or performance execution'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--originals', action='store_true', help='Rehash retained original records')
    parser.add_argument('--binaries', action='store_true', help='Rehash retained external binaries/frontend')
    parser.add_argument('--git-source', nargs='?', const='executed', help='Read-only compiler-input comparison at a Git ref')
    parser.add_argument('--source-root', type=Path, default=HERE.parents[3])
    parser.add_argument('--extract-to', type=Path, help='Extract verified source/log/image records to a new directory')
    args = parser.parse_args()
    manifest = json.loads((HERE / 'manifest.json').read_text())
    require(manifest['format'] == 'incant-editor-scene-msaa-v1', 'Known packet format')
    archive = (HERE / 'records.json.gz').read_bytes()
    require(sha(archive) == manifest['archive_sha256'] and len(archive) == manifest['archive_bytes'], 'Archive identity')
    with gzip.open(HERE / 'records.json.gz', 'rb') as stream:
        decoded = stream.read(32 * 1024**2 + 1)
    require(len(decoded) <= 32 * 1024**2, 'Bounded decoded archive')
    data = {}
    for row in json.loads(decoded):
        name = row['name']
        require(not PurePosixPath(name).is_absolute() and '..' not in PurePosixPath(name).parts
                and name not in data and row['encoding'] in ('utf8', 'base64'), 'Safe unique record')
        data[name] = row['data'].encode() if row['encoding'] == 'utf8' else base64.b64decode(row['data'], validate=True)
    require(set(data) == {r['name'] for r in manifest['records']}, 'Complete record set')
    for row in manifest['records']:
        require(sha(data[row['name']]) == row['sha256'] and len(data[row['name']]) == row['bytes'], 'Raw record identity')
        if args.originals:
            require(sha(Path(row['original']).read_bytes()) == row['sha256'], 'Retained original:' + row['name'])
    if args.binaries:
        for row in manifest['retained_artifacts']:
            raw = Path(row['path']).read_bytes()
            require(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'Retained external artifact')
    if args.git_source:
        ref = manifest['final_code_source'] if args.git_source == 'executed' else args.git_source
        commit = subprocess.check_output(['git', '-C', str(args.source_root), 'rev-parse', '--verify',
                                          '--end-of-options', ref + '^{commit}'], text=True).strip()
        inventory = json.loads(data[manifest['compiler_inventory']['record']])
        for row in inventory['records']:
            raw = subprocess.check_output(['git', '-C', str(args.source_root), 'show', commit + ':' + row['path']])
            require(sha(raw) == row['sha256'], 'Git compiler source:' + row['path'])
    result = replay(manifest, data)
    if args.extract_to:
        require(not args.extract_to.exists(), 'Extraction destination must be fresh')
        args.extract_to.mkdir(parents=True)
        for name, raw in data.items():
            destination = args.extract_to / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(raw)
    print(json.dumps(result))


if __name__ == '__main__':
    main()
