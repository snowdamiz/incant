"""Public authoring and unchanged fresh exporter; explicit source-free replays."""
import json
import contextlib
from pathlib import Path
import shutil
import tempfile
import time

import export_native
from native_package import write_json
from ..native_wasm_restart.inputs import record
from ..native_wasm_restart.owner import Rpc
from ..native_wasm_restart.route import remove_owned
from ..native_wasm_restart.verify import digest, equal, require, validate_deleted
from . import fixture, oracle


@contextlib.contextmanager
def stage(owner, seconds):
    original = owner.deadline
    owner.deadline = min(original, time.monotonic() + seconds)
    try:
        yield
        owner.check(force=True)
    finally:
        owner.deadline = original


def inventory(root):
    paths = sorted(root.rglob('*'))
    require(all(not path.is_symlink() for path in paths), 'No package links')
    return {str(path.relative_to(root)): record(path) | {'mode': path.stat().st_mode & 0o777}
            for path in paths if path.is_file()}


def author(owner, root, binary, guest, environment):
    output = owner.output
    inputs = output / 'authoring'
    inputs.mkdir(mode=0o700)
    project, journal = inputs / 'project.json', inputs / 'project.journal.jsonl'
    raw = inputs / 'game.wasm'
    shutil.copy2(guest, raw)
    source = inputs / 'guest.rs'
    source.write_bytes((root / 'tools/probes/native_wasm_checkpoint/guest.rs').read_bytes())
    fixture.tone(inputs / 'tone.wav')

    def execute(args):
        result = owner.run(args, cwd=root, env=environment, timeout=30)
        require(result.returncode == 0, 'Public command failed; see retained logs')
        return json.loads(result.stdout)

    execute([binary, 'init', project, '--entities', '0'])
    first = execute([binary, 'import', project, 'game.wasm'])
    repeated = execute([binary, 'import', project, 'game.wasm'])
    audio = execute([binary, 'import', project, 'tone.wav'])
    require(first['changed'] is True and first['asset']['kind'] == 'wasm', 'Public Wasm import')
    require(repeated['cache_hit'] is True and repeated['changed'] is False
            and repeated['revision'] == first['revision'], 'Idempotent public reimport')
    require(audio['changed'] is True and audio['asset']['kind'] == 'audio', 'Public WAV import')
    write_json(output / 'imports.json', {'first': first, 'repeated': repeated, 'audio': audio})
    document = json.loads(project.read_text())
    require(document['settings']['tick_rate'] == 60 and len(document['scenes']) == 1, 'Default public clock')
    scene = next(iter(document['scenes']))
    argv = list(map(str, [binary, 'rpc', project, '--journal', journal]))
    rpc = Rpc(owner, argv, root, environment)
    try:
        before = rpc.call('project.read')
        commands = [
            {'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': fixture.ENTITY, 'name': 'Persistent mover', 'parent': None, 'provenance': None,
                'components': {'Transform': {'translation': [0, 0, 0],
                    'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}}}},
            {'op': 'create_entity', 'scene_id': scene, 'entity': {
                'id': fixture.AUDIO, 'name': 'Saved loop', 'parent': None, 'provenance': None,
                'components': {'AudioSource': {'clip': audio['asset']['id'], 'bus': None,
                    'gain_db': -4, 'pan': .2, 'rate': 1.13, 'start_seconds': .00037,
                    'playing': True, 'looping': True, 'streaming': False, 'spatial': None}}}},
            {'op': 'set_scene_program', 'scene_id': scene,
             'program': {'kind': 'wasm', 'module': first['asset']['id']}}]
        changed = rpc.call('command.execute', {'expected_revision': before['revision'],
            'description': 'Author persistent gameplay and audio checkpoint proof', 'commands': commands})
        require(changed['revision'] == before['revision'] + 1, 'Atomic public edit')
        selected = rpc.call('project.read')
        rpc.call('history.undo')
        equal(rpc.call('project.read')['project'], before['project'], 'Undo all authored changes')
        rpc.call('history.redo')
        equal(rpc.call('project.read')['project'], selected['project'], 'Redo exact scene')
        rpc.call('project.save')
        saved = rpc.call('project.read')
        rpc.finish()
    finally:
        rpc.close()
    rpc = Rpc(owner, argv, root, environment)
    try:
        equal(rpc.call('project.read'), saved, 'Public saved journal reopens exactly')
        rpc.finish()
    finally:
        rpc.close()
    cache = inputs / '.incant/cache/wasm' / (first['asset']['sha256'] + '.wasm.json')
    require(cache.stat().st_size <= 64 * 1024**2, 'Bounded capture metadata')
    captured = json.loads(cache.read_text())
    require(oracle.sha(bytes(captured['original'])) == digest(guest) == captured['input_sha256'],
            'Exact guest entered public import')
    require(oracle.sha(bytes(captured['prepared'])) == captured['prepared_sha256'], 'Exact metered capture')
    identity = {'project_sha256': digest(project), 'scene': scene, 'revision': saved['revision'],
                'guest_sha256': digest(guest), 'module': first['asset']['id'],
                'fingerprint': first['asset']['sha256'], 'registry_sha256': captured['registry_sha256'],
                'prepared_sha256': captured['prepared_sha256'], 'policy': captured['policy']}
    write_json(output / 'authored.json', identity)
    # Preserve the actual authored read result, without replacing it with a direct
    # document mutation. This is data-only evidence, not a runtime input.
    write_json(output / 'authored-read.json', saved)
    remove_owned(output, [source, raw, inputs / 'tone.wav', guest.parent])
    return project, identity


def run(owner, root, plan, binary, guest, environment):
    output = owner.output
    with stage(owner, 120):
        project, authored = author(owner, root, binary, guest, environment)
    trusted = output / 'trusted-wabt'
    (trusted / 'bin').mkdir(parents=True, mode=0o700)
    for name in ('wasm2c', 'wat2wasm'):
        shutil.copy2(Path(plan['wabt']) / 'bin' / name, trusted / 'bin' / name)
    previous = tempfile.tempdir
    tempfile.tempdir = str(owner.scratch)
    try:
        with stage(owner, 3300):
            exported = export_native.export(project, output / 'package', mode='headless', wabt=trusted)
    finally:
        tempfile.tempdir = previous
    player = output / 'package/player/incant_game_player'
    expected_program = {'kind': 'wasm', 'module': authored['module'], 'fingerprint': authored['fingerprint'],
        'input_sha256': authored['guest_sha256'], 'prepared_sha256': authored['prepared_sha256'],
        'registry_sha256': digest(root / 'sdk/abi/native-v1.json'), 'translator': 'wasm2c-1.0.42'}
    equal(exported['program'], expected_program, 'Public captured guest reaches generated shipping linkage')
    require(exported['project_sha256'] == authored['project_sha256'] and exported['player_sha256'] == digest(player),
            'Exact authored project and copied shipping player')
    require(exported['build_storage_removed_before_startup'] is True and exported['startup_source_parsing'] is False,
            'Ordinary exporter source-free startup')
    remove_owned(output, [project.parent, trusted, *sorted((output / 'package/symbols').glob('*.map'))])
    require(not list(owner.scratch.iterdir()), 'Ordinary exporter retained private build')
    package_files = inventory(player.parent)
    require(all(not name.endswith(('.wasm', '.rs', '.c', '.rlib', '.js', '.map')) for name in package_files),
            'No source/module/compiler artifacts in runtime package')
    write_json(output / 'package-files.json', package_files)
    owner.deadline = min(owner.deadline, time.monotonic() + 300)
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TEMP': str(output), 'TMP': str(output)}
    outputs = {}

    def launch(name, ticks, load=None, save=None, replay=None, observe=None):
        pcm = output / (name + '.pcm')
        argv = [player, '--package', player.parent / 'data', '--ticks', str(ticks), '--output', pcm]
        for flag, value in [('--load-checkpoint', load), ('--save-checkpoint', save),
                            ('--input-recording', replay), ('--observe', observe)]:
            if value is not None:
                argv.extend([flag, value])
        result = owner.run(argv, cwd=player.parent, env=env, timeout=30)
        receipt = {'argv': list(map(str, argv)), 'cwd': str(player.parent), 'environment': env,
                   'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr}
        write_json(output / (name + '-process.json'), receipt)
        return result, pcm, receipt

    for name, (start, end) in oracle.RUNS.items():
        replay, observe = output / (name + '-input.json'), output / (name + '-observe.json')
        write_json(replay, fixture.recording(start, end - start))
        write_json(observe, fixture.observation(start, end))
        saved = output / (name + '.save')
        result, pcm, _ = launch(name, end - start, output / 'prefix.save' if start else None,
                               saved, replay, observe)
        require(result.returncode == 0, 'Positive checkpoint player failed; original logs retained')
        require(pcm.stat().st_size <= 256000 and saved.stat().st_size <= 1024**2, 'Fixed output read bound')
        report = json.loads(result.stdout)
        write_json(output / (name + '.json'), report)
        outputs[name] = {'report': report, 'pcm': pcm.read_bytes(), 'saved': saved.read_bytes()}
        oracle.run(**outputs[name], name=name)
    oracle.joined(outputs)
    for name in oracle.REFUSALS:
        corrupted, saved = output / (name + '.save'), output / (name + '-new.save')
        corrupted.write_bytes(oracle.mutate_checkpoint(outputs['prefix']['saved'], name))
        result, pcm, receipt = launch(name, 1, corrupted, saved)
        receipt.update(outputs_present=[pcm.exists() or pcm.is_symlink(), saved.exists() or saved.is_symlink()],
                       staging=[p.name for p in output.glob('.incant-*')])
        write_json(output / (name + '-refusal.json'), receipt)
        oracle.refusal(receipt, name)
    for name in ('pcm-exists', 'save-exists'):
        pcm, saved = output / (name + '.pcm'), output / (name + '.save')
        sentinel = pcm if name == 'pcm-exists' else saved
        sentinel.write_bytes(b'Preserve existing authored checkpoint output.\n')
        sentinel.chmod(0o600)
        before = record(sentinel) | {'mode': sentinel.stat().st_mode & 0o777}
        result, pcm, receipt = launch(name, 1, output / 'prefix.save', saved)
        receipt.update(sentinel_before=before, sentinel_after=record(sentinel) | {'mode': sentinel.stat().st_mode & 0o777},
                       outputs_present=[pcm.exists() or pcm.is_symlink(), saved.exists() or saved.is_symlink()],
                       staging=[p.name for p in output.glob('.incant-*')])
        write_json(output / (name + '-refusal.json'), receipt)
        oracle.refusal(receipt, name)
    equal(inventory(player.parent), package_files, 'Runtime files/modes unchanged after all controls')
    validate_deleted([project.parent, trusted, guest.parent])
    return {'passed': True, 'source_head': plan['source_head'], 'player_sha256': digest(player),
            'program': expected_program, 'positive_runs': list(oracle.RUNS),
            'refusals': [*oracle.REFUSALS, 'pcm-exists', 'save-exists'],
            'exact_checkpoint_and_pcm_continuation': True,
            'public_import_undo_redo_save_reopen': True, 'source_free_package': True,
            'limitations': ['desktop CPU only; no performance/device/phase claim',
                'explicit host bytes; no arbitrary guest memory snapshot',
                'plain scene without physics/UI; integral48kHz/60Hz output clock',
                'PCM and checkpoint publication are separate no-clobber transactions',
                'source-free restricted environment is not a filesystem access sandbox']}
