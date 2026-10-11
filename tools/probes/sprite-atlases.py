#!/usr/bin/env python3
"""Import/repack stable atlas frames through commands and replay saved typed gameplay."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    digest = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(*arguments):
        return json.loads(subprocess.check_output([str(binary), *map(str, arguments)], cwd=ROOT,
                                                  text=True, encoding='utf-8'))

    def image(name, w, h, rgba):
        def chunk(kind, data):
            return struct.pack('>I', len(data))+kind+data+struct.pack('>I', zlib.crc32(kind+data))
        (out/name).write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB', w,h,8,6,0,0,0))+
            chunk(b'IDAT', zlib.compress((b'\0'+bytes(rgba)*w)*h))+chunk(b'IEND', b''))

    a, b = f'{1:026d}', f'{2:026d}'
    image('a.png', 1, 1, [255,0,0,255])
    image('b.png', 2, 2, [0,0,255,255])
    descriptor = {'version':1,'id':f'{9:026d}','maximum_size':[64,64],'padding':2,'frames':{
        a:{'name':'A','path':'a.png'}, b:{'name':'B','path':'b.png'}}}
    atlas = out/'sheet.atlas.json'
    atlas.write_text(json.dumps(descriptor)+'\n', encoding='utf-8')
    project, journal = out/'game.incant.json', out/'game.incant.journal.jsonl'
    run('init',project,'--name','Stable atlas replay','--entities',0)
    first = run('import',project,atlas.name)
    sid = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    eid = f'{10:026d}'
    sprite = {'texture':first['asset']['id'],'region':{'frame':b},'size':[2,2],
              'pivot':[.5,.5],'tint':[1,1,1,1]}

    def rpc(requests):
        text = subprocess.check_output([str(binary),'rpc',str(project),'--journal',str(journal)],
            input=''.join(json.dumps(r)+'\n' for r in requests), cwd=ROOT,text=True,encoding='utf-8')
        replies = [json.loads(line) for line in text.splitlines()]
        assert len(replies)==len(requests) and all('error' not in r for r in replies), replies
        return replies

    rpc([{'id':1,'method':'command.execute','params':{'commands':[{'op':'create_entity','scene_id':sid,
        'entity':{'id':eid,'name':'Atlas sprite','parent':None,'provenance':None,'components':{
            'Transform':{'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]},'SpriteRenderer':sprite}}}],
        'expected_revision':first['revision'],'description':'Author stable frame reference'}},{'id':2,'method':'project.save'}])
    before = project.read_bytes()
    image('a.png', 13, 17, [0,255,0,255])
    second = run('import',project,atlas.name)
    assert second['asset']['id']==first['asset']['id'] and second['asset']['sha256']!=first['asset']['sha256']
    assert second['details']['atlas']['frames'][b]['region']!=first['details']['atlas']['frames'][b]['region']
    assert json.loads(project.read_bytes())['scenes'][sid]['entities'][eid]['components']['SpriteRenderer']==sprite
    after = project.read_bytes()
    rpc([{'id':1,'method':'history.undo'},{'id':2,'method':'project.save'}])
    assert project.read_bytes()==before
    rpc([{'id':1,'method':'history.redo'},{'id':2,'method':'project.save'}])
    assert project.read_bytes()==after
    source, compiled = out/'atlas.ts', out/'atlas.js'
    source.write_text('import type {ScriptApi,SpriteRenderer} from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+';\n'+
        'const sid='+json.dumps(sid)+';const eid='+json.dumps(eid)+';const frames='+json.dumps([a,b])+';\n'+'''
export default defineBehavior<{tick:number}>({initialState:{tick:0},update(api:ScriptApi,_dt,state){
  state.tick++;
  const sprite=api.query('SpriteRenderer').find(e=>e.id===eid)!.components.SpriteRenderer as SpriteRenderer;
  const value:SpriteRenderer={...sprite,region:{frame:frames[Math.floor(state.tick/5)%2]},flip_x:state.tick>=30};
  api.command({op:'set_component',scene_id:sid,entity_id:eid,component:'SpriteRenderer',value});
}});
''', encoding='utf-8')
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit','--target','ES2022',
        '--module','ESNext','--moduleResolution','bundler',str(source)],cwd=ROOT,check=True)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],cwd=ROOT,check=True)
    for path in [atlas,out/'a.png',out/'b.png']:
        path.unlink()
    hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    whole=run('play',project,'--ticks',60,'--compiled-script',compiled,'--save-output',out/'final.save.json')
    first_part=run('play',project,'--ticks',25,'--compiled-script',compiled,'--save-output',out/'mid.save.json')
    resumed=run('play',project,'--ticks',35,'--compiled-script',compiled,'--load-save',out/'mid.save.json')
    repeat=run('play',project,'--ticks',60,'--compiled-script',compiled)
    assert first_part['script_state']=={'tick':25}
    assert whole['script_state']==resumed['script_state']==repeat['script_state']=={'tick':60}
    assert whole['state']==resumed['state']==repeat['state']
    saved=json.loads((out/'final.save.json').read_text(encoding='utf-8'))['project']['scenes'][sid]['entities'][eid]['components']['SpriteRenderer']
    assert saved['region']=={'frame':a} and saved['flip_x']
    assert hashes=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    assert digest==hashlib.sha256(binary.read_bytes()).hexdigest()
    result={'passed':True,'strict_typescript':True,'stable_asset_and_frame_ids':True,
        'target_frame_relocated':True,'exact_reimport_undo_redo':True,'sources_removed_before_runtime':True,
        'exact_saved_and_repeated_state':True,'binary_sha256':digest,'authored_hashes':hashes,'rendering_or_device_gate':False}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result))


if __name__=='__main__':
    main()
