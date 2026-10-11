#!/usr/bin/env python3
"""Exercise the actual developer cook/build/native-play path after source deletion.

This uses the development headless coordinator, not a shipping or device proof.
The copied binary retains authoring tools; its simulated world is disposable and
uses the native graph. No GPU frame capture, editor Play or full SDK is claimed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools'))
from native_play import prepare

SOURCE=b'''
// INCANT_SOURCE_SENTINEL_native_project_a29e
for (const compile of [() => eval('1'), () => (0,eval)('1'),
    () => Function('return 1'), () => (function(){}).constructor('return 1')]) {
    let denied=false; try { compile(); } catch (_) { denied=true; }
    if (!denied) throw Error('source compiler active');
}
exports.default={update({chunks,input,clock,log}) {
    for(const chunk of chunks) if(chunk.velocities)
        for(let i=0;i<chunk.ids.length;i++) chunk.velocities[i*3]=input.actions.right.active?3:0;
    log('accepted '+clock.tick+' \\u{1f680}');
    if(clock.tick===3 && input.actions.fail.active) throw Error('later failure');
}};
'''

def require(condition,message):
    if not condition: raise ValueError(message)

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    args=parser.parse_args(); out=args.output.resolve(); out.mkdir(parents=True,exist_ok=False)
    with prepare(SOURCE) as (player,digest):
        require(not (player.parent/'behavior.js').exists(),'compile input still present')
        player_hash=sha(player)
        def run(command,**kwargs):
            return subprocess.run([player,*command],cwd=out,capture_output=True,text=True,encoding='utf-8',timeout=60,**kwargs)
        project=out/'game.incant.json'
        run(['init',project,'--name','Native project play','--entities','0'],check=True)
        document=json.loads(project.read_text()); scene=next(iter(document['scenes'])); entity='00000000000000000000000001'; rate=document['settings']['tick_rate']
        commands=[{'op':'create_entity','scene_id':scene,'entity':{'id':entity,'name':'Mover','parent':None,'provenance':None,'components':{'Transform':{'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]},'Velocity':{'linear':[2,0,0]}}}},
            {'op':'set_input_actions','actions':{'right':{'kind':'button','bindings':[{'type':'key','code':'KeyD'}]},'fail':{'kind':'button','bindings':[{'type':'key','code':'Space'}]}}}]
        journal=out/'game.journal.jsonl'
        requests=[{'id':1,'method':'command.execute','params':{'expected_revision':0,'description':'Native fixture','commands':commands}},{'id':2,'method':'project.save'}]
        result=run(['rpc',project,'--journal',journal],input=''.join(json.dumps(r)+'\n' for r in requests),check=True)
        require(all('error' not in json.loads(line) for line in result.stdout.splitlines()),result.stdout)
        before={p.name:sha(p) for p in (project,journal)}
        recording={'format':'incant-input','version':1,'tick_rate':rate,'start_tick':0,'ticks':5,'frames':[{'tick':1,'events':[{'type':'key','code':'KeyD','down':True}]}]}
        replay=out/'input.json'; replay.write_text(json.dumps(recording))
        observations={'format':'incant-native-observation','version':1,'entities':[entity],'checks':[{'name':'motion','tick':5,'expect':{'type':'vector','entity':entity,'property':'translation','value':[14/rate,0,0],'tolerance':1e-10}}]}
        plan=out/'observe.json'; plan.write_text(json.dumps(observations))
        base=['native-play',project,'--ticks','5','--script-sha256',digest,'--input-replay',replay,'--observe',plan]
        reports=[]
        for repeat in range(3):
            result=run([*base,'--pcm-output',out/f'{repeat}.pcm'],check=True)
            report=json.loads(result.stdout); reports.append(report)
            require(report['passed'] and not report['script_source_parsing'],'wrong execution mode')
            require(report['script_source_sha256']==digest and report['tick']==5,'wrong script/tick')
            require(len(report['logs']['entries'])==5 and report['logs']['entries'][-1]['message']=='accepted 5 🚀','logs/Unicode')
            require(report['observations']['entities'][entity]['velocity']==[3,0,0],'behavior not applied')
        require(reports[0]==reports[1]==reports[2],'fresh process drift')
        require(sha(out/'0.pcm')==sha(out/'1.pcm')==sha(out/'2.pcm'),'PCM drift')
        observations['checks'][0]['expect']['value']=[99,0,0]; plan.write_text(json.dumps(observations))
        false=run([*base,'--pcm-output',out/'false.pcm'])
        require(false.returncode!=0 and json.loads(false.stdout)['passed'] is False and (out/'false.pcm').is_file(),'false assertions must return completed state')
        recording['frames'][0]['events'].append({'type':'key','code':'Space','down':True}); replay.write_text(json.dumps(recording))
        failed=run([*base,'--pcm-output',out/'failed.pcm'])
        require(failed.returncode!=0 and not failed.stdout and not (out/'failed.pcm').exists(),'terminal failure leaked output')
        require('later failure' in failed.stderr,'expected guest failure')
        require(not any(p.name.startswith('.tmp') for p in out.iterdir()),'partial PCM remained')
        bad=run(['native-play',project,'--script-sha256','0'*64])
        require(bad.returncode!=0 and not bad.stdout,'wrong compiled behavior accepted')
        require(before=={p.name:sha(p) for p in (project,journal)},'authoring changed')
        require(player_hash==sha(player),'player modified itself')
        result={'passed':True,'successful_runs':3,'false_assertion_runs':1,'terminal_failure_runs':1,'mismatched_script_runs':1,'player_sha256':player_hash,'source_sha256':digest,'temporary_compile_input_deleted':True,'authoring_unchanged':True,'baseline':reports[0],'shipping_profile':False,'gpu_submitted':False,'reference_phone_gate':False}
        (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({'passed':True,'runtime_processes':6}))

if __name__=='__main__': main()
