"""Native flipbook SDK controls on the existing source-free GPU game proof.

These numerical frame hashes use independently cooked static sprite controls;
this module does not review or approve appearance. Guest input and metadata are
bounded copies; source setup and compilation happen before player execution.
"""
import importlib.util
import json
from pathlib import Path
import shutil
import rust_guest

ROOT = Path(__file__).resolve().parents[1]
ACTIONS = [('flip_pause','F1'),('flip_seek','F2'),('flip_play','F3'),
           ('flip_restart','F4'),('flip_invalid','F5'),('flip_trap','F6')]
BEHAVIOR = '''
const flipbookUpdate = exports.default.update;
exports.default.update = function(ctx) {
    flipbookUpdate(ctx);
    const {flipbooks, input, commands} = ctx;
    if(flipbooks.ids.length !== flipbooks.frames.length || flipbooks.ids.length !== flipbooks.flags.length) throw Error('unaligned flipbook snapshot');
    for(let i=0;i<flipbooks.ids.length;i++) {
        const id=flipbooks.ids[i], frame=flipbooks.frames[i], flags=flipbooks.flags[i];
        if(frame>=256 || (flags & ~3)!==0) throw Error('flipbook payload');
        if(id!=='00000000000000000000000001') continue;
        if(input.actions.flip_pause?.pressed) commands.controlFlipbook(id,'pause');
        if(input.actions.flip_seek?.pressed) commands.controlFlipbook(id,'seek',2);
        if(input.actions.flip_play?.pressed) commands.controlFlipbook(id,'play');
        if(input.actions.flip_restart?.pressed) commands.controlFlipbook(id,'restart');
        if(input.actions.flip_invalid?.pressed) commands.controlFlipbook(id,'seek',3);
        if(flipbooks.frames[i]!==frame || flipbooks.flags[i]!==flags) throw Error('pending command changed committed snapshot');
    }
    if(input.actions.flip_trap?.pressed) throw Error('flipbook callback failure');
};
'''


def prepare(inputs, out, rustc, cargo, env, suffix):
    spec=importlib.util.spec_from_file_location('flipbook_static_shipping', ROOT/'tools/probes/shipping-wasm2c.py')
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    helper.cargo_build([*cargo,'build','-j2','-p','incant_wasm_cook','--locked'],cwd=ROOT,env=env,log=out/'wasm-cook-build.log')
    wabt,provenance=helper.provision(inputs)
    source=inputs/'flipbook.rs';shutil.copy2(ROOT/'crates/incant_game_player/fixtures/flipbook.rs',source)
    module=inputs/'flipbook.wasm'
    rust_guest.compile(rustc,source,module,env=env)
    bundle=inputs/'static-bundle'
    manifest=helper.wasm2c.prepare(bundle,[('flipbook-game',module)],wabt,ROOT/f'target/debug/incant_wasm_cook{suffix}')
    env.update(INCANT_WASM_BUNDLE=str(bundle),CC_ENABLE_DEBUG_OUTPUT='1')
    return {'rust_sdk':rust_guest.provenance(),'source_sha256':helper.sha(source),'module_sha256':helper.sha(module),'bundle':manifest,'translator':provenance}


