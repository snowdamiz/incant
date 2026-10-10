#!/usr/bin/env python3
"""Verify imported source-free sprites and saved typed animation through public commands."""
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

    def chunk(kind, data):
        return struct.pack('>I', len(data))+kind+data+struct.pack('>I', zlib.crc32(kind+data))

    source_image = out/'sheet.png'
    # Numerical strip, not sample art: four one-pixel frames.
    source_image.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB',4,1,8,6,0,0,0))+
        chunk(b'IDAT', zlib.compress(b'\0'+bytes([255,0,0,255,0,255,0,255,0,0,255,255,255,255,255,128])))+chunk(b'IEND', b''))
    project, journal = out/'game.incant.json', out/'game.incant.journal.jsonl'
    run('init',project,'--name','Saved sprite animation','--entities',0)
    imported = run('import',project,'sheet.png')
    sid = next(iter(json.loads(project.read_text(encoding='utf-8'))['scenes']))
    eid = f'{10:026d}'
    sprite = {'texture':imported['asset']['id'],'region':[0,0,1,1],'size':[2,1],
              'pivot':[.5,.5],'tint':[1,1,1,1]}

    def rpc(requests, success=True):
        text = subprocess.check_output([str(binary),'rpc',str(project),'--journal',str(journal)],
            input=''.join(json.dumps(r)+'\n' for r in requests), cwd=ROOT,text=True,encoding='utf-8')
        replies = [json.loads(line) for line in text.splitlines()]
        assert len(replies)==len(requests)
        if success:
            assert all('error' not in r for r in replies), replies
        return replies

    rpc([{'id':1,'method':'command.execute','params':{'commands':[{'op':'create_entity','scene_id':sid,
        'entity':{'id':eid,'name':'Animated sprite','parent':None,'provenance':None,'components':{
            'Transform':{'translation':[0,0,0],'rotation':[0,0,0,1],'scale':[1,1,1]},'SpriteRenderer':sprite}}}],
        'expected_revision':imported['revision'],'description':'Author source-free sprite'}},{'id':2,'method':'project.save'}])
    authored = project.read_bytes()
    rpc([{'id':1,'method':'history.undo'},{'id':2,'method':'history.redo'},{'id':3,'method':'project.save'}])
    assert project.read_bytes()==authored
    revision = rpc([{'id':1,'method':'project.read'}])[0]['result']['revision']
    previous_journal = journal.read_bytes()
    invalid = dict(sprite, region=[0,0,0,1])
    replies = rpc([{'id':1,'method':'command.execute','params':{'commands':[
        {'op':'rename_entity','scene_id':sid,'entity_id':eid,'name':'Partial'},
        {'op':'set_component','scene_id':sid,'entity_id':eid,'component':'SpriteRenderer','value':invalid}],
        'expected_revision':revision,'description':'Invalid sprite rollback'}}],False)
    assert 'error' in replies[0] and project.read_bytes()==authored and journal.read_bytes()==previous_journal
    source, compiled = out/'sprite.ts',out/'sprite.js'
    source.write_text('import type {ScriptApi,SpriteRenderer,Transform} from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+';\n'+
        'const sid='+json.dumps(sid)+';const eid='+json.dumps(eid)+';\n'+'''
export default defineBehavior<{tick:number}>({initialState:{tick:0},update(api:ScriptApi,_dt,state){
  state.tick++;
  const entity=api.query('SpriteRenderer').find(e=>e.id===eid)!;
  const sprite=entity.components.SpriteRenderer as SpriteRenderer;
  const pose=entity.components.Transform as Transform;
  const value:SpriteRenderer={...sprite,region:[Math.floor(state.tick/5)%4,0,1,1],
    flip_x:state.tick>=20,flip_y:state.tick>=40,layer:2,order:state.tick,
    mode:state.tick>=30?{kind:'nine_slice',border:[0,0,0,0],pixels_per_unit:16}:{kind:'simple'}};
  api.command({op:'set_component',scene_id:sid,entity_id:eid,component:'SpriteRenderer',value});
  api.command({op:'set_component',scene_id:sid,entity_id:eid,component:'Transform',value:{...pose,translation:[state.tick/16,0,0]}});
}});
''',encoding='utf-8')
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit','--target','ES2022',
        '--module','ESNext','--moduleResolution','bundler',str(source)],cwd=ROOT,check=True)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],cwd=ROOT,check=True)
    source_image.unlink()
    file_hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    whole=run('play',project,'--ticks',60,'--compiled-script',compiled,'--save-output',out/'final.save.json')
    first=run('play',project,'--ticks',25,'--compiled-script',compiled,'--save-output',out/'mid.save.json')
    resumed=run('play',project,'--ticks',35,'--compiled-script',compiled,'--load-save',out/'mid.save.json')
    repeat=run('play',project,'--ticks',60,'--compiled-script',compiled)
    assert first['script_state']=={'tick':25}
    assert whole['script_state']==resumed['script_state']==repeat['script_state']=={'tick':60}
    assert whole['state']==resumed['state']==repeat['state']
    saved=json.loads((out/'final.save.json').read_text(encoding='utf-8'))['project']['scenes'][sid]['entities'][eid]['components']
    assert saved['SpriteRenderer']['region']==[0,0,1,1] and saved['SpriteRenderer']['order']==60
    assert saved['SpriteRenderer']['flip_x'] and saved['SpriteRenderer']['flip_y']
    assert saved['SpriteRenderer']['mode']=={'kind':'nine_slice','border':[0,0,0,0],'pixels_per_unit':16}
    assert saved['Transform']['translation']==[3.75,0,0]
    assert file_hashes=={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    assert digest==hashlib.sha256(binary.read_bytes()).hexdigest()
    result={'passed':True,'strict_typescript':True,'source_removed_before_runtime':True,'exact_saved_and_repeated_state':True,
            'atomic_rollback_and_undo_redo':True,'authored_hashes':file_hashes,'binary_sha256':digest,'rendering_or_device_gate':False}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps(result))


if __name__=='__main__':
    main()
