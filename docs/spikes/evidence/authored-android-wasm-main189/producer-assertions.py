"""Read-only producer re-audit of retained Android static-Wasm shipping evidence."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

OUT = Path.home()/'.codex/incant-evidence/authored-android-wasm-shipping-001'
FREEZE = json.loads((OUT / 'freeze.json').read_text())
ORIGINAL = json.loads((OUT / 'freeze.json').read_text())
ROOT = Path('/Users/andreyyurlov/.codex/worktrees/agent-budget-settings/incant')
sys.path.insert(0, str(ROOT / 'tools'))
import android_archive
import android_wasm
import runpy
audio = runpy.run_path(str(ROOT / "tools/probes/native-wasm-audio-export.py"))
from physics_features import selected_graph_errors, validate_artifacts


def sha(path):
    value = hashlib.sha256()
    with Path(path).open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def read(path):
    return json.loads(Path(path).read_text())


def files(path):
    return {str(p.relative_to(path)): sha(p) for p in sorted(path.rglob('*')) if p.is_file()}


def raw_commands(rows, directory=None):
    for row in rows:
        for stream in ('stdout', 'stderr'):
            path = Path(row[stream])
            if not path.is_absolute():
                path = directory / path
            assert sha(path) == row[stream + '_sha256'], path
    return 2 * len(rows)


records = []
for mode in ('deterministic', 'fast'):
    prepared = OUT.parent / 'authored-android-wasm-preflight-001' / mode
    declared = read(prepared / 'result.json')
    assert declared['passed'] and declared['physics_mode'] == mode and declared['public_import_select_mode_undo_redo_reopen']
    assert not (prepared / 'authoring').exists()
    calls = read(prepared / 'public-commands.json')
    assert [row['method'] for row in calls] == ['project.read', 'command.execute', 'project.read', 'history.undo', 'project.read', 'history.redo', 'project.read', 'project.save', 'project.read', 'project.read']
    assert calls[0]['result']['project'] == calls[4]['result']['project']
    assert calls[2]['result']['project'] == calls[6]['result']['project']
    assert calls[8]['result'] == calls[9]['result']
    assert calls[1]['params']['commands'][-2] == {'op': 'set_physics_mode', 'mode': mode}
    assert calls[2]['result']['project']['scenes'][declared['scene']]['program'] == {'kind': 'wasm', 'module': declared['program_asset']['id']}
    native_runs = [read(prepared / f'development-{i}.json') for i in range(2)]
    assert native_runs[0] == native_runs[1]
    oracle = read(prepared / 'oracle.json')
    assert len(oracle) == 96 and all(len(row['values']) == 186 for row in oracle)
    for i, native in enumerate(native_runs):
        assert native['passed'] and not native['script_source_parsing'] and native['cpu_render_input'] and not native['gpu_submitted']
        audio['check_logs'](native['logs'], oracle, lambda ok, reason: None if ok else (_ for _ in ()).throw(AssertionError(reason)))
        assert (prepared / f'development-{i}.pcm').read_bytes() == (prepared / 'oracle.pcm').read_bytes()
    preflight_commands = read(prepared / 'commands.json')
    assert len(preflight_commands) == 14 and [r['exit_code'] for r in preflight_commands] == [0]*8 + [1]*6
    for row in preflight_commands:
        for stream in ('stdout', 'stderr'):
            assert sha(prepared / row[stream]['path']) == row[stream]['sha256']
    for row, (name, _, _) in zip(preflight_commands[-6:], audio['ACTIONS']):
        assert not (prepared / row['stdout']['path']).read_bytes()
        assert (prepared / row['stderr']['path']).read_text().strip() == audio['DEV_ERRORS'][name]
        assert not (prepared / ('refused-' + name + '.pcm')).exists()
    for path, expected in declared['source_deleted'].items():
        assert not (prepared / path).exists()
    for path, expected in declared['binaries'].items():
        assert sha(path) == expected
    exported=OUT/(mode+'-archive');caller=OUT/('deterministic-callers' if mode=='deterministic' else 'fast-callers-002')
    receipt=read(exported/'export.json');result=read(caller/'result.json');target='aarch64-linux-android'
    assert receipt['target']==target and receipt['physics_mode']==result['physics_mode']==mode
    assert result['passed'] and receipt['private_build_storage_removed'] and receipt['physics_variant']==mode
    assert sha(exported/'export.json')==result['export_sha256']
    producer=read(exported/'project-build.json')
    android_archive.producer_receipt(producer,declared['authoring_unchanged']['game.json'],mode,receipt['program'])
    assert producer['script']==receipt['script']
    graph=(exported/'normal-dependencies.txt').read_text();assert android_archive.normal_dependencies(graph)==receipt['normal_dependencies']
    assert selected_graph_errors(graph,mode,target)==[]
    command=read(exported/'build-command.json');assert command==receipt['command'] and command['exit_code']==0
    assert sha(exported/'build.log')==command['stderr_sha256'] and sha(exported/'cargo.jsonl')==command['stdout_sha256']
    target_dir=Path(command['argv'][command['argv'].index('--target-dir')+1]);assert not target_dir.parent.exists()
    log=(exported/'build.log').read_text();assert android_archive.verify_codegen(log,True)==receipt['codegen']
    messages=[json.loads(line) for line in (exported/'cargo.jsonl').read_text().splitlines() if line.startswith('{')]
    assert validate_artifacts(messages,mode,target_dir=target_dir,target=target,profile='shipping')==receipt['physics']
    assert receipt['program']['fingerprint']==declared['program_asset']['sha256']
    assert sha(exported/'runtime/data/world.scene')==declared['oracle_scene_sha256']
    compiled=receipt['static_module']['compiled'];facade=next(Path(path) for path in compiled['sources'] if path.endswith('/m0-facade.c'))
    prepared_c={'bundle':str(facade.parent),'compiler':compiled['compiler']}
    assert android_wasm.audit_build((exported/'cargo.jsonl').read_text(),log,target,prepared_c,ROOT)==compiled
    assert not facade.parent.exists()
    assert read(exported/'wasm-translation.json')==receipt['static_module']['translation']
    assert files(exported/'runtime')==result['runtime_before']
    assert files(exported/'runtime/data')==receipt['data']['files']
    assert sha(exported/receipt['archive']['path'])==receipt['archive']['sha256']
    assert sha(exported/'symbols/libincant_game_player.unstripped.a')==receipt['symbols']['unstripped_sha256']
    assert not (exported/'symbols/behavior.js.map').exists()
    audits=read(exported/'audit/commands.json');assert all(row['exit_code']==0 for row in audits)
    streams=raw_commands(audits,exported/'audit')+raw_commands(result['commands'])+raw_commands([result['create'],result['delete']])
    item=result['caller'];assert sha(item['path'])==item['sha256']
    links=[r for r in result['commands'] if r['argv'][0].endswith('aarch64-linux-android24-clang')]
    assert len(links)==1 and links[0]['exit_code']==0 and '-Wl,--gc-sections' in links[0]['argv']
    sections=[r for r in result['commands'] if Path(r['argv'][0]).name=='llvm-objdump' and r['argv'][-1]==item['path']]
    symbols=[r for r in result['commands'] if Path(r['argv'][0]).name=='llvm-nm' and r['argv'][-1]==item['path']]
    assert len(sections)==len(symbols)==1
    assert '.debug_' not in Path(sections[0]['stdout']).read_text()
    names=Path(symbols[0]['stdout']).read_text()
    assert all(n in names for n in ('incant_game_run_v1','incant_m0_update','wasm2c_m0_instantiate'))
    assert not any(n in names for n in android_archive.PARSERS)
    assert result['api']=='37' and result['abi']=='arm64-v8a' and not result['physical_device']
    assert result['execution']=='local ARM Android emulator CPU'
    assert result['emulator_cleanup']['exit_code']==0 and result['emulator_cleanup']['graceful'] and not result['cleanup_errors']
    assert result['delete']['exit_code']==0 and result['create']['exit_code']==0
    assert not list((caller/'avd').iterdir())
    argv=result['emulator']['argv'];assert argv[argv.index('-gpu')+1]=='software'
    assert argv[argv.index('-cores')+1]=='2' and argv[argv.index('-memory')+1]=='2048'
    assert '-no-window' in argv and '-no-audio' in argv and result['serial']==('emulator-5584' if mode=='deterministic' else 'emulator-5586')
    assert [r['expected_status'] for r in result['controls']]==[0,0,0,1,3]+[1]*6+[2]*4+[1]
    real_calls=[r for r in result['commands'] if result['remote_directory']+'/caller-c' in r['argv'] and r['argv'][3:6]==['shell','env','-i']]
    assert len(real_calls)==16
    opposite='fast' if mode=='deterministic' else 'deterministic';foreign=OUT/(opposite+'-archive')
    assert sha(foreign/'export.json')==result['foreign_export_sha256']
    positive=refused=0
    for control,call in zip(result['controls'],real_calls):
        status=control['expected_status'];assert call['exit_code']==status and read(call['stdout'])=={'abi_status':status}
        assert call['argv'][3:6]==['shell','env','-i']
        wire=caller/Path(call['argv'][-1]).name;assert wire.is_file()
        local_request=read(control['request']);request=read(wire)
        for key,value in local_request.items():
            if key in ('package','input_recording','observations','output_directory') and Path(value).is_absolute():
                assert request[key]==result['remote_directory']+'/'+str(Path(value).relative_to(caller))
            else:assert request[key]==value
        assert sha(control['request'])==control['request_sha256']
        destination=Path(local_request['output_directory'])
        if status==0:
            positive+=1;report=read(destination/'result.json');assert report==control['report']
            assert report['passed'] and report['error'] is None and report['os']=='android' and report['architecture']=='aarch64'
            assert report['panic']=='abort' and not report['debug_assertions'] and report['execution']=='offline_cpu'
            game=report['game'];assert game['logs']==native_runs[0]['logs']
            audio['check_logs'](game['logs'],oracle,lambda ok,reason: None if ok else (_ for _ in ()).throw(AssertionError(reason)))
            assert len(game['logs']['entries'])==576 and game['logs']['dropped']==0
            for key in ('tick','frames','observations','pcm_sha256'):assert game[key]==native_runs[0][key]
            assert game['observations']['passed'] and len(game['observations']['checks'])==248
            pcm=(destination/'audio.pcm').read_bytes();assert pcm==(prepared/'oracle.pcm').read_bytes() and len(pcm)==614400 and any(pcm)
        else:
            refused+=1
            if status==2:assert not destination.exists()
            elif status==1:
                report=read(destination/'result.json');assert report==control['report'] and not report['passed'] and report['game'] is None and not (destination/'audio.pcm').exists()
                suffix=control['name'].split('-',1)[1]
                error={'wrong-mode':f'cooked physics kernel {opposite} does not match compiled kernel {mode}',**{n:'static Wasm trapped (code 1)' for n,_,_ in audio['ACTIONS'][:3]},'late-owner':'runtime entity StableId([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 90]) does not exist','bad-role':'invalid scene data: audio control requires final source','seek-clip':'invalid audio setting: pan, rate or start time outside supported range','checksum-failure':'native package startup checksum mismatch'}[suffix]
                assert report['error']==error
                if suffix=='wrong-mode':assert files(Path(local_request['package']))==files(foreign/'runtime/data')
            elif status==3:
                assert control['name']=='c-no-clobber' and files(destination)==files(caller/'c-fresh-2')
    assert positive==3 and refused==13
    records.append({'mode':mode,'target':target,'actual_api':result['api'],'compiler_api':receipt['ndk']['api'],'positive_processes':positive,'exact_refusals':refused,'raw_command_streams_rehashed':streams,'archive_bytes':receipt['archive']['bytes'],'caller_bytes':item['bytes'],'runtime_files':len(result['runtime_before']),'export_receipt_sha256':sha(exported/'export.json'),'caller_receipt_sha256':sha(caller/'result.json')})
fallback=[]
for path,expected in FREEZE['files'].items():
    if (ROOT/path).is_file() and sha(ROOT/path)==expected:continue
    original=subprocess.check_output(['git','show','026426d1dca4ff8e41aacbf02741ec68fc4aa386:'+path],cwd=ROOT)
    assert hashlib.sha256(original).hexdigest()==expected,path
    fallback.append(path)
assert not (OUT/'compiled-retained').exists() and not (OUT/'trusted-tools').exists() and not any((OUT/'private-builds').iterdir())
for file in ['commands.json','caller-commands.json','caller-continuation-commands-002.json']:
    rows=read(OUT/file)
    expected=[0,1] if file=='caller-commands.json' else ([0] if file=='caller-continuation-commands-002.json' else [0,0])
    assert [r['exit_code'] for r in rows]==expected
    for row in rows:
        for stream in ('stdout','stderr'):assert sha(row[stream]['path'])==row[stream]['sha256']
terminal=read(OUT/'terminal.json');assert terminal['passed']
assert '[Errno 48] Address already in use' in Path(terminal['retained_failed_attempt']['stderr']['path']).read_text()
assert not (OUT/'fast-callers/emulator.log').exists()
assert all(sha(OUT/(name+'/result.json'))==terminal['reports'][mode] for mode,name in [('deterministic','deterministic-callers'),('fast','fast-callers-002')])
value={'passed':True,'records':records,'frozen_inputs_verified':len(FREEZE['files']),'immutable_git_fallback':fallback,'all_fields_per_tick':186,'ticks':96,'development_processes':4,'fresh_android_processes':6,'android_refusals':26,'sources_builds_maps_removed':True,'scope':'API37 local ARM emulator CPU only; no physical-device, minimum-OS, GPU or performance gate.'}
(OUT/'producer-reaudit.json').write_text(json.dumps(value,indent=2)+'\n');print(json.dumps(value))
