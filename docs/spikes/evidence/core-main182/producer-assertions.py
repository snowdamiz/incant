#!/usr/bin/env python3
"""Assertion-only re-audit of preserved Core process outputs; never launches a game."""
import hashlib,json,math,struct,sys
from pathlib import Path
REPO=Path('/Users/andreyyurlov/.codex/worktrees/color-mip-roles/incant')
E=Path.home()/'.codex/incant-evidence/core-runtime-bindings-001'
sys.path.insert(0,str(REPO/'samples/core'))
import binding_oracle as oracle
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
read=lambda p:json.loads(p.read_text())
freeze=read(E.parent/'core-runtime-bindings-source-006/inventory.json')
for row in freeze['sources']:
 assert sha(REPO/row['path'])==row['sha256'],row['path']
for row in read(E/'build-inputs.json')['rows']:
 assert sha(REPO/row['path'])==row['sha256'],row['path']
n=E/'native-003';s=E/'shipping-003';proof=E/'shipping-001';prepared=read(n/'prepared.json');result=read(s/'result.json');assert prepared['passed'] and result['passed']
for rel,h in result['package_files'].items(): assert sha(proof/'package'/rel)==h,rel
export=read(proof/'package/export.json');player=proof/'package/player/incant_game_player'
assert sha(player)==export['player_sha256']==result['player_sha256']
assert not (proof/'authoring').exists() and not list((proof/'package').rglob('*.map'))
assert export['build_storage_removed_before_startup']
assert read(proof/'deleted-authoring.json')['gameplay.ts']==prepared['source_sha256']
assert export['project_sha256']==prepared['project_sha256']
receipt=prepared['receipt'];texts=read(n/'localized.json');recordings=oracle.cases()
for name in recordings:
 native=read(n/f'{name}.json');shipped=read(s/f'{name}.json');oracle.verify(native,receipt,name);oracle.verify(shipped,receipt,name)
 for field in ('tick','frames','logs','observations','pcm_sha256'):assert native[field]==shipped[field],(name,field)
 assert oracle.pcm(n/f'{name}.pcm',native)==oracle.pcm(s/f'{name}.pcm',shipped)
 assert sha(s/f'{name}.pcm')==shipped['pcm_sha256']
for name in ('win','lose'):
 observed=read(n/'observer'/f'{name}.json');oracle.verify_hud(observed,texts['en'])
 assert oracle.game_logs(observed)==read(n/f'{name}.json')['logs']['entries']
 assert (n/'observer'/f'{name}.pcm').read_bytes()==(n/f'{name}.pcm').read_bytes()
for locale in ('ar','zh-Hans','en-XA'):
 observed=read(n/f'locale-{locale}/win.json');oracle.verify_hud(observed,texts[locale]);assert (n/f'locale-{locale}/win.pcm').read_bytes()==(n/'win.pcm').read_bytes()
for role in oracle.MUTE_KEYS:
 import wave
 case='lose' if role=='lost' else 'win';clip={'pickup':'cue_pickup','damage':'cue_hurt','won':'cue_won','lost':'cue_lost'}[role]
 with wave.open(str(n/'authoring/art'/f'{clip}.wav')) as sound:seconds=sound.getnframes()/sound.getframerate()
 actual=oracle.cue_difference((n/f'{case}.pcm').read_bytes(),(n/'observer'/f'mute-{role}.pcm').read_bytes(),read(n/f'{case}.json'),role,seconds)
 assert actual==prepared['cue_checks'][role]
records,restarts=oracle.restart_cases();short=read(s/'lose-short.json');oracle.verify(short,receipt,'lose')
assert (s/'lose.pcm').read_bytes().startswith((s/'lose-short.pcm').read_bytes())
assert not any((s/'lose-short.pcm').read_bytes()[-oracle.TICK_BYTES*60:])
for name,first,second,pad,held in restarts:
 report=read(s/f'{name}.json');a=read(s/f'{first}.json');b=read(s/f'{second}.json');one,two=records[first],records[second]
 assert read(s/f'{name}-input.json')==oracle.restarted(one,two,pad=pad,held=held)
 plan=oracle.restart_plan(receipt,first,second,one,two,held=held);assert read(s/f'{name}-observe.json')==plan
 assert report['observations']['passed'] and len(report['observations']['checks'])==len(plan['checks'])
 assert all(c['passed'] for c in report['observations']['checks'])
 for field in ('logs','render_input','flipbooks','sources'):assert report[field]==b[field],(name,field)
 assert report['observations']['entities']==b['observations']['entities']
 assert report['session']=={'generation':2,'host_tick':one['ticks']+two['ticks'],'simulation_tick':two['ticks'],'restarts':[{'host_step':one['ticks'],'completed_generation':1,'simulation_tick':one['ticks'],'output_frames':str(a['frames'])}]}
 data=oracle.pcm(s/f'{name}.pcm',report);assert data==(s/f'{first}.pcm').read_bytes()+(s/f'{second}.pcm').read_bytes()
 assert sha(s/f'{name}.pcm')==report['pcm_sha256']
positive=negative=0
for base in (n,s):
 for p in sorted(base.glob('command-*.json')):
  cmd=read(p)
  if cmd['exit_code']==0:
   report=json.loads(cmd['stdout']);assert report['observations']['passed'];assert report['logs']['dropped']==0;positive+=1
   name=Path(cmd['argv'][cmd['argv'].index('--observe')+1]).name.removesuffix('-observe.json')
   saved=Path(cmd['argv'][cmd['argv'].index('--observe')+1]).with_name(name+'.json');assert report==read(saved)
  else:
   assert base==n and not cmd['stdout'];assert cmd['stderr']=='Error: native play startup/runtime failure: script tick failed: TypeError: host does not accept session restart requests\n';negative+=1
assert (positive,negative)==(24,1),(positive,negative)
red=read(proof/'command-006.json');assert red['exit_code']==1 and red['stderr']=='Error: "tick/sample-rate bounds"\n' and not red['stdout']
assert not (proof/'win-restart-win.pcm').exists()
output={'passed':True,'source_rows':len(freeze['sources']),'build_rows':len(read(E/'build-inputs.json')['rows']),'positive_processes':positive,'precise_headless_refusals':negative,'retained_overlimit_restart_refusals':1,'shipping_processes':10,'native_processes':15,'restart_exact_pcm_pairs':len(restarts),'package_files':len(result['package_files']),'player_sha256':sha(player),'player_bytes':player.stat().st_size,'source_only_observer':True,'gpu':False}
(E/'audit-result.json').write_text(json.dumps(output,indent=2)+'\n');print(json.dumps(output,indent=2))
