#!/usr/bin/env python3
"""Author cameras through shared commands and restore runtime projection edits. No GPU."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/incant_headless')
    parser.add_argument('--pixel-perfect', action='store_true', help='Verify logical-resolution cameras and unsnapped simulation movement')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    kind = 'pixel_perfect' if args.pixel_perfect else 'orthographic'
    def projection(size):
        return ({'kind':kind,'pixels_per_unit':size*8,'reference_resolution':[320,180]}
                if args.pixel_perfect else {'kind':kind,'vertical_size':size})

    def run(*arguments):
        return json.loads(subprocess.check_output([str(binary), *map(str, arguments)], cwd=ROOT,
                                                 text=True, encoding='utf-8'))

    project, journal = out/'camera.incant.json', out/'camera.incant.journal.jsonl'
    run('init', project, '--name', 'Projection continuity', '--entities', 0)
    sid = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    cid = f'{10:026d}'
    legacy = {'fov_degrees': 60, 'near': .1, 'far': 100}

    def rpc(requests):
        text = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
            input=''.join(json.dumps(r)+'\n' for r in requests), cwd=ROOT, text=True, encoding='utf-8')
        replies = [json.loads(line) for line in text.splitlines()]
        assert all('error' not in r for r in replies), replies
        return replies

    revision = rpc([{'id': 0, 'method': 'project.read'}])[0]['result']['revision']
    rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': [
        {'op': 'create_entity', 'scene_id': sid, 'entity': {'id': cid, 'name': 'Legacy camera',
         'parent': None, 'provenance': None, 'components': {'Camera': legacy,
         'Transform': {'translation': [0, 0, 8], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]}}}}],
         'expected_revision': revision, 'description': 'Author camera for saved projection test'}},
         {'id': 2, 'method': 'project.save'}])
    source, compiled = out/'camera.ts', out/'camera.js'
    source.write_text('import type { ScriptApi, Camera, Transform } from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+';\n'+
        'const sid='+json.dumps(sid)+'; const cid='+json.dumps(cid)+';\n'+'''
export default defineBehavior<{tick:number,size:number}>({initialState:{tick:0,size:0},
  update(api:ScriptApi,_dt,s){
    s.tick++;
    const camera=api.query('Camera').find(e=>e.id===cid)!.components.Camera as Camera;
    if(s.tick===1 || s.tick===40){
      s.size=s.tick===1?4:8;
      const value:Camera={...camera,projection:PROJECTION_VALUE};
      api.command({op:'set_component',scene_id:sid,entity_id:cid,component:'Camera',value});
    }
    if(s.tick>1 && camera.projection?.kind!==PROJECTION_KIND)throw Error('projection was lost');
    UNSNAPPED_MOTION
  }
});
'''.replace('PROJECTION_VALUE', "{kind:'pixel_perfect',pixels_per_unit:s.size*8,reference_resolution:[320,180]}" if args.pixel_perfect else "{kind:'orthographic',vertical_size:s.size}")
        .replace('PROJECTION_KIND',json.dumps(kind))
        .replace('UNSNAPPED_MOTION', '''const pose=api.query('Camera').find(e=>e.id===cid)!.components.Transform as Transform;
    if(s.tick>1 && pose.translation[0] !== (s.tick-1)/256)throw Error('Camera snapping leaked into simulation');
    api.command({op:'set_component',scene_id:sid,entity_id:cid,component:'Transform',value:{...pose,translation:[s.tick/256,0,8]}});''' if args.pixel_perfect else ''), encoding='utf-8')
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit','--target','ES2022',
        '--module','ESNext','--moduleResolution','bundler',str(source)],cwd=ROOT,check=True)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],cwd=ROOT,check=True)
    authored = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    whole=run('play',project,'--ticks',60,'--compiled-script',compiled,'--save-output',out/'final.save.json')
    first=run('play',project,'--ticks',25,'--compiled-script',compiled,'--save-output',out/'mid.save.json')
    resumed=run('play',project,'--ticks',35,'--compiled-script',compiled,'--load-save',out/'mid.save.json')
    repeat=run('play',project,'--ticks',60,'--compiled-script',compiled)
    assert first['script_state']=={'tick':25,'size':4}
    assert whole['script_state']==resumed['script_state']==repeat['script_state']=={'tick':60,'size':8}
    assert whole['state']==resumed['state']==repeat['state']
    for file,size in [('mid.save.json',4),('final.save.json',8)]:
        saved=json.loads((out/file).read_text(encoding='utf-8'))
        assert saved['project']['scenes'][sid]['entities'][cid]['components']['Camera']['projection']==projection(size)
    assert authored=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    assert digest==hashlib.sha256(binary.read_bytes()).hexdigest()
    result={'passed':True,'strict_typescript':True,'legacy_camera_authored_via_shared_commands':True,
        'projection_kind':kind,'subpixel_simulation_movement_checked':args.pixel_perfect,
        'runtime_projection_edits_save_resume_and_repeat_exact':True,'author_files_unchanged':authored,
        'binary_sha256':digest,'rendering_or_device_gate':False}
    (out/'runs.json').write_text(json.dumps({'whole':whole,'first':first,'resumed':resumed,'repeat':repeat},indent=2)+'\n',encoding='utf-8')
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
