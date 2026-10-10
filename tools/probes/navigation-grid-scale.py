#!/usr/bin/env python3
"""Measure the full grid storage/search bound through a public script query."""
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
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    project = out/'grid.incant.json'
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    sha = digest(binary)
    def run(*argv):
        return json.loads(subprocess.check_output([str(binary), *map(str, argv)], text=True, cwd=ROOT))
    run('init', project, '--name', 'Grid search bound', '--entities', 0)
    scene = next(iter(json.loads(project.read_text())['scenes']))
    entity = '00000000000000000000000001'
    costs = [1]*65536
    # Keep the destination walkable but disconnect it. A no-route answer must
    # exhaust the reachable cells; this avoids timing only an easy straight path.
    costs[-2] = costs[-257] = costs[-258] = 0
    requests = [
        {'id':1,'method':'command.execute','params':{'expected_revision':0,
          'description':'Author maximum-size disconnected grid','commands':[
          {'op':'create_entity','scene_id':scene,'entity':{'id':entity,'name':'Grid',
           'parent':None,'provenance':None,'components':{'NavigationGrid':{
           'dimensions':[256,256],'costs':costs}}}}]}},
        {'id':2,'method':'project.save'}]
    output = subprocess.check_output([str(binary),'rpc',str(project)],
        input=''.join(json.dumps(r)+'\n' for r in requests),text=True,cwd=ROOT)
    assert all('error' not in json.loads(line) for line in output.splitlines()), output
    source, compiled = out/'bound.ts', out/'bound.js'
    source.write_text('import type {ScriptApi} from '+json.dumps(str(ROOT/'sdk/ts/src/index'))+';\n'
      +'const scene='+json.dumps(scene)+',grid='+json.dumps(entity)+';\n'+'''
export default defineBehavior<{queries:number}>({initialState:{queries:0},update(api:ScriptApi,dt,state){
  const path=api.findGridPath({scene_id:scene,grid_entity:grid,path:{start:[0,0],end:[255,255],diagonal:true,max_expansions:65536}});
  if(path!==null)throw Error('Disconnected endpoint unexpectedly reachable');
  state.queries++;
}});
''')
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit',
      '--target','ES2022','--module','ESNext','--moduleResolution','bundler',str(source)],check=True,cwd=ROOT)
    subprocess.run(['node',str(ROOT/'tools/build_script.mjs'),str(source),str(compiled)],check=True,cwd=ROOT)
    authored = digest(project)
    runs=[]
    for i in range(3):
        report=run('script',project,compiled,'--ticks',30)
        assert report['state']=={'queries':30}
        (out/f'run{i+1}.json').write_text(json.dumps(report,indent=2)+'\n')
        runs.append({'p95_frame_ms':report['p95_frame_ms'],'max_frame_ms':report['max_frame_ms']})
    assert digest(project)==authored and digest(binary)==sha
    result={'passed':True,'cells':65536,'reachable_cells_per_query':65532,'queries_per_run':30,
      'runs':runs,'binary_sha256':sha,'project_sha256':authored,
      'measurement':'Headless script ticks including host synchronization; no rendering, no controlled contention or device gate'}
    (out/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))

if __name__=='__main__':
    main()
