from pathlib import Path
import hashlib, json, subprocess, sys

r = Path('/Users/andreyyurlov/.codex/worktrees/ci-build-cache/incant')
o = Path.home()/'.codex/incant-integration-evidence/cooked-renderer-main186-001'
sha = lambda b: hashlib.sha256(b).hexdigest()
gitbytes = lambda *a: subprocess.check_output(['git', *a], cwd=r)
main = 'b4c8b2f507a10000fa4dfceb169433278bce37a8'
tested = '385d623b'
joined = 'b7bbbe4d'
subprocess.run(['git', 'merge-base', '--is-ancestor', main, joined], cwd=r, check=True)
expected = {
    'crates/incant_cook/src/project_build.rs',
    'crates/incant_cook/src/project_build_wasm.rs',
    'crates/incant_cook/tests/mobile_project.rs',
    'tools/export_ios.py', 'tools/ios_archive.py', 'tools/ios_wasm.py',
    'tools/native_wasm.py', 'tools/wasm2c.py',
    'tools/probes/authored-ios-wasm/prepare.py',
    'tools/probes/authored-ios-wasm/verify.py',
    'tools/tests/fixtures/ios-wasm-cargo.txt',
    'tools/tests/test_ios_export.py', 'tools/tests/test_ios_wasm.py',
}
changed = set(gitbytes('diff', '--name-only', tested, joined, '--', '.', ':!docs', ':!STATUS.md').decode().splitlines())
assert changed == expected, changed
f = json.loads((o/'freeze.json').read_text())
qualified = {}
for path in sorted(expected):
    actual = (r/path).read_bytes()
    assert actual == gitbytes('show', main+':'+path)
    assert actual == gitbytes('show', joined+':'+path)
    qualified[path] = sha(actual)
excluded = expected & f['files'].keys()
assert len(excluded) == 7
for path in excluded:
    assert sha(gitbytes('show', tested+':'+path)) == f['files'][path]
for path in ('Cargo.toml', 'Cargo.lock'):
    assert gitbytes('show', tested+':'+path) == gitbytes('show', joined+':'+path)
# None of the new source paths participates in these renderer/compiler roots.
assert not gitbytes('diff', '--name-only', tested, joined, '--',
    'crates/incant_render', 'crates/incant_render_gpu_host', 'crates/incant_shader_cook',
    '.cargo', 'rust-toolchain.toml', 'tools/cargo', '.github/workflows/ci.yml', '.github/workflows/desktop.yml')
collector = o.parent/'collect-cooked-renderer-main186.py'
text = collector.read_text()
needle = 'assert h(r/name) == digest, name'
assert text.count(needle) == 1
text = text.replace(needle, 'assert name in excluded or h(r/name) == digest, name')
sys.argv = [str(collector), '--verify-only']
exec(compile(text, str(collector), 'exec'), {'__file__': str(collector), 'excluded': excluded})
