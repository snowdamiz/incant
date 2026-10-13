#!/usr/bin/env python3
"""Retain exact CPU profiler evidence; never rebuild or change original proof."""
import gzip,hashlib,json,shutil,sys
from pathlib import Path

ROOT=Path('/Users/andreyyurlov/.codex/worktrees/cooked-visual-assets/incant')
BASE=Path.home()/'.codex/incant-evidence'
DST=ROOT/'docs/spikes/evidence/native-cpu-profiling'

def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for c in iter(lambda:f.read(65536),b''):h.update(c)
 return h.hexdigest()

def write(p,d):p.write_text(json.dumps(d,indent=2)+'\n')

def collect(proof):
 DST.mkdir(parents=True,exist_ok=False);rows=[]
 def retain(p,key):
  dest=DST/key;dest.parent.mkdir(parents=True,exist_ok=True)
  rawsha=sha(p);rawsize=p.stat().st_size
  if rawsize>16384 and p.suffix not in ('.gz','.zip','.tracy'):
   dest=dest.with_name(dest.name+'.gz')
   with p.open('rb') as src,dest.open('wb') as out:
    with gzip.GzipFile(filename='',fileobj=out,mode='wb',mtime=0) as gz:shutil.copyfileobj(src,gz)
   codec='gzip'
  else:shutil.copyfile(p,dest);codec='raw'
  rows.append({'original':str(p),'retained':str(dest.relative_to(ROOT)),'raw_sha256':rawsha,'raw_bytes':rawsize,'encoding':codec,'sha256':sha(dest),'bytes':dest.stat().st_size})
 for name in ['native-cpu-profiler-source-001','native-cpu-profiler-source-002','native-cpu-profiler-source-003','native-cpu-profiler-source-004','native-cpu-profiler-helper-001','native-cpu-profiler-helper-002','native-cpu-profiler-helper-003','native-cpu-profiler-helper-004','native-cpu-profiler-helper-005','native-cpu-profiler-engine-001','native-cpu-profiler-checks-001']:
  for p in sorted((BASE/name).rglob('*')):
   if p.is_file() and '__pycache__' not in p.parts:retain(p,Path(name)/p.relative_to(BASE/name))
 for name in ['native-cpu-profiler-export-001','native-cpu-profiler-export-002','native-cpu-profiler-export-003']:
  for p in sorted((BASE/name).rglob('*')):
   if p.is_file() and not any(x.startswith('package-') for x in p.relative_to(BASE/name).parts):retain(p,Path(name)/p.relative_to(BASE/name))
 for p in sorted(proof.rglob('*')):
  if p.is_file() and not any(x.startswith('package-') for x in p.relative_to(proof).parts):retain(p,Path(proof.name)/p.relative_to(proof))
 packages=[]
 package_root=Path(json.loads((proof/'result.json').read_text())['package_root'])
 for name in ['package-summary','package-tracy']:
  for p in sorted((package_root/name).rglob('*')):
   if not p.is_file():continue
   packages.append({'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size})
   if p.name in ['export.json','codegen-audit.json'] or p.suffix in ('.json','.log','.stdout','.stderr','.txt'):
    retain(p,Path(package_root.name)/p.relative_to(package_root))
 for name in ['native-cpu-profiler-source-20261013.json','native-cpu-profiler-export-source-20261013.json','native-cpu-profiler-helper-002-review.json','native-cpu-profiler-helper-final-20261013.json','native-cpu-profiler-helper-004-review.json','native-cpu-profiler-helper-005-review.json']:
  p=Path.home()/'.codex/incant-reviews'/name;retain(p,Path('reviews')/name)
 for name in ['native-cpu-profiler-audit.py','native-cpu-profiler-collect.py']:
  retain(BASE/name,Path('tools')/name)
 inspection=BASE/'native-cpu-profiler-inspection'
 for name in ['tracy-tools-0.14.0/acquisition.json']:
  retain(inspection/name,Path('inspection')/name)
 manifest={'status':'final execution awaiting independent audit','base':'1c5658cf5d7eee8390b272394f38b8d9b49e1628','source_freeze':str(BASE/'native-cpu-profiler-source-004/freeze.json'),'helper_freeze':str(BASE/'native-cpu-profiler-helper-005/freeze.json'),'proof_root':str(proof),'artifacts':rows,'package_files':packages,'result':json.loads((proof/'result.json').read_text()),'limitations':['CPU diagnostic functionality; no performance budget/gain or reference-phone claim','Default feature graph and default tests pass; shipped inactive baselines use the same summary/Tracy-capable binaries with runtime profiling absent','Process-scoped requested Rust bytes separate from QuickJS sampled VM heap; neither is RSS','Explicit localhost Tracy process transport; no GUI or GPU; no arbitrary external compiler/environment override acceptance','Historical pre-export failures retained; original proof is never overwritten']}
 manifest['source_identities']=[r for r in json.loads((BASE/'native-cpu-profiler-source-004/freeze.json').read_text())['sources'] if not r['path'].startswith('docs/')]
 manifest['helper_identities']=json.loads((BASE/'native-cpu-profiler-helper-005/freeze.json').read_text())['sources']
 manifest['engine_inventory']={'path':str(BASE/'native-cpu-profiler-engine-001/inventory.json'),'sha256':sha(BASE/'native-cpu-profiler-engine-001/inventory.json'),'count':1426}
 manifest['post_execution_docs']=['docs/spikes/native-cpu-profiling.md','STATUS.md']
 manifest['test_counts']={'affected':{'passed':495,'failed':0,'ignored':0,'groups':62},'default_player':{'passed':19,'groups':1},'focused_python_export':23,'process_lifecycle':4,'overlap':'Focused Rust tests overlap affected/default counts; do not sum as unique tests.'}
 manifest['external_binaries']=[{'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size} for p in [Path('/tmp/incant-cpu-profiler-target/debug/incant_headless'),*(Path(x) for x in manifest['result']['tools'])]]
 out=ROOT/'docs/spikes/evidence/native-cpu-profiling.json';write(out,manifest);print(out,sha(out),len(rows),len(packages))

if __name__=='__main__':collect(Path(sys.argv[1]).resolve())