def verify(out, command, fixtures, results, replay, run, reject, require):
    by_key={(r['fixture'],r['rate']):r['report'] for r in results}
    reports=[]
    # Closed-form end states for the fixed 13/3 FPS and 60Hz fixture. The
    # engine advances first, then commands apply. Pause keeps the remainder;
    # seek/restart reset it. Every case ends at an independent static oracle.
    scenarios=[('paused',[(1,'F1')],0,0,False),
               ('seek-paused',[(1,'F1'),(1,'F2')],2,2,False),
               ('restart',[(120,'F4')],0,0,True),
               ('resume',[(1,'F1'),(90,'F3')],0,2,True)]
    def modified(events):
        frames={1:[{'type':'key','code':'KeyD','down':True}]}
        for tick,key in events:frames.setdefault(tick,[]).append({'type':'key','code':key,'down':True})
        return dict(replay,frames=[{'tick':tick,'events':events}for tick,events in sorted(frames.items())])
    for streaming in ('static','stream'):
        prefix=streaming+'-sprite'
        fixture=next(f for f in fixtures if f['name']==prefix+'-animated')
        base=by_key[prefix,48000]; final=by_key[prefix+'-final-reference',48000]
        for name,events,first,last,playing in scenarios:
            recording=out/f'sdk-{streaming}-{name}.json';recording.write_text(json.dumps(modified(events)))
            pair=[]
            for vm in ('quickjs','wasm2c'):
                runs=[]
                for repeat in range(2):
                    dest=out/f'sdk-{streaming}-{name}-{vm}-{repeat}.pcm'
                    cmd=command(fixture['path'],recording,48000,dest)
                    if vm=='wasm2c':cmd=[cmd[0],'--static-module','flipbook-game',*cmd[1:]]
                    report=json.loads(run(cmd));images=report['render_input']['gpu']['captures']
                    require(images[0]['sha256']==(base if first==0 else final)['render_input']['gpu']['captures'][0]['sha256'], 'SDK first frame differs from static oracle')
                    require(images[1]['sha256']==(base if last==0 else final)['render_input']['gpu']['captures'][1]['sha256'], 'SDK final frame differs from static oracle')
                    require(report['pcm_sha256']==base['pcm_sha256'] and report['sources']==base['sources'], 'SDK changed simulation/audio')
                    require(report['tick']==120 and report['render_input']['gpu']['accepted_frames']==120, 'SDK lost accepted tick')
                    require(report['flipbooks']==[{'id':[0]*15+[1],'frame':last,'playing':playing,'completed':False}], 'Wrong committed SDK state')
                    require(report['render_input']['binding_generation']==0 and report['render_input']['topology_generation']==0, 'SDK controls rebuilt scene')
                    runs.append(report)
                require(runs[0]==runs[1], 'SDK fresh processes diverged')
                pair.append(runs[0]);reports.append({'streaming':streaming,'case':name,'vm':vm,'report':runs[0],'repeats':2})
            require(pair[0]['render_input']==pair[1]['render_input'] and pair[0]['flipbooks']==pair[1]['flipbooks'], 'QuickJS/static frame publication differs')
        for vm in ('quickjs','wasm2c'):
            for name,key in [('invalid','F5'),('trap','F6')]:
                recording=out/f'sdk-{streaming}-{name}-{vm}.json';recording.write_text(json.dumps(modified([(2,'F1'),(2,key)])))
                dest=out/f'sdk-{streaming}-{name}-{vm}.pcm';cmd=command(fixture['path'],recording,48000,dest)
                if vm=='wasm2c':cmd=[cmd[0],'--static-module','flipbook-game',*cmd[1:]]
                reject(f'sdk-{streaming}-{name}-{vm}',cmd,dest,'flipbook seek frame' if name=='invalid' else None)
    return {'bulk_copy':True,'readback':'committed pre-command snapshot','cases':reports,'processes':len(reports)*2,
            'normal_module_loader':False,'device_or_performance_gate':False}


def verify_build(cargo, env, output, symbols, kind):
    spec=importlib.util.spec_from_file_location('flipbook_static_audit', ROOT/'tools/probes/shipping-wasm2c.py')
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    metadata=helper.cargo_metadata([*cargo,'metadata','--locked','--format-version=1','--features','incant_game_player/render-gpu,incant_game_player/static-module'],cwd=ROOT,env=env)
    names={p['id']:p['name'] for p in metadata['packages']};nodes={n['id']:n for n in metadata['resolve']['nodes']}
    pending=[ident for ident,name in names.items() if name=='incant_game_player'];helper.require(len(pending)==1,'Missing unique game player graph')
    seen=set()
    forbidden=('wasmtime','cranelift','wasmparser','wasm-encoder','incant_wasm_cook')
    authoring={'incant_doc','incant_cmd','incant_agent','incant_core','incant_cook','incant_play_legacy','loro'}
    while pending:
        ident=pending.pop()
        if ident in seen:continue
        seen.add(ident)
        helper.require(not names[ident].startswith(forbidden) and names[ident] not in authoring,f'Forbidden normal player dependency: {names[ident]}')
        pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] is None for k in d['dep_kinds']))
    helper.verify_module_symbols(symbols)
    return {'c_flags':helper.verify_c_flags(output,kind,'2'),
            'normal_dependencies':sorted(names[i] for i in seen),'normal_compiler_jit_excluded':True,
            'quickjs_remains_linked':True,'flag_audit':'global presence in trusted build log; not adversarial environment validation'}
