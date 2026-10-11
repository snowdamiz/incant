#!/usr/bin/env python3
"""Numerical Play controls in an actual native editor window (not pixel review).

Build incant_editor with custom-protocol,play-probe and incant_headless first.
All fixture content is authored through public incant_cmd RPC transactions.
"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import native_process


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--editor', type=Path, required=True)
    parser.add_argument('--headless', type=Path, required=True)
    parser.add_argument('--author', action='store_true')
    args = parser.parse_args()
    output, editor, headless = args.output.resolve(), args.editor.resolve(), args.headless.resolve()
    project, journal = output / 'project.json', output / 'project.journal.jsonl'
    module = runpy.run_path(str(ROOT / 'tools/probes/script-source-transactions.py'))
    Rpc, require = module['Rpc'], module['require']
    entity, script, camera = module['ENTITY'], module['SCRIPT'], '00000000000000000000000003'
    if args.author:
        scene = next(iter(json.loads(project.read_text())['scenes']))
        rpc = Rpc(headless, project, journal)
        try:
            source = "import type { NativeBehavior } from '@incant/runtime/native';\nexport default { update({chunks}) { for (const chunk of chunks) for (let i=0;i<chunk.ids.length;i++) chunk.transforms[i*chunk.transformStride]+=2; } } satisfies NativeBehavior;\n"
            def item(identity, name, components):
                return {'op': 'create_entity', 'scene_id': scene, 'entity': {
                    'id': identity, 'name': name, 'parent': None, 'provenance': None, 'components': components}}
            transform = {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}
            result = rpc.call('command.execute', {'expected_revision': 0, 'description': 'Author native editor lifecycle fixture', 'commands': [
                {'op': 'write_script', 'script_id': script, 'source': source, 'expected_source': {'state': 'missing'}},
                {'op': 'upsert_script', 'script': {'id': script, 'name': 'Move', 'path': 'move.ts'}},
                item(entity, 'Mover', {'Transform': transform, 'Script': {'source': script, 'props': {}}}),
                item(camera, 'Camera', {'Transform': transform, 'Camera': {'projection': {'kind': 'perspective'}, 'fov_degrees': 60, 'near': 0.1, 'far': 100}}),
            ]})
            require(result['revision'] == 1, 'fixture creation was not atomic')
            require(rpc.call('project.save')['saved'], 'fixture save failed')
        finally:
            rpc.close()
        return
    output.mkdir(parents=True, exist_ok=False)
    def run(command, timeout=180):
        result = native_process.run(command, cwd=ROOT, timeout=timeout)
        require(result.returncode == 0, f'Owned probe failed ({result.returncode}): {result.stderr[-4000:]}')
        return result
    run([str(headless), 'init', str(project), '--entities', '0'])
    run([sys.executable, str(Path(__file__).resolve()), str(output), '--editor', str(editor), '--headless', str(headless), '--author'])
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    inputs = {name: digest(output / name) for name in ['project.json', 'project.journal.jsonl', 'move.ts']}
    receipt = output / 'native-window.json'
    result = native_process.run([str(editor), str(project), '--play-probe', str(receipt), entity], cwd=ROOT, timeout=120)
    (output / 'stderr.log').write_text(result.stderr)
    require(receipt.is_file(), 'native window did not write a receipt')
    proof = json.loads(receipt.read_text())
    require(result.returncode == 0 and proof.get('passed') is True, f'native window lifecycle failed: {proof}')
    require(inputs == {name: digest(output / name) for name in inputs}, 'Play changed authored bytes')
    evidence = {'passed': True, 'window': proof, 'authoring_sha256': inputs,
        'editor_sha256': digest(editor), 'headless_sha256': digest(headless),
        'authored_bytes_unchanged': True, 'fixture_authored_through_incant_cmd': True}
    (output / 'result.json').write_text(json.dumps(evidence, indent=2) + '\n')
    print(json.dumps({'passed': True, 'evidence': str(output / 'result.json')}))


if __name__ == '__main__':
    main()
