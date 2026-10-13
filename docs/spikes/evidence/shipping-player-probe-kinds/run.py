from pathlib import Path
import subprocess,os,json,time,hashlib
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');o=Path.home()/'.codex/incant-integration-evidence/shipping-player-probe-kinds-main190';h=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();freeze=json.loads((o/'source-freeze.json').read_text());rows=[]
e=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');e['PATH']='/Users/andreyyurlov/.local/share/fnm/node-versions/v22.23.3/installation/bin:/opt/homebrew/bin:'+e['PATH']
commands=[('focused',['python3','-B','-m','unittest','discover','-s','tools/tests','-p','test_player_probe_codegen.py','-v']),('actual-log',['python3',str(o/'actual-green.py')]),('tools',['npm','run','test:tools']),('conventions',['python3','tools/generate_conventions.py','--check']),('diff',['git','diff','--check'])]
for name,command in commands:
 assert all(h(r/p)==v for p,v in freeze.items());t=time.monotonic();print('starting',name,flush=True)
 with (o/(name+'.stdout')).open('wb') as out,(o/(name+'.stderr')).open('wb') as err:p=subprocess.run(command,cwd=r,env=e,stdout=out,stderr=err)
 row={'name':name,'command':command,'exit':p.returncode,'seconds':time.monotonic()-t}
 for suffix in ('stdout','stderr'):row[suffix]={'path':str(o/(name+'.'+suffix)),'sha256':h(o/(name+'.'+suffix))}
 rows.append(row);(o/'checks.json').write_text(json.dumps(rows,indent=2)+'\n');print(name,p.returncode,flush=True)
 assert all(h(r/p)==v for p,v in freeze.items())
 if p.returncode:raise SystemExit(p.returncode)
(o/'terminal.json').write_text(json.dumps({'exit':0,'commands':len(rows),'unchanged':True},indent=2)+'\n')
