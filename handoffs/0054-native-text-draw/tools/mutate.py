"""Apply one source mutation at a time, run the text tests, restore exact bytes.

Usage: python3 -I -B mutate.py LOG. Each mutation must make at least one test
fail; the original file hash is verified after every restore.
"""
import hashlib, os, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
R = 'crates/incant_render/src/'
MUTATIONS = [
    ('quad shifted by its padding', R + 'text/geometry.rs',
     'let left = f64::from(position[0]) + f64::from(origin[0]) - 1.;',
     'let left = f64::from(position[0]) + f64::from(origin[0]) - 0.;', 'text::'),
    ('layout Y not flipped', R + 'text/geometry.rs',
     'DMat4::from_scale(DVec3::new(unit, -unit, 1.))',
     'DMat4::from_scale(DVec3::new(unit, unit, 1.))', 'text::'),
    ('motion transforms not staged', R + 'text/mod.rs',
     'update.world = geometry::node_matrix(node.world, geometry.bounds)?;',
     'let _ = geometry::node_matrix(node.world, geometry.bounds)?;', 'retained::text_tests::retained_motion'),
    ('atlas prefix drops right padding column', R + 'text/geometry.rs',
     'plan.extent[0] = plan.extent[0].max(rect[0] + rect[2] + 1);',
     'plan.extent[0] = plan.extent[0].max(rect[0] + rect[2]);', 'text::draw_tests'),
    ('single tap when minified', R + 'text/text.wgsl',
     'let taps = u32(clamp(ceil(max(length(dx), length(dy)) - 0.25), 1.0, 4.0));',
     'let taps = 1u;', 'text::draw_tests::rotated'),
    ('missing text binding accepted', R + 'retained.rs',
     '            (None, None) => {}\n            _ => {',
     '            (None, None) => {}\n            (None, Some(_)) | (Some(_), None) => {}\n            #[allow(unreachable_patterns)]\n            _ => {', 'retained::text_tests::retained_motion'),
    ('no depth test', R + 'text/drawing.rs',
     'depth_compare: Some(wgpu::CompareFunction::LessEqual),',
     'depth_compare: Some(wgpu::CompareFunction::Always),', 'retained::text_tests::opaque'),
    ('foreign handles not checked', R + 'text/geometry.rs',
     '''            let entry = input
                .atlas
                .get(handle)
                .ok_or(SceneError::TextGlyph(node.id))?;''',
     '''            let Some(entry) = input.atlas.get(handle) else {
                continue;
            };''', 'text::'),
]

def run(filter_):
    env = dict(os.environ, CARGO_TARGET_DIR='/tmp/incant-agent-native-target')
    cmd = [str(ROOT / 'tools/cargo'), 'test', '--locked', '--offline', '--profile', 'ci', '-j2',
           '-p', 'incant_render', '--lib', filter_, '--', '--include-ignored', '--test-threads=1']
    result = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True, timeout=1800)
    summary = [l for l in result.stdout.splitlines() if l.startswith('test result') or ' FAILED' in l]
    return result.returncode, summary

def main():
    log = open(sys.argv[1], 'w', encoding='utf-8')
    failures_seen = 0
    for name, path, old, new, filter_ in MUTATIONS:
        file = ROOT / path
        original = file.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode('utf-8')
        assert text.count(old) == 1, (name, text.count(old))
        file.write_text(text.replace(old, new), encoding='utf-8')
        try:
            code, summary = run(filter_)
        finally:
            file.write_bytes(original)
        assert hashlib.sha256(file.read_bytes()).hexdigest() == digest
        caught = code != 0
        failures_seen += caught
        log.write(f'{name}: {"CAUGHT" if caught else "NOT CAUGHT"} ({filter_})\n')
        for line in summary:
            log.write(f'    {line}\n')
        log.flush()
        print(name, 'CAUGHT' if caught else 'NOT CAUGHT', flush=True)
    code, summary = run('text')
    log.write(f'unmutated: {"PASS" if code == 0 else "FAIL"}\n')
    for line in summary:
        log.write(f'    {line}\n')
    print('unmutated', 'PASS' if code == 0 else 'FAIL')
    sys.exit(0 if failures_seen == len(MUTATIONS) and code == 0 else 1)

main()
