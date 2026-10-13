#!/usr/bin/env python3
"""CPU-only public proof of optional result-role binding and fresh-session reset.

The fixture copies an existing approved label and its translated controls text.
It supplies no new UI design, wording, font or layout. All project/source changes
use atomic incant_cmd transactions; original presentation evidence stays intact.
"""
import argparse
import copy
import json
from pathlib import Path
import shutil

import bindings
import binding_oracle as oracle
import verify_bindings as proof
from art import ids

PROMPT = ids.widget(13)


def fixture(original, output, binary):
    output.mkdir()
    shutil.copytree(original / 'project', output / 'project')
    receipt = json.loads((original / 'receipt.json').read_text())
    project = output / 'project/core.json'
    author = proof.Author(binary, project, output)
    source = (proof.HERE / 'gameplay.ts').read_text()
    try:
        before = author.call('project.read')['project']
        prior = author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data']
        entities = before['scenes'][bindings.SCENE]['entities']
        node = copy.deepcopy(entities[ids.widget(8)]['components']['UiNode'])
        node['order'] = 13
        table = copy.deepcopy(before['string_tables'][bindings.HUD['table']])
        table['messages']['hud.prompt'] = copy.deepcopy(table['messages']['hud.controls'])
        controller = copy.deepcopy(entities[bindings.CONTROLLER]['components']['Script'])
        controller['props']['hud'] = bindings.roles(PROMPT)['hud']
        transaction = author.edit('Author optional result-label correctness fixture', [
            {'op': 'create_entity', 'scene_id': bindings.SCENE, 'entity': {
                'id': PROMPT, 'name': 'Result label correctness fixture', 'parent': bindings.HUD['inactive'],
                'provenance': None, 'components': {'UiNode': node}}},
            {'op': 'upsert_string_table', 'table': table},
            {'op': 'set_component', 'scene_id': bindings.SCENE, 'entity_id': bindings.CONTROLLER,
             'component': 'Script', 'value': controller},
            {'op': 'write_script', 'script_id': bindings.SCRIPT,
             'expected_source': prior['version'], 'source': source}])
        after = author.call('project.read')['project']
        assert bindings.authored_hud(after)['prompt'] == PROMPT
        author.call('history.undo')
        assert author.call('project.read')['project'] == before
        assert proof.same_source(author.call('script.read', {'script_id': bindings.SCRIPT})['untrusted_source_data'], prior)
        author.call('history.redo')
        assert author.call('project.read')['project'] == after
        author.save_reopen()
    finally:
        author.close()
    receipt['widgets']['Result label correctness fixture'] = PROMPT
    receipt['hud'] = bindings.roles(PROMPT)['hud']
    receipt['project_sha256'] = proof.digest(project)
    receipt['result_fixture'] = {'transaction': transaction, 'undo_redo_reopen': True,
        'copied_style_from': ids.widget(8), 'copied_localization_from': 'hud.controls',
        'appearance_approved': False}
    proof.write(output / 'receipt.json', receipt)
    return receipt


