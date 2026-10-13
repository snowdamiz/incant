#!/usr/bin/env python3
"""Read-only assertions over the CPU profiler's retained source/actual process proof."""
import csv,gzip,hashlib,io,json,math,sys
from pathlib import Path

def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for chunk in iter(lambda:f.read(65536),b''):h.update(chunk)
 return h.hexdigest()

def audit(proof):
 result=json.loads((proof/'result.json').read_text()); assert result['passed']
 commands={p.stem:json.loads(p.read_text()) for p in (proof/'commands').glob('*.json')}
 assert len(commands)==result['commands']
 for name,row in commands.items():
  assert row['exit'] is not None,(name,'not terminal')
  for stream in ['stdout','stderr']:
   p=proof/'commands'/f'{name}.{stream}'
   assert digest(p)==row[f'{stream}_sha256'],p
   assert p.read_text()==row[stream],p
  assert row['exception'] is None,(name,row['exception'])
 assert commands['csv-version']['stdout'].startswith('tracy-csvexport 0.14.0 / ')
 package_root=Path(result.get('package_root',str(proof)))
 authored=json.loads((package_root/'authoring.json').read_text())
 assert not (package_root/'authoring').exists()
 for mode,rows in result['packages'].items():
  package=package_root/('package-'+mode)
  assert {str(p.relative_to(package)):digest(p) for p in package.rglob('*') if p.is_file()}==rows
  export=json.loads((package/'export.json').read_text()); player=package/'player/incant_game_player'
  assert digest(player)==export['player_sha256']
  assert export['project_sha256']==authored['project_sha256']
  assert export['profiling']['mode']==mode
  assert export['build_storage_removed_before_startup']
  assert not list(package.rglob('*.map'))
 baseline=None; pcm=None; samples=0
 for name in ['summary-baseline','summary-summary','tracy-baseline','tracy-summary','tracy-captured']:
  row=commands[name]; assert row['exit']==0 and not row['stderr']
  report=json.loads(row['stdout']); profile=report.pop('profile',None)
  assert report['tick']==3600 and report['observations']['passed']
  output=proof/(name+'.pcm');assert digest(output)==report['pcm_sha256']
  assert output.stat().st_size==report['frames']*8
  # This mathematical script fixture has no sound source: exact silence also
  # establishes finite PCM without labeling it a sound-content workload.
  with output.open('rb') as f:
   assert all(not any(block) for block in iter(lambda:f.read(65536),b''))
  if baseline is None:baseline=report;pcm=digest(output)
  assert report==baseline and digest(output)==pcm
  if name.endswith('baseline'):assert profile is None and not row['network']
  else:
   assert profile['completed_frames']==3600 and profile['failed_frames']==0
   assert profile['quickjs_heap']['samples']==3600 and profile['quickjs_heap']['latest']['used_bytes']>0
   assert profile['rust_allocations']['installed'] and not profile['rust_allocations']['saturated']
   assert profile['dropped_source_samples']==0 and len(profile['scripts'])==2
   assert {tuple(r['source']) for r in profile['scripts']}=={tuple(i.to_bytes(16,'big')) for i in (1,2)}
   assert all(r['totals']['calls']==3600 for r in profile['scripts'])
   assert profile['tracy_process_transport']==(name=='tracy-captured')
   assert 'PRIVATE-authored-marker' not in json.dumps(profile)
   if name!='tracy-captured':assert not row['network']
  assert row['network_queries']>0
  samples+=report['frames']*2
 for name,diagnostic in [('duplicate','Error: Options\n'),('summary-transport','Error: TracyUnavailable\n'),('unsupported','Error: UnsupportedRoute\n')]:
  row=commands['refusal-'+name];assert row['exit']!=0 and not row['stdout'] and row['stderr']==diagnostic
 for i in range(2):
  row=commands['refusal-guest-'+str(i)];assert row['exit']!=0 and not row['stdout']
  assert row['stderr']==result['guest_diagnostic'] and 'cpu-profiler deliberate trap' in row['stderr']
  assert not (proof/f'failed-{i}.pcm').exists()
 assert not list(proof.glob('.incant-pcm-*'))
 network=commands['tracy-captured']['network'];assert network
 assert any('127.0.0.1:8086 (LISTEN)' in s for s in network)
 for s in network:
  for row in s.splitlines()[1:]:assert ' TCP ' in row and '127.0.0.1:' in row and '*:' not in row
 assert commands['capture']['exit']==0 and digest(proof/'capture.tracy')==result['capture_sha256']
 zones=list(csv.DictReader(io.StringIO(commands['trace-zones']['stdout'])))
 assert zones==result['zones']
 assert {'player.frame','native.tick','world.step','ecs.integrate','script.tick','script.source','game.audio_render'}<={r['name'] for r in zones if int(r['counts'])>0}
 assert commands['trace-zones']['exit']==commands['trace-events-plots']['exit']==0
 events=commands['trace-events-plots']['stdout'];assert 'PRIVATE-authored-marker' not in events
 rows=list(csv.DictReader(io.StringIO(events)))
 for name,count in result['plot_samples'].items():
  values=[float(r['value']) for r in rows if r['name']==name and not r['src_file']]
  assert len(values)==count and values and all(math.isfinite(v) and v>0 for v in values)
 return {'passed':True,'commands':len(commands),'full_runs':5,'refusals':5,'compared_pcm_samples':samples,'captured_zones':len(zones),'plot_samples':result['plot_samples']}

if __name__=='__main__':print(json.dumps(audit(Path(sys.argv[1]).resolve()),indent=2))
