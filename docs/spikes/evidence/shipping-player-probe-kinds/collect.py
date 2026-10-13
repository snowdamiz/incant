from pathlib import Path
import json,hashlib,gzip,base64,subprocess,shutil
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/shipping-player-probe-kinds-main190';d=r/'docs/spikes/evidence/shipping-player-probe-kinds';d.mkdir(parents=True,exist_ok=True)
h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();read=lambda p:json.loads(Path(p).read_text())
f=read(o/'source-freeze.json');assert all(h(r/p)==v for p,v in f.items());assert read(o/'terminal.json')['exit']==0
checks=read(o/'checks.json');assert len(checks)==5 and all(c['exit']==0 for c in checks)
for c in checks:
 for key in ('stdout','stderr'):assert h(c[key]['path'])==c[key]['sha256']
red=read(o/'baseline.json');assert red['exit']==1 and 'Missing fresh target compile for incant_game_player' in (o/'baseline.stderr').read_text();assert h(red['log'])==red['log_sha256']
green=read(o/'actual-log.stdout');assert green['incant_game_player']['lto']=='fat' and green['incant_game_player_library']['panic']=='abort'
paths=[p for p in sorted(o.rglob('*')) if p.is_file()]
cp=o.parent/'checkpoint17';paths += [cp/'terminal-state.json',cp/'macos-checks-115094354322.log']+sorted((cp/'macos-shipping-artifact').glob('*'))
records=[];payloads={}
for p in paths:
 if not p.is_file():continue
 b=p.read_bytes();sha=hashlib.sha256(b).hexdigest();records.append({'original':str(p),'sha256':sha,'bytes':len(b)});payloads.setdefault(sha,base64.b64encode(b).decode())
a=d/'artifacts.json.gz';a.write_bytes(gzip.compress(json.dumps({'records':records,'payloads':payloads},separators=(',',':')).encode(),mtime=0));assert a.stat().st_size<32*1024**2
for src,name in [(Path(__file__),'collect.py'),(o.parent/'check-shipping-player-probe-kinds.py','run.py')]:shutil.copyfile(src,d/name)
manifest={'scope':'Desktop shipping probe helper correction only; no engine or runtime change','base_main':subprocess.check_output(['git','rev-parse','HEAD'],cwd=r,text=True).strip(),'source':f,'checks':checks,'red':red,'actual_green':green,'count_audit':read(o/'count-classification.json'),'archive':{'path':str(a.relative_to(r)),'sha256':h(a),'records':len(records),'payloads':len(payloads)},'limits':['Actual retained hosted compile log is re-audited; no new shipping build or player process claimed','Checkpoint17 remains terminal20SUCCESS/3FAILURE and unpromoted','Next frozen checkpoint must run all23 required contexts successfully'],'review_state':'Final source/evidence review pending'}
p=r/'docs/spikes/evidence/shipping-player-probe-kinds.json';p.write_text(json.dumps(manifest,indent=2)+'\n');print(json.dumps({'manifest':str(p),'sha256':h(p),'records':len(records),'payloads':len(payloads),'archive_bytes':a.stat().st_size}))
