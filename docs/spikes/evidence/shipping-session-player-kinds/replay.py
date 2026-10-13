from pathlib import Path
import hashlib,json,sys
r=Path('/Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant');sys.path.insert(0,str(r/'tools'));from shipping_session import target_commands
p=Path.home()/'.codex/incant-integration-evidence/checkpoint16/failures/shipping-session';o=Path(__file__).parent
h=lambda q:hashlib.sha256(q.read_bytes()).hexdigest()
rows=[json.loads(l) for l in (p/'build-2.stdout.jsonl').read_text().splitlines() if l.startswith('{')]
artifacts=[x for x in rows if x.get('reason')=='compiler-artifact' and x['target']['name']=='incant_game_player'];assert len(artifacts)==2
parts=Path(artifacts[0]['filenames'][0]).parts;i=parts.index('aarch64-apple-darwin');storage=Path(*parts[:i]);target=parts[i]
commands=target_commands((p/'build-2.stderr.log').read_text(),target,storage);out=[]
for a in artifacts:
 crate=a['target']['name'];files=[Path(v) for v in a['filenames']]
 old=[c for c in commands if c['crate']==crate and c['features']==sorted(a['features']) and (a.get('executable') or (c['extra_filename'] and any(q.name.startswith(f'lib{crate}{c["extra_filename"]}.') or q.name.startswith(f'{crate}{c["extra_filename"]}.') for q in files)))]
 new=[c for c in old if c['crate_types']==sorted(a['target']['crate_types'])]
 assert len(new)==1 and not a['fresh']
 assert len(old)==(2 if a['executable'] else 1)
 out.append({'target':a['target'],'old_matches':len(old),'corrected_matches':len(new),'selected_command':new[0]['line']})
receipt={'passed':True,'scope':'Replays exact hosted Cargo/rustc command identity only; no rebuilt artifacts or full shipping-session success claimed','raw':[{'path':str(q),'sha256':h(q)} for q in sorted(p.iterdir()) if q.is_file()],'units':out}
(o/'replay.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps({'passed':True,'units':[(x['target']['kind'],x['old_matches'],x['corrected_matches']) for x in out]}))
