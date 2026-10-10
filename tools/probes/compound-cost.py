#!/usr/bin/env python3
"""Measure local full-tick compound cost; never a device/game performance gate."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='new output directory')
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/incant_headless')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = str(args.binary.resolve())

    def run(*args):
        return json.loads(subprocess.check_output([binary, *map(str, args)], text=True, cwd=ROOT))

    source = output/'observe.ts'
    source.write_text('import type { ScriptApi } from '+json.dumps((ROOT/'sdk/ts/src/index').as_posix())+';\n'+'''export default defineBehavior<{ticks:number}>({
  initialState:{ticks:0}, update(_api:ScriptApi,_dt,state){state.ticks++;}
});
''')
    subprocess.run(['node',str(ROOT/'node_modules/typescript/bin/tsc'),'--strict','--noEmit',
                    '--target','ES2022','--module','ESNext','--moduleResolution','bundler',str(source)],
                   check=True,cwd=ROOT)
    compiled = output/'observe.js'
    subprocess.run(['node', str(ROOT/'tools/build_script.mjs'), str(source), str(compiled)],
                   check=True, cwd=ROOT)

    def transform(at):
        return {'translation':at, 'rotation':[0,0,0,1], 'scale':[1,1,1]}

    def collider(shape):
        return {'shape':shape, 'density':1000, 'friction':.5, 'restitution':0,
                'sensor':False, 'memberships':4294967295, 'filter':4294967295}

    result = {'binary_sha256':hashlib.sha256(Path(binary).read_bytes()).hexdigest(),
              'bodies':32, 'ticks':180, 'trials':3, 'sleeping':False, 'ccd':True,
              'rendering':False, 'game_or_device_gate':False, 'cases':{}}
    for nx, ny, nz in [(1,1,1), (2,2,1), (4,2,2), (4,4,4)]:
        count = nx*ny*nz
        case = output/str(count)
        case.mkdir()
        project = case/'cost.incant.json'
        run('init', project, '--entities', 0, '--name', f'{count}-part cost probe')
        sid = next(iter(json.loads(project.read_text())['scenes']))
        parts = []
        for x in range(nx):
            for y in range(ny):
                for z in range(nz):
                    parts.append({'id':f'{len(parts)+100:026d}',
                                  'translation':[(x+.5)/nx-.5, (y+.5)/ny-.5, (z+.5)/nz-.5],
                                  'rotation':[0,0,0,1], 'shape':{'type':'box',
                                  'half_extents':[.5/nx,.5/ny,.5/nz]}})
        commands = []
        for i in range(33):
            components = {'Transform':transform([0,-.5,0] if i == 0 else
                          [((i-1)%8-3.5)*1.4, 3+(i%3)*.2, ((i-1)//8-1.5)*1.4]),
                          'Collider':collider({'type':'box','half_extents':[20,.5,20]} if i == 0
                          else {'type':'compound','parts':parts})}
            if i:
                components['RigidBody'] = {'motion':'dynamic', 'gravity_scale':1,
                    'linear_damping':0, 'angular_damping':0, 'can_sleep':False, 'ccd':True}
            commands.append({'op':'create_entity', 'scene_id':sid, 'entity':{
                'id':f'{i+10:026d}', 'name':f'Body {i}', 'parent':None,
                'components':components, 'provenance':None}})
        request = {'id':1, 'method':'command.execute', 'params':{'commands':commands,
                   'expected_revision':0, 'description':'Create bounded compound cost workload'}}
        save = {'id':2, 'method':'project.save', 'params':{}}
        response = subprocess.check_output([binary,'rpc',str(project)], text=True, cwd=ROOT,
                    input=json.dumps(request)+'\n'+json.dumps(save)+'\n')
        replies = [json.loads(line) for line in response.splitlines()]
        assert len(replies) == 2 and all('error' not in reply for reply in replies), replies
        before = hashlib.sha256(project.read_bytes()).hexdigest()
        trials = [run('script', project, compiled, '--ticks',180) for _ in range(3)]
        assert all(t['state']['ticks'] == 180 for t in trials)
        # The full playback result independently verifies physics repeatability.
        first = run('play',project,'--ticks',180)
        second = run('play',project,'--ticks',180)
        assert first['completed'] and first['state'] == second['state']
        assert before == hashlib.sha256(project.read_bytes()).hexdigest()
        summary = {'median_p95_ms':statistics.median(t['p95_frame_ms'] for t in trials),
                   'trials':[{'p95_ms':t['p95_frame_ms'], 'max_ms':t['max_frame_ms']} for t in trials],
                   'final_state_equal':True, 'author_unchanged':True}
        result['cases'][str(count)] = summary
        (case/'trials.json').write_text(json.dumps(trials,indent=2)+'\n')
        (case/'play.json').write_text(json.dumps(first,indent=2)+'\n')
        print(json.dumps({'parts':count, **summary}), flush=True)
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n')


if __name__ == '__main__':
    main()
