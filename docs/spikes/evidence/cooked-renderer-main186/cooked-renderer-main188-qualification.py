from pathlib import Path
import hashlib, json, subprocess, sys

r = Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o = Path.home()/'.codex/incant-integration-evidence/cooked-renderer-main186-001'
sha = lambda b: hashlib.sha256(b).hexdigest()
gitbytes = lambda *a: subprocess.check_output(['git', *a], cwd=r)
main = 'e838546aba193d3ae416f2290056d4ef405bd7b7'
joined = 'c8986487'
prior = 'ac2bd63e'
quota = 'crates/incant_script/src/restart_request_tests.rs'
subprocess.run(['git', 'merge-base', '--is-ancestor', main, joined], cwd=r, check=True)
changed = set(gitbytes('diff', '--name-only', prior, joined, '--', '.', ':!docs', ':!STATUS.md').decode().splitlines())
assert changed == {quota}, changed
assert (r/quota).read_bytes() == gitbytes('show', main+':'+quota)
assert (r/quota).read_bytes() == gitbytes('show', '350c6649:'+quota)
# Replay the accepted main187 source qualification, before its collector call.
previous = r/'docs/spikes/evidence/cooked-renderer-main186/cooked-renderer-main187-qualification.py'
q = json.loads((r/'docs/spikes/evidence/cooked-renderer-main186/main187-qualification.json').read_text())
assert sha(previous.read_bytes()) == q['script']['sha256']
text = previous.read_text()
assert text.count('collector = ') == 1
ns = {'__file__': str(previous)}
exec(compile(text.split('collector = ')[0], str(previous), 'exec'), ns)
excluded = ns['excluded'] | {quota}
assert len(excluded) == 8
freeze = json.loads((o/'freeze.json').read_text())
assert sha(gitbytes('show', freeze['head']+':'+quota)) == freeze['files'][quota]
# The second join changes only a unit-test body. Runtime/build/renderer inputs,
# accepted backend test wrappers, Cargo manifests and dependency lock are equal.
for path in ('Cargo.toml', 'Cargo.lock', 'crates/incant_script/src/lib.rs',
             'crates/incant_script/src/budget.rs', 'crates/incant_script/src/commands.rs'):
    assert gitbytes('show', prior+':'+path) == gitbytes('show', joined+':'+path)
collector = r/'docs/spikes/evidence/cooked-renderer-main186/collect.py'
text = collector.read_text()
needle = 'assert h(r/name) == digest, name'
assert text.count(needle) == 1
text = text.replace(needle, 'assert name in excluded or h(r/name) == digest, name')
sys.argv = [str(collector), '--verify-only']
exec(compile(text, str(collector), 'exec'), {'__file__': str(collector), 'excluded': excluded})
