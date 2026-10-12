"""Apply one source mutation at a time, run the UI tests that should catch it,
restore the exact bytes and verify their hash.

Usage: python3 -I -B mutate.py LOG. Commit first and do not edit sources while
it runs. Each mutation must make its selected tests fail; the unmutated suites
must pass afterwards. GPU suites run with --include-ignored under the 0056 lease.
"""
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = Path('/tmp/incant0056')
R = 'crates/incant_render/src/ui/'
RENDER = ('incant_render', ['--lib', 'ui::'], True)
GAME = ('incant_render_game', ['--test', 'ui'], False)
HOSTED = ('incant_render_gpu_host', ['--test', 'ui'], True)
MUTATIONS = [
    ('quads not intersected with their clip', R + 'ui.wgsl',
     [('let low = max(p0, c0);\n    let high = max(min(p1, c1), low);', 'let low = p0;\n    let high = p1;')], RENDER),
    ('glyph pen not snapped to a physical pixel', R + 'ui.wgsl',
     [('let pen = snap(physical(quad.pen));', 'let pen = physical(quad.pen);')], RENDER),
    ('sRGB theme colours not decoded on sRGB targets', R + 'ui.wgsl',
     [('    if frame.encode.x == 0.0 {\n        color = decode(color);\n    }\n', '')], RENDER),
    ('straight instead of premultiplied output', R + 'ui.wgsl',
     [('return vec4f(color * alpha, alpha);', 'return vec4f(color, alpha);')], RENDER),
    ('stroke hole ignored (rings become fills)', R + 'ui.wgsl',
     [('coverage = max(coverage - hole, 0.0);', 'coverage = max(coverage - 0.0 * hole, 0.0);')], RENDER),
    ('glyphs drawn before the selection', R + 'geometry.rs',
     [('    let (drawn, missing) = glyphs(atlas, widget, text_color, [0.; 2], out)?;\n    out.glyphs += drawn;',
       '    out.glyphs += drawn;'),
      ('    let layout = &*widget.layout;\n    let edit = widget',
       '    let layout = &*widget.layout;\n    let (drawn, missing) = glyphs(atlas, widget, text_color, [0.; 2], out)?;\n    let edit = widget')],
     RENDER),
    ('RTL caret uses the LTR leading edge', R + 'geometry.rs',
     [('let leading = |c: &Cluster| if c.rtl { c.right } else { c.left };',
       'let leading = |c: &Cluster| c.left;')], RENDER),
    ('label shadow drawn at the glyph pen', R + 'geometry.rs',
     [('glyphs(atlas, widget, shadow, theme::LABEL_SHADOW_OFFSET, out)?;',
       'glyphs(atlas, widget, shadow, [0.; 2], out)?;')], RENDER),
    ('glyphs clipped to the border box, not the content clip', R + 'geometry.rs',
     [('            clip: widget.clip,\n            extra: [x - 1.', '            clip: widget.visible,\n            extra: [x - 1.')],
     RENDER),
    ('identical input not reused', R + 'mod.rs',
     [('&& Arc::ptr_eq(&accepted.input, input)', '&& Arc::ptr_eq(&accepted.input, input) && false')], RENDER),
    ('shared atlas never reused', R + 'mod.rs',
     [('.is_some_and(|s| Arc::ptr_eq(s, &input.atlas))', '.is_some_and(|s| Arc::ptr_eq(s, &input.atlas) && false)')],
     RENDER),
    ('stage accepts another renderer', R + 'mod.rs',
     [('        if !Arc::ptr_eq(&self.owner, &renderer.identity) {\n            return Err(SceneError::Input("UI layer belongs to another renderer").into());\n        }\n', '')],
     RENDER),
    ('consumer offered the accepted UI, not the candidate', 'crates/incant_render_game/src/lib.rs',
     [('let ui_input = ui_next.as_ref().map(|next| next.input());',
       'let ui_input = self.ui.as_ref().map(|ui| &ui.input);')], GAME),
    ('viewport relayout published without admission', 'crates/incant_render_game/src/lib.rs',
     [('        self.consumer\n            .admit_ui(next.input())\n            .map_err(RenderHostError::Consumer)?;\n', '')],
     GAME),
    ('unchanged paint reused despite changed widget state', 'crates/incant_render_game/src/ui.rs',
     [('node.focused == widget.focused', 'true || node.focused == widget.focused')], GAME),
    ('GPU consumer never submits the staged UI', 'crates/incant_render_gpu_host/src/lib.rs',
     [('        if let Some(stage) = ui {\n            live.ui\n                .submit(&self.renderer, stage, &frame.target())\n                .map_err(GpuHostError::Render)?;\n        }\n',
       '        drop(ui);\n')], HOSTED),
]


def leased():
    lease = json.loads((SCRATCH / 'lease.json').read_text(encoding='utf-8'))
    assert lease.get('packet') == '0056-native-game-ui-draw', 'no 0056 lease'
    assert not (SCRATCH / 'released').exists(), '0056 lease released'


def run(suite):
    package, target, gpu = suite
    env = dict(os.environ, CARGO_TARGET_DIR='/tmp/incant-agent-native-target')
    command = [str(ROOT / 'tools/cargo'), 'test', '--locked', '--offline', '-j2', '-p', package, *target]
    if gpu:
        command += ['--', '--include-ignored', '--test-threads=1']
    result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, timeout=3600)
    summary = [line for line in (result.stdout + result.stderr).splitlines()
               if line.startswith('test result') or ' FAILED' in line or line.startswith('error')]
    return result.returncode, summary


def main():
    leased()
    status = subprocess.run(['git', 'status', '--porcelain', '--', 'crates'], cwd=ROOT,
                            capture_output=True, text=True, check=True).stdout
    assert status == '', 'commit source changes before mutating'
    log = open(sys.argv[1], 'w', encoding='utf-8')
    caught_all = True
    for name, path, edits, suite in MUTATIONS:
        file = ROOT / path
        original = file.read_bytes()
        digest = hashlib.sha256(original).hexdigest()
        text = original.decode('utf-8')
        for old, new in edits:
            assert text.count(old) == 1, (name, old, text.count(old))
            text = text.replace(old, new)
        file.write_text(text, encoding='utf-8')
        try:
            code, summary = run(suite)
        finally:
            file.write_bytes(original)
        assert hashlib.sha256(file.read_bytes()).hexdigest() == digest, f'{path} not restored'
        caught = code != 0
        caught_all &= caught
        log.write(f'{name}: {"CAUGHT" if caught else "NOT CAUGHT"} ({suite[0]} {" ".join(suite[1])})\n')
        for line in summary[:12]:
            log.write(f'    {line}\n')
        log.flush()
        print(name, 'CAUGHT' if caught else 'NOT CAUGHT', flush=True)
    clean = True
    for suite in (RENDER, GAME, HOSTED):
        code, summary = run(suite)
        clean &= code == 0
        log.write(f'unmutated {suite[0]} {" ".join(suite[1])}: {"PASS" if code == 0 else "FAIL"}\n')
        for line in summary:
            log.write(f'    {line}\n')
    print('unmutated', 'PASS' if clean else 'FAIL')
    sys.exit(0 if caught_all and clean else 1)


main()