def refusals(prepared, output, binary):
    output.mkdir()
    shutil.copytree(prepared / 'authoring', output / 'project')
    project = output / 'project/core.json'
    author = proof.Author(binary, project, output)
    process = proof.Processes(output)
    try:
        before = author.call('project.read')['project']
        entities = before['scenes'][bindings.SCENE]['entities']
        baseline = entities[bindings.CONTROLLER]['components']['Script']
        cases = []
        for name, role, value, diagnostic in [
                ('unknown-role', 'unknown', PROMPT, 'unknown HUD binding'),
                ('duplicate-role', 'prompt', bindings.HUD['won'], 'HUD roles must be distinct'),
                ('null-prompt', 'prompt', None, 'invalid hud.prompt entity'),
                ('absent-widget', 'prompt', ids.widget(99), 'missing HUD widget prompt')]:
            controller = copy.deepcopy(baseline); controller['props']['hud'][role] = value
            cases.append((name, [{'op': 'set_component', 'scene_id': bindings.SCENE,
                'entity_id': bindings.CONTROLLER, 'component': 'Script', 'value': controller}], diagnostic))
        controller = copy.deepcopy(baseline); del controller['props']['hud']['objective']
        cases.append(('missing-required', [{'op': 'set_component', 'scene_id': bindings.SCENE,
            'entity_id': bindings.CONTROLLER, 'component': 'Script', 'value': controller}], 'invalid hud.objective entity'))
        node = copy.deepcopy(entities[PROMPT]['components']['UiNode']); node['kind'] = {'role': 'container'}
        cases.append(('wrong-kind', [{'op': 'set_component', 'scene_id': bindings.SCENE,
            'entity_id': PROMPT, 'component': 'UiNode', 'value': node}], 'wrong HUD widget kind prompt'))
        table = copy.deepcopy(before['string_tables'][bindings.HUD['table']]); del table['messages']['hud.prompt']
        cases.append(('missing-localization', [{'op': 'upsert_string_table', 'table': table}], 'missing HUD key hud.prompt'))
        for name, commands, diagnostic in cases:
            author.edit('Public result-label refusal ' + name, commands); author.save_reopen()
            pcm = output / (name + '.pcm')
            process.run([binary, 'native-play', project, '--camera', ids.entity(3), '--ui-viewport', '640x480',
                         '--ticks', '1', '--sample-rate', oracle.RATE, '--pcm-output', pcm],
                diagnostic='Error: native play startup/runtime failure: script tick failed: Error: Core Sample: ' + diagnostic + '\n')
            assert not pcm.exists()
            author.call('history.undo'); author.save_reopen()
            assert author.call('project.read')['project'] == before
    finally:
        author.close()
    proof.write(output / 'result.json', {'passed': True, 'cases': [c[0] for c in cases],
        'exact_diagnostics': True, 'no_pcm_on_failure': True, 'public_edit_undo_reopen': True})


def prepare(args):
    original = args.build.resolve(strict=True); protected = proof.inventory(original)
    binary = args.binary.resolve(strict=True); output = args.output.resolve(); output.mkdir(parents=True)
    for variant, build in [('existing', original), ('prompt', output / 'prompt-fixture')]:
        if variant == 'prompt': fixture(original, build, binary)
        proof.prepare(argparse.Namespace(build=build, output=output / variant, binary=binary))
    refusals(output / 'prompt', output / 'refusals', binary)
    assert proof.inventory(original) == protected
    proof.write(output / 'result.json', {'passed': True, 'variants': ['existing', 'prompt'],
        'binary': {'path': str(binary), 'sha256': proof.digest(binary)},
        'controller_sha256': proof.digest(proof.HERE / 'gameplay.ts'),
        'original': {'path': str(original), 'files': protected},
        'limits': ['CPU public authoring/native Play only; no appearance/GPU/phone gate',
                   'Existing approved label style and controls translations reused solely as correctness fixture',
                   'Actual packaged restart proof is separate ship stage; no old controller equality claim']})


def ship(args):
    output = args.output.resolve(); output.mkdir(parents=True)
    for variant in ('existing', 'prompt'):
        proof.ship(argparse.Namespace(development=args.development.resolve(strict=True) / variant,
                                      output=output / variant, existing_proof=None))
    proof.write(output / 'result.json', {'passed': True, 'variants': ['existing', 'prompt'],
        'controller_sha256': proof.digest(proof.HERE / 'gameplay.ts'),
        'source_free_public_exports': True, 'actual_r_and_start_reset': True,
        'limits': ['CPU ordinary shipping only; no appearance/window/GPU/device/performance gate']})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='mode', required=True)
    p = commands.add_parser('prepare'); p.add_argument('build', type=Path); p.add_argument('output', type=Path)
    p.add_argument('--binary', type=Path, required=True)
    p = commands.add_parser('ship'); p.add_argument('development', type=Path); p.add_argument('output', type=Path)
    args = parser.parse_args(); (prepare if args.mode == 'prepare' else ship)(args)


if __name__ == '__main__':
    main()
