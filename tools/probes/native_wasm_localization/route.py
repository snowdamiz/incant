"""One ordinary public export, four continuations and six source-free refusals."""
import json
from pathlib import Path
import shutil
import tempfile
import time

import export_native
from native_package import write_json
from ..native_wasm_checkpoint.route import inventory, stage
from ..native_wasm_restart.route import remove_owned
from ..native_wasm_restart.verify import digest, equal, require, validate_deleted
from . import author, fixture as f, oracle


def run(owner, root, plan, binary, guest, environment):
    output = owner.output
    with stage(owner, 240):
        project, authored = author.run(owner, root, binary, guest, environment)
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
    expected = {'kind': 'wasm', 'module': authored['module'], 'fingerprint': authored['fingerprint'],
        'input_sha256': authored['guest_sha256'], 'prepared_sha256': authored['prepared_sha256'],
        'registry_sha256': digest(root / 'sdk/abi/native-v1.json'), 'translator': 'wasm2c-1.0.42'}
    equal(exported['program'], expected, 'Ordinary authored wasm2c program linkage')
    require(exported['project_sha256'] == authored['project_sha256'] and exported['player_sha256'] == digest(player),
            'Actual project and copied player')
    require(exported['build_storage_removed_before_startup'] is True and exported['startup_source_parsing'] is False,
            'Normal source-free startup after owned build removal')
    remove_owned(output, [project.parent, trusted, *sorted((output / 'package/symbols').glob('*.map'))])
    require(not list(owner.scratch.iterdir()), 'Exporter scratch removed')
    files = inventory(player.parent)
    require(all(not name.endswith(('.wasm', '.rs', '.c', '.rlib', '.js', '.map')) for name in files),
            'No source/guest/compiler artifacts in runtime package')
    write_json(output / 'package-files.json', files)
    owner.deadline = min(owner.deadline, time.monotonic() + 300)
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TEMP': str(output), 'TMP': str(output)}
    outputs = {}

    def launch(name, ticks, replay, observe=None, load=None):
        pcm, saved = output / (name + '.pcm'), output / (name + '.save')
        argv = [player, '--package', player.parent / 'data', '--ticks', str(ticks),
                '--output', pcm, '--save-checkpoint', saved, '--input-recording', replay]
        for flag, value in [('--load-checkpoint', load), ('--observe', observe)]:
            if value is not None:
                argv.extend([flag, value])
        result = owner.run(argv, cwd=player.parent, env=env, timeout=30)
        receipt = {'argv': list(map(str, argv)), 'cwd': str(player.parent), 'environment': env,
            'returncode': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr,
            'outputs_present': [pcm.exists() or pcm.is_symlink(), saved.exists() or saved.is_symlink()],
            'staging': [p.name for p in output.glob('.incant-*')]}
        write_json(output / (name + '-process.json'), receipt)
        return result, pcm, saved, receipt

    for name, (start, end) in oracle.RUNS.items():
        replay, observe = output / (name + '-input.json'), output / (name + '-observe.json')
        write_json(replay, f.recording(end - start))
        write_json(observe, f.observation(start, end))
        result, pcm, saved, _ = launch(name, end - start, replay, observe,
                                       output / 'prefix.save' if start else None)
        require(result.returncode == 0, 'Positive player failed; original output retained')
        require(pcm.stat().st_size <= 256000 and saved.stat().st_size <= 1024**2, 'Fixed read bound')
        report = json.loads(result.stdout)
        write_json(output / (name + '.json'), report)
        outputs[name] = {'report': report, 'pcm': pcm.read_bytes(), 'saved': saved.read_bytes()}
        oracle.run(**outputs[name], name=name)
    oracle.joined(outputs, json.loads((output / 'reference.json').read_text()),
                  (output / 'reference.pcm').read_bytes())
    for key, name in f.FAILURES.items():
        replay = output / (name + '-input.json')
        write_json(replay, f.recording(1, key))
        _, _, _, receipt = launch(name, 1, replay, load=output / 'prefix.save')
        oracle.refusal(receipt, name)
    equal(inventory(player.parent), files, 'Unchanged runtime files/modes after controls')
    validate_deleted([project.parent, trusted, guest.parent])
    return {'passed': True, 'source_head': plan['source_head'], 'player_sha256': digest(player),
        'program': expected, 'positive_runs': list(oracle.RUNS), 'refusals': list(f.FAILURES.values()),
        'ts_reference': True, 'public_import_undo_redo_save_reopen': True, 'source_free_package': True,
        'exact_checkpoint_and_pcm_continuation': True,
        'limitations': ['CPU desktop only; no performance/hardware/full SDK parity claim',
            'report rows are missing translations; unknown keys are query results, not accumulated report rows',
            'failed native sessions are terminal; process refusals prove nonpublication, not whole-tick rollback',
            'callback rollback is separately covered by the PR401 ABI/Wasmtime/wasm2c controls',
            'explicit eight-byte guest state, not arbitrary guest memory',
            'source-free restricted PATH is not a filesystem access sandbox']}
