"""Public authoring, unchanged ordinary export, then source-free restart observations."""
import hashlib
import json
from pathlib import Path
import shutil
import tempfile

import export_native
from native_package import write_json
from .inputs import record
from .owner import Rpc
from .verify import (CHECKPOINT_REFUSAL, DEV_REFUSAL, ENTITY, GUEST_SHA256, digest, equal,
                     observation, require, validate_deleted, validate_program, validate_refusal,
                     validate_run)


def remove_owned(root, paths):
    """Hash and delete only known proof-owned inputs, with no link traversal."""
    removed = {}
    for path in paths:
        require(path.is_relative_to(root) and path != root and not path.is_symlink(), 'Owned removal')
        entries = sorted(path.rglob('*')) if path.is_dir() else [path]
        require(all(not p.is_symlink() for p in entries), 'Unexpected link in owned inputs')
        for entry in entries:
            if entry.is_file():
                removed[str(entry.relative_to(root))] = record(entry)
    receipt = root / f'deleted-{len(list(root.glob("deleted-*.json")))}.json'
    write_json(receipt, removed)  # Preserve the complete inventory before the first removal.
    for path in paths:
        shutil.rmtree(path) if path.is_dir() else path.unlink()
    validate_deleted(paths)
    return removed


def run(owner, root, plan, binary, environment):
    output = owner.output

    def execute(argv, **kwargs):
        result = owner.run(argv, cwd=root, env=environment, timeout=30, **kwargs)
        require(result.returncode == 0, f'Public command failed; retained child logs: {argv[1:2]}')
        return result

    def call_json(argv):
        return json.loads(execute(argv).stdout)

    inputs = output / 'authoring'
    inputs.mkdir(mode=0o700)
    project, journal = inputs / 'project.json', inputs / 'project.journal.jsonl'
    guest = inputs / 'guest.wasm'
    shutil.copy2(plan['guest'], guest)
    source = inputs / 'session_restart.rs'
    source.write_bytes((root / 'sdk/rust/samples/session_restart.rs').read_bytes())
    execute([binary, 'init', project, '--entities', '0'])
    imported = call_json([binary, 'import', project, 'guest.wasm'])
    require(imported['asset']['kind'] == 'wasm' and imported['changed'] is True, 'First public import')
    repeated = call_json([binary, 'import', project, 'guest.wasm'])
    require(repeated['cache_hit'] is True and repeated['changed'] is False
            and repeated['revision'] == imported['revision'], 'Idempotent public reimport')
    write_json(output / 'imports.json', {'first': imported, 'repeated': repeated})
    document = json.loads(project.read_text())
    require(document['settings']['tick_rate'] == 60 and len(document['scenes']) == 1, 'Default authored clock')
    scene = next(iter(document['scenes']))

    def rpc_open():
        return Rpc(owner, list(map(str, [binary, 'rpc', project, '--journal', journal])), root, environment)

    rpc = rpc_open()
    try:
        before = rpc.call('project.read')

        def change(commands, description):
            revision = rpc.call('project.read')['revision']
            return rpc.call('command.execute', {'expected_revision': revision,
                            'description': description, 'commands': commands})

        change([{'op': 'create_entity', 'scene_id': scene, 'entity': {
                    'id': ENTITY, 'name': 'Restart witness', 'parent': None, 'provenance': None,
                    'components': {'Transform': {'translation': [0, 0, 0],
                        'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}}}},
                {'op': 'set_scene_program', 'scene_id': scene,
                 'program': {'kind': 'wasm', 'module': imported['asset']['id']}}],
               'Author SDK restart-once scene')
        selected = rpc.call('project.read')['project']
        rpc.call('history.undo')
        equal(rpc.call('project.read')['project'], before['project'], 'Atomic selection Undo')
        rpc.call('history.redo')
        equal(rpc.call('project.read')['project'], selected, 'Selection Redo')
        change([{'op': 'set_scene_program', 'scene_id': scene, 'program': None}], 'Clear module')
        require('program' not in rpc.call('project.read')['project']['scenes'][scene], 'Clear selection')
        rpc.call('history.undo')
        equal(rpc.call('project.read')['project'], selected, 'Clear Undo')
        rpc.call('project.save')
        saved = rpc.call('project.read')
        rpc.finish()
    finally:
        rpc.close()
    rpc = rpc_open()
    try:
        equal(rpc.call('project.read'), saved, 'Saved public journal reopen')
        rpc.finish()
    finally:
        rpc.close()
    write_json(output / 'authored-identity.json', {'project_sha256': digest(project),
        'revision': saved['revision'], 'scene': scene, 'entity': ENTITY,
        'selected_module': imported['asset']['id']})
    cache = inputs / '.incant/cache/wasm' / (imported['asset']['sha256'] + '.wasm.json')
    require(cache.stat().st_size <= 64 * 1024**2, 'Bounded public capture')
    captured = json.loads(cache.read_text())
    require(hashlib.sha256(bytes(captured['original'])).hexdigest() == GUEST_SHA256
            and hashlib.sha256(bytes(captured['prepared'])).hexdigest() == captured['prepared_sha256'],
            'Public original/metered bytes')
    write_json(output / 'captured-identity.json', {k: captured[k] for k in
        ('policy', 'registry_sha256', 'input_sha256', 'prepared_sha256')})
    removed = remove_owned(output, [source, guest])
    dev_pcm = output / 'refused-development.pcm'
    # Native-play deliberately has no restart owner. This is an expected refusal,
    # not a development/package parity claim or a private fixture bypass.
    dev = owner.run([binary, 'native-play', project, '--ticks', '1', '--pcm-output', dev_pcm],
                    cwd=root, env=environment, timeout=30)
    validate_refusal(dev, DEV_REFUSAL, [dev_pcm])
    trusted = output / 'trusted-wabt'
    (trusted / 'bin').mkdir(parents=True, mode=0o700)
    for name in ('wasm2c', 'wat2wasm'):
        shutil.copy2(Path(plan['wabt']) / 'bin' / name, trusted / 'bin' / name)
    # Both existing export TemporaryDirectories live under the observed scratch.
    # No exporter feature, command, codegen, graph or symbol audit is replaced.
    previous_temp = tempfile.tempdir
    tempfile.tempdir = str(owner.scratch)
    try:
        report = export_native.export(project, output / 'package', mode='headless', wabt=trusted)
    finally:
        tempfile.tempdir = previous_temp
    player = output / 'package/player/incant_game_player'
    validate_program(report, imported, captured, digest(root / 'sdk/abi/native-v1.json'), player)
    removed.update(remove_owned(output, [inputs, trusted, *sorted((output / 'package/symbols').glob('*.map'))]))
    require(not list(owner.scratch.iterdir()), 'Exporter private build remains')
    replay, observe = output / 'input.json', output / 'observe.json'
    write_json(replay, {'format': 'incant-input', 'version': 1, 'tick_rate': 60,
                       'start_tick': 0, 'ticks': 3, 'frames': []})
    write_json(observe, observation())
    package_files = {str(p.relative_to(player.parent)): record(p)
                     for p in sorted(player.parent.rglob('*')) if p.is_file()}
    write_json(output / 'package-files.json', package_files)
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TEMP': str(output), 'TMP': str(output)}
    for index in range(3):
        pcm = output / f'run-{index}.pcm'
        result = owner.run([player, '--package', player.parent / 'data', '--ticks', '3',
                           '--input-recording', replay, '--observe', observe, '--output', pcm],
                          cwd=player.parent, env=env, timeout=30)
        require(result.returncode == 0, 'Source-free ordinary package run failed')
        value = json.loads(result.stdout)
        write_json(output / f'run-{index}.json', value)
        require(pcm.stat().st_size <= 19200, 'Fixed PCM read bound')
        validate_run(value, pcm.read_bytes())
    refused_pcm, refused_save = output / 'unsupported.pcm', output / 'unsupported.checkpoint'
    refusal = owner.run([player, '--package', player.parent / 'data', '--ticks', '1',
                         '--output', refused_pcm, '--save-checkpoint', refused_save],
                        cwd=player.parent, env=env, timeout=30)
    validate_refusal(refusal, CHECKPOINT_REFUSAL, [refused_pcm, refused_save])
    equal({str(p.relative_to(player.parent)): record(p)
           for p in sorted(player.parent.rglob('*')) if p.is_file()}, package_files, 'Runtime package changed')
    validate_deleted([inputs, trusted])
    return {'passed': True, 'source_head': plan['source_head'], 'program': report['program'],
            'player_sha256': digest(player), 'guest_recompiled': False,
            'public_import_select_undo_redo_clear_save_reopen': True,
            'native_play': 'expected no-restart-owner refusal',
            'private_build_removed_before_exporter_startup': True,
            'authoring_source_cache_tool_copies_removed_before_replays': True,
            'ordinary_shipping_package': True, 'fresh_processes': 3, 'restarts_each': 3,
            'three_runs_exact_observations_and_pcm': True, 'static_checkpoint_refused': True,
            'deleted_private_files': removed, 'no_fixture_selection_flag': True,
            'limitations': ['desktop CPU only; unsigned', 'no native window, GPU or device audio',
                            'no phone, performance, full SDK or phase gate',
                            'static guest checkpoint unsupported; cold restart is separately observed']}
