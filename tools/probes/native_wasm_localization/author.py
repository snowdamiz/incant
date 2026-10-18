"""Ordinary import/source/command/save routes; no direct project-file mutation."""
import json
import shutil

from native_package import write_json
from ..native_wasm_restart.inputs import record
from ..native_wasm_restart.owner import Rpc
from ..native_wasm_restart.route import remove_owned
from ..native_wasm_restart.verify import digest, equal, require
from . import fixture as f, oracle


def commands(scene, module, audio):
    return [*[{'op': 'upsert_string_table', 'table': table} for table in f.tables()],
        {'op': 'set_locale', 'settings': f.settings(0)},
        {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': f.ENTITY, 'name': 'Coverage witness', 'parent': None, 'provenance': None,
            'components': {'Transform': {'translation': [0, 0, 0],
                'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}}}},
        {'op': 'create_entity', 'scene_id': scene, 'entity': {
            'id': f.AUDIO, 'name': 'Continuity witness', 'parent': None, 'provenance': None,
            'components': {'AudioSource': {'clip': audio, 'bus': None, 'gain_db': -4,
                'pan': .2, 'rate': 1.13, 'start_seconds': .00037, 'playing': True,
                'looping': True, 'streaming': False, 'spatial': None}}}},
        {'op': 'set_scene_program', 'scene_id': scene, 'program': {'kind': 'wasm', 'module': module}}]


def rejected_edit(rpc, before, scene):
    request = {'expected_revision': before['revision'], 'description': 'Invalid locale must roll back sibling edit',
        'commands': [{'op': 'rename_entity', 'scene_id': scene, 'entity_id': f.ENTITY, 'name': 'Must not commit'},
                     {'op': 'set_locale', 'settings': {'locale': 'fr-ca'}}]}
    try:
        rpc.call('command.execute', request)
    except ValueError:
        reply = rpc.history[-1]['response']
        require(reply.get('id') == rpc.requests and 'error' in reply and 'result' not in reply,
                'Actual typed command refusal, not transport failure')
    else:
        raise ValueError('Invalid authoring transaction accepted')
    equal(rpc.call('project.read'), before, 'Rejected authoring edit preserves project/revision/provenance')


def run(owner, root, binary, guest, environment):
    output, inputs = owner.output, owner.output / 'authoring'
    inputs.mkdir(mode=0o700)
    project, journal = inputs / 'project.json', inputs / 'project.journal.jsonl'
    raw, source = inputs / 'game.wasm', inputs / 'guest.rs'
    shutil.copy2(guest, raw)
    source.write_bytes((root / 'tools/probes/native_wasm_localization/guest.rs').read_bytes())
    f.tone(inputs / 'tone.wav')

    def execute(args):
        result = owner.run(args, cwd=root, env=environment, timeout=30)
        require(result.returncode == 0, 'Public command failed; retained logs identify it')
        return json.loads(result.stdout)

    execute([binary, 'init', project, '--entities', '0'])
    first = execute([binary, 'import', project, 'game.wasm'])
    repeated = execute([binary, 'import', project, 'game.wasm'])
    audio = execute([binary, 'import', project, 'tone.wav'])
    require(first['changed'] is True and first['asset']['kind'] == 'wasm', 'Public Wasm import')
    require(repeated['changed'] is False and repeated['cache_hit'] is True
            and repeated['revision'] == first['revision'], 'Idempotent reimport')
    require(audio['changed'] is True and audio['asset']['kind'] == 'audio', 'Public WAV import')
    write_json(output / 'imports.json', {'first': first, 'repeated': repeated, 'audio': audio})
    initial = json.loads(project.read_text())
    require(initial['settings']['tick_rate'] == 60 and len(initial['scenes']) == 1, 'Fixed authored clock')
    scene = next(iter(initial['scenes']))
    argv = list(map(str, [binary, 'rpc', project, '--journal', journal]))
    rpc = Rpc(owner, argv, root, environment)
    try:
        before = rpc.call('project.read')
        changed = rpc.call('command.execute', {'expected_revision': before['revision'],
            'description': 'Author localization coverage and audio continuity',
            'commands': commands(scene, first['asset']['id'], audio['asset']['id'])})
        require(changed['revision'] == before['revision'] + 1, 'One atomic authoring transaction')
        selected = rpc.call('project.read')
        rpc.call('history.undo')
        equal(rpc.call('project.read')['project'], before['project'], 'Undo all fixture authoring')
        rpc.call('history.redo')
        equal(rpc.call('project.read')['project'], selected['project'], 'Redo exact fixture')
        rejected_edit(rpc, rpc.call('project.read'), scene)
        rpc.call('project.save')
        saved = rpc.call('project.read')
        rpc.finish()
    finally:
        rpc.close()
    reference_source = (root / 'tools/probes/native_wasm_localization/reference.ts').read_text()
    rpc = Rpc(owner, argv, root, environment)
    try:
        equal(rpc.call('project.read'), saved, 'Fresh saved Wasm journal reader')
        rpc.call('command.execute', {'expected_revision': saved['revision'],
            'description': 'Select independently authored TypeScript reference reversibly', 'commands': [
                {'op': 'write_script', 'script_id': f.SCRIPT, 'expected_source': {'state': 'missing'}, 'source': reference_source},
                {'op': 'upsert_script', 'script': {'id': f.SCRIPT, 'name': 'Coverage reference', 'path': 'reference.ts'}},
                {'op': 'set_scene_program', 'scene_id': scene, 'program': None},
                {'op': 'set_component', 'scene_id': scene, 'entity_id': f.ENTITY, 'component': 'Script',
                 'value': {'source': f.SCRIPT, 'props': {}}}]})
        require(rpc.call('script.read', {'script_id': f.SCRIPT})['untrusted_source_data']['source'] == reference_source,
                'Public source publication')
        rpc.call('project.save')
        reference_read = rpc.call('project.read')
        rpc.finish()
    finally:
        rpc.close()
    observe = output / 'reference-observe.json'
    write_json(observe, f.observation(0, f.TICKS))
    reference = execute([binary, 'native-play', project, '--ticks', str(f.TICKS), '--observe', observe,
                         '--pcm-output', output / 'reference.pcm'])
    write_json(output / 'reference.json', reference)
    oracle.gameplay(reference, (output / 'reference.pcm').read_bytes(), 0, f.TICKS)
    rpc = Rpc(owner, argv, root, environment)
    try:
        equal(rpc.call('project.read'), reference_read, 'Runtime locale edits never author project changes')
        rpc.call('history.undo')
        equal(rpc.call('project.read')['project'], saved['project'], 'Reference selection undo restores Wasm')
        require(not (inputs / 'reference.ts').exists(), 'Undo removes only bus-owned reference source')
        rpc.call('project.save')
        final = rpc.call('project.read')
        rpc.finish()
    finally:
        rpc.close()
    rpc = Rpc(owner, argv, root, environment)
    try:
        equal(rpc.call('project.read'), final, 'Final saved Wasm selection reopens')
        rpc.finish()
    finally:
        rpc.close()
    cache = inputs / '.incant/cache/wasm' / (first['asset']['sha256'] + '.wasm.json')
    require(cache.stat().st_size <= 64 * 1024**2, 'Bounded imported metadata')
    captured = json.loads(cache.read_text())
    require(oracle.sha(bytes(captured['original'])) == digest(guest) == captured['input_sha256'], 'Public guest identity')
    require(oracle.sha(bytes(captured['prepared'])) == captured['prepared_sha256'], 'Metered guest identity')
    identity = {'project_sha256': digest(project), 'scene': scene, 'revision': final['revision'],
        'guest_sha256': digest(guest), 'module': first['asset']['id'], 'fingerprint': first['asset']['sha256'],
        'registry_sha256': captured['registry_sha256'], 'prepared_sha256': captured['prepared_sha256'],
        'policy': captured['policy'], 'reference_source': record(root / 'tools/probes/native_wasm_localization/reference.ts')}
    write_json(output / 'authored.json', identity)
    write_json(output / 'authored-read.json', final)
    write_json(output / 'expected-coverage.json', [f.rows(phase) for phase in range(4)])
    remove_owned(output, [source, raw, inputs / 'tone.wav', guest.parent])
    return project, identity
