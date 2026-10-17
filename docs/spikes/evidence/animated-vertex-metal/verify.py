"""Bounded data-only replay. Never imports or executes archived source/binaries."""
import argparse
import base64
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
from pixels import validate as pixels

HEAD='d053542e69c7ce33ea64226bbd41d591caa8fe10'
PACKET_HASH='14a26cb32a7b923eb74d2db4b8c7cda48772e18d8fca6ab9ce4726b32dd370c9'
BINARY={'bytes':34585304,'sha256':'d9f979ad8a8d92ca5e4d9eee0760a707c2b78fcbe57055b94c42be178ee34f6c','executable':True}


def need(value,message):
    if not value:raise ValueError(message)


def sha(data):return hashlib.sha256(data).hexdigest()


def load(directory):
    directory=Path(directory)
    manifest=json.loads((directory/'manifest.json').read_text())
    archive=directory/'records.json.gz'
    need(archive.stat().st_size<=4*1024**2,'Compressed record bound')
    raw=archive.read_bytes()
    need(len(raw)==manifest['archive']['bytes'] and sha(raw)==manifest['archive']['sha256'],'Archive binding')
    with gzip.open(archive,'rb') as stream:data=stream.read(16*1024**2+1)
    need(len(data)<=16*1024**2 and len(data)==manifest['archive']['decoded_bytes'],'Decoded record bound')
    bundle=json.loads(data)
    need(bundle['format']=='incant-animated-vertex-metal-records-v1','Record format')
    records={}
    need(set(bundle['records'])==set(manifest['records']) and len(bundle['records'])==manifest['record_count'],'Exact archive inventory')
    for name,row in bundle['records'].items():
        need(not Path(name).is_absolute() and '..' not in Path(name).parts,'Relative inert record name')
        need(row['encoding'] in ('utf8','base64'),'Explicit data encoding')
        value=row['data'].encode('utf-8') if row['encoding']=='utf8' else base64.b64decode(row['data'],validate=True)
        need(len(value)==row['bytes'] and sha(value)==row['sha256'],'Raw record hash: '+name)
        need(manifest['records'][name]=={'bytes':len(value),'sha256':sha(value)},'Manifest record hash: '+name)
        records[name]=value
    return records,manifest


