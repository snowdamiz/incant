from pathlib import Path
import sys,json,subprocess
root=Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant');sys.path.insert(0,str(root/'tools'));import native_process
out=Path(__file__).resolve().parent
label,project=sys.argv[1:]
argv=[str(out/'Incant UI Review.app/Contents/MacOS/incant_editor'),str(out/project)]
(out/(label+'-launch.json')).write_text(json.dumps({'argv':argv,'timeout_seconds':900},indent=2)+'\n')
with (out/(label+'.log')).open('xb') as log:r=native_process.run(argv,cwd=root,stdout=log,stderr=subprocess.STDOUT,timeout=900)
(out/(label+'-exit.json')).write_text(json.dumps({'exit':r.returncode,'owned_process_group_cleaned_and_reaped':True},indent=2)+'\n')
print('native exited',r.returncode)