def validate(records,manifest):
    js=lambda n:json.loads(records[n])
    need(manifest['producer_head']==HEAD and manifest['producer_packet_sha256']==PACKET_HASH,'Fixed actual producer')
    need(manifest['external_executable']==BINARY,'Fixed external artifact identity')
    need(sha(records['packet.json'])==PACKET_HASH,'Original frozen packet')
    packet=js('packet.json');plan=js('plan.json');source=js('source.json')
    need(plan['source_head']==source['head']==HEAD,'Source/plan identity')
    for name,identity in packet['files'].items():
        need(len(records[name])==identity['bytes'] and sha(records[name])==identity['sha256'],'Frozen input: '+name)
    need(sha(Path(__file__).with_name('pixels.py').read_bytes())==packet['files']['pixels.py']['sha256'],'Reviewed pixel verifier identity')
    cpu,gpu=js('cpu-result.json'),js('gpu-result.json')
    needed=set(packet['files'])|{'packet.json','cpu-result.json','gpu-result.json','cpu-terminal-closeout.json','gpu-terminal-closeout.json','root-review.json'}
    for phase,result in [('cpu',cpu),('gpu',gpu)]:
        needed.update(phase+'-output/'+name for name in result['retained'])
    need(set(records)==needed,'Complete exact retained output inventory')
    artifact=js('cpu-output/artifact.json')
    need(artifact['identity']==cpu['artifact']==gpu['artifact']==BINARY,'Artifact binding throughout both executions')
    need(artifact['source_head']==HEAD and artifact['cargo']['fresh'] is False,'Actual fresh test producer')
    cargo=artifact['cargo']
    need(cargo['manifest_path']==plan['source_root']+'/crates/incant_render_gpu_host/Cargo.toml','Actual Cargo source path')
    need(cargo['features']==['default','physics-deterministic'] and cargo['profile']['test'] is True and cargo['profile']['opt_level']=='1','Scoped CI/default feature route')
    built=[json.loads(line) for line in records['cpu-output/000.stdout.log'].decode().splitlines()]
    need(cargo in built,'Artifact appears in actual build output')
    review=js('root-review.json')
    need(review['source']==HEAD and review['root_raw_pixel_replays_passed']==['geometry','tangents','clip'],'Independent review scope')
    for name in ('cpu-result.json','gpu-result.json','packet.json'):
        need(review['records'][name]==sha(records[name]),'Independent review binding: '+name)
    need(review['records']['game-tests']==BINARY['sha256'],'Independent artifact review binding')
    for phase,result in [('cpu',cpu),('gpu',gpu)]:
        owner=js(phase+'-output/owner.json');close=js(phase+'-terminal-closeout.json')
        need(result['passed'] is True and owner['passed'] is True and close['passed'] is True,'Successful terminal chain')
        need(result['source_head']==close['source_head']==HEAD and result['packet_sha256']==close['packet_sha256']==PACKET_HASH,'Producer provenance')
        need(close['exec_terminal_exit']==0 and close['artifact']==BINARY,'Actual wrapper terminal and artifact')
        need(close['result_sha256']==sha(records[phase+'-result.json']),'Terminal result hash')
        need(result['owner_sha256']==close['owner_sha256']==sha(records[phase+'-output/owner.json']),'Owner hash')
        need(result['source_tools_predecessors_protected_unchanged'] is True,'Recorded final source/tool preservation')
        for name,identity in result['retained'].items():
            data=records[phase+'-output/'+name]
            need(len(data)==identity['bytes'] and sha(data)==identity['sha256'],'Retained raw binding: '+name)
        calls=owner['calls'];groups=owner['owned']
        need(len(calls)==len(groups)==3,'Exactly three actual stage owners')
        ids=[g['pid'] for g in groups]
        need(len(set(ids))==3 and all(type(p) is int and p>0 for p in ids),'Distinct positive owner IDs')
        need(result['groups']==close['groups']==ids,'Exact stage groups')
        for call,group in zip(calls,groups):
            need(call['finished'] is True and call['exit']==0 and call['cleanup_confirmed'] is True and group['cleanup_confirmed'] is True and group['exit']==0,'Stage terminal and cleanup')
            need(call['pid']==group['pid'] and call['argv']==group['argv'],'Stage ownership/command binding')
            for stream in ('stdout','stderr'):
                need(call[stream]['sha256']==sha(records[phase+'-output/'+call[stream]['path']]),'Exact captured stream')
        observed=result['observed_pids']
        need(observed and len(set(observed))==len(observed) and all(type(p) is int and p>0 for p in observed),'Nonempty distinct observed IDs')
        need(set(ids)<=set(observed),'Observed IDs cover every stage owner')
        need(owner['wrapper_pid']==result['wrapper_pid']==close['wrapper_pid'],'Wrapper binding')
        need(type(owner['wrapper_pid']) is int and owner['wrapper_pid']>0 and owner['wrapper_pid'] not in observed,'Distinct positive wrapper identity')
        need([r['pid'] for r in result['absence']]==sorted(observed) and all(r['pid_and_group_absent'] is True for r in result['absence']),'Exact internal absence inventory')
        expected=set(observed)|{owner['wrapper_pid']}
        need(len(close['absence'])==len(expected) and {r['pid'] for r in close['absence']}==expected and all(r['pid_and_group_absent'] is True for r in close['absence']),'Exact final observed-plus-wrapper absence inventory')
        need(result['target_peak']<=6665195520 and owner['peaks']['rss']<=8*1024**3,'Recorded original resource caps')
        if phase=='cpu':
            need(calls[0]['argv']==plan['build_argv'] and calls[2]['argv']==plan['lint_argv'],'Actual build/strict-lint commands')
            need('test result: ok. 3 passed; 0 failed; 3 ignored;' in records['cpu-output/001.stdout.log'].decode(),'Three executed CPU fixtures')
        else:
            for index,stage in enumerate(plan['gpu_stages']):
                need(calls[index]['argv']==[artifact['copy'],stage['test'],'--exact','--ignored','--nocapture','--test-threads=1'],'Exact native GPU test command')
                need('test result: ok. 1 passed; 0 failed; 0 ignored;' in records[f'gpu-output/{index:03}.stdout.log'].decode(),'Actual GPU test success')
                need('backend: Metal' in records[f'gpu-output/{index:03}.stderr.log'].decode(),'Actual Metal adapter log')
    need(abs(cpu['cumulative_seconds']-(plan['prior_seconds']+cpu['seconds']))<1e-8,'Preserved previous CPU charge')
    need(abs(gpu['cumulative_seconds']-(cpu['cumulative_seconds']+gpu['seconds']))<1e-8 and gpu['cumulative_seconds']<2400,'Preserved complete GPU charge')
    outcomes=[]
    with tempfile.TemporaryDirectory(prefix='incant-metal-record-replay-') as temporary:
        for kind in ('geometry','tangents','clip'):
            target=Path(temporary)/kind;target.mkdir()
            # Only bounded raw pixels/proofs are materialized; archived code is inert.
            prefix='gpu-output/'+kind+'/'
            for name,data in records.items():
                if name.startswith(prefix) and (name.endswith('.rgba') or name==prefix+'proof.json'):
                    leaf=name.removeprefix(prefix)
                    need(Path(leaf).name==leaf and len(data)<=160*96*4,'Bounded pixel/proof leaf')
                    (target/leaf).write_bytes(data)
            outcomes.append(pixels(target,kind))
    return {'passed':True,'records':len(records),'producer':HEAD,'pixel_replays':outcomes,'native_reexecuted':False,'artifact_contents_checked':False,'window_or_performance_claim':False}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact',type=Path,help='Optional surviving copied game-tests file; read/hash only')
    args=parser.parse_args()
    records,manifest=load(Path(__file__).resolve().parent)
    result=validate(records,manifest)
    if args.artifact is not None:
        need(args.artifact.is_file() and not args.artifact.is_symlink() and args.artifact.stat().st_size==BINARY['bytes'],'Exact regular external artifact size')
        need(sha(args.artifact.read_bytes())==BINARY['sha256'],'Exact external artifact bytes')
        result['artifact_contents_checked']=True
    print(json.dumps(result,indent=2))


if __name__=='__main__':main()
