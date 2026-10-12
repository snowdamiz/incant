#!/usr/bin/env python3
"""Assemble the committed 0058 evidence from the scratch directory (read only there).

Usage: python3 -I -B evidence.py

Reads ~/.codex/incant-evidence/0058 (ledger, run exits and native reports,
captures, release record) and the 0057 raw captures used as the "before"
reference, and writes only handoffs/0058-native-text-retirement/{evidence,
screenshots}. It starts no process, posts no event, takes no capture and
writes nothing into the scratch directory, so it may run after the release.
Per-run steps below are the exact plans in tools/plans; "observed" entries
cite what the ordinary player, macOS or the window captures showed,
"inferred" entries are source reasoning.
"""
import hashlib
import json
from pathlib import Path
import runpy
import shutil
import tempfile

HERE = Path(__file__).resolve().parent
PACKET = HERE.parent
ROOT = PACKET.parents[1]
SCRATCH = Path.home() / '.codex/incant-evidence/0058'
BEFORE = Path.home() / '.codex/incant-evidence/0057/run-2/caps'
png = runpy.run_path(str(HERE / 'pngtool.py'))
EVIDENCE, SHOTS = PACKET / 'evidence', PACKET / 'screenshots'
# UI rows of a 1x capture (window 640 x 512 px incl. the 32 px title bar). Row 32 holds the macOS
# title-bar hairline; the crate never reaches above y 210 at slot 1, lifted or resting.
UI = (0, 33, 330, 176)
NAME_ROW = (0, 120, 320, 34)
TOP_ROWS = (0, 33, 330, 87)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')


def diff(a, b, region, offset_b=(0, 0)):
    x, y, w, h = region
    ia, ib = png['read'](a), png['read'](b)
    count, worst, box = 0, 0, None
    for yy in range(h):
        for xx in range(w):
            pa = ia['px'][y + yy][x + xx]
            pb = ib['px'][y + offset_b[1] + yy][x + offset_b[0] + xx]
            d = max(abs(i - j) for i, j in zip(pa, pb))
            worst = max(worst, d)
            if d:
                count += 1
                box = [xx, yy, xx, yy] if box is None else [min(box[0], xx), min(box[1], yy),
                                                              max(box[2], xx), max(box[3], yy)]
    return {'region_xywh': list(region), 'offset_b': list(offset_b), 'pixels': w * h, 'differing': count,
            'max_delta': worst, 'bbox': box}


RUNS = {
    'r1-latin': {
        'purpose': 'repeated Latin edit, field switches and blur/refocus, close with a field focused',
        'bursts': ['p1a', 'p1b', 'p1c'],
        'steps': [
            ['p1a', 'Tab x4 -> Name, End', 'capture p1-01: caret after "Ada Lovelace"'],
            ['p1a', 'keys " Byron", Shift+Left x5', 'capture p1-02: "Byron" selected'],
            ['p1a', 'Right; Tab -> Arabic (R1); End; key b', 'b committed into Arabic'],
            ['p1a', 'Tab -> CJK (R2); End; key c', 'c committed into CJK'],
            ['p1a', 'Shift-Tab -> Arabic (R3); End; key d', 'd committed into Arabic'],
            ['p1a', 'Shift-Tab -> Name (R4); End; key e', 'e committed into Name'],
            ['p1a', 'activate previous app: window blur with Name focused (R5)', 'process alive 25 s later'],
            ['p1b', 'raise; Tab x4 -> Name; End; key f; blur (R6)', 'f in Name'],
            ['p1b', 'refocus; Tab x5 -> Arabic; End; key g; blur (R7)', 'g in Arabic: UI focus was not restored'],
            ['p1b', 'refocus; Tab x6 -> CJK; End; key h; blur (R8)', 'h in CJK; process alive 21 s later'],
            ['p1c', 'raise; Tab x4 -> Name; End', 'capture p1-03: all three fields'],
            ['p1c', 'AX close button with Name focused (R9 at loop exit)', 'exit 0, report ui.focus = Name'],
        ],
        'retirements': {'field_switch': 4, 'blur_with_field_focused': 4, 'close_with_field_focused': 1},
        'final_fields_observed': {'Name': 'Ada Lovelace Byronef', 'Arabic': 'سلام + bdg (rendered bdg|سلام, RTL)',
                                  'CJK': '中文ch'},
    },
    'r2-deadkey': {
        'purpose': 'live U.S. dead-key composition (Option-E) retired by blur, pointer focus change and close',
        'bursts': ['p2a', 'p2b'],
        'steps': [
            ['p2a', 'Tab x4 -> Name; End; keys " Byron"; Option-E', 'capture p2-01: live marked "´", underlined, IME caret'],
            ['p2a', 'blur while marked (R1, composition active)', 'process alive 38 s later'],
            ['p2b', 'raise; Tab x5 -> Arabic; End; key e', 'plain e in Arabic: no dead-key carry-over'],
            ['p2b', 'Shift-Tab -> Name (R2); End; Option-E', 'marked text in Name (not captured)'],
            ['p2b', 'pointer click into CJK while marked (R3, composition active)', 'CJK focused; crate not lifted'],
            ['p2b', 'End; key e', 'plain e in CJK: no "´" or "é" from the retired Name context'],
            ['p2b', 'Shift-Tab -> Arabic (R4); Shift-Tab -> Name (R5); End; Option-E; key e', 'é committed into Name'],
            ['p2b', 'capture', 'capture p2-02: all three fields'],
            ['p2b', 'Option-E; AX close while marked (R6 at loop exit, composition active)', 'exit 0, report ui.focus = Name'],
        ],
        'retirements': {'field_switch': 3, 'pointer_field_switch_while_composing': 1,
                        'blur_while_composing': 1, 'close_while_composing': 1},
        'final_fields_observed': {'Name': 'Ada Lovelace Byroné', 'Arabic': 'سلام + e (rendered e|سلام, RTL)',
                                  'CJK': '中文e'},
        'click_submit_note': ('The click focused CJK but did not submit it, so the crate stayed down. Inferred from '
                              'source: focus loss cancelled the Name preedit, the display text changed, the layout '
                              'generation advanced and UiInput cancelled the captured press, suppressing the release. '
                              'A plain click into an unfocused field (0057) still submits; that limitation is unchanged.'),
    },
    'r3-unicode': {
        'purpose': 'Unicode-string RTL/CJK commits, field switches, blur and close with CJK focused',
        'bursts': ['p3a', 'p3b'],
        'steps': [
            ['p3a', 'Tab x5 -> Arabic; End; unicode " مرحبا"', 'Arabic commits'],
            ['p3a', 'Tab -> CJK (R1); End; unicode "日本語"', 'CJK commits'],
            ['p3a', 'blur with CJK focused (R2)', 'process alive 20 s later'],
            ['p3b', 'raise; Tab x5 -> Arabic; End; unicode " نعم"', 'Arabic commits'],
            ['p3b', 'Tab -> CJK (R3); End; unicode "一"; Shift+Left x2', 'capture p3-01: "語一" selected'],
            ['p3b', 'Shift-Tab -> Arabic (R4); Tab -> CJK (R5)', 'no text typed'],
            ['p3b', 'AX close with CJK focused (R6 at loop exit)', 'exit 0, report ui.focus = CJK'],
        ],
        'retirements': {'field_switch': 4, 'blur_with_field_focused': 1, 'close_with_field_focused': 1},
        'final_fields_observed': {'Name': 'Ada Lovelace (unchanged)', 'Arabic': 'سلام مرحبا نعم',
                                  'CJK': '中文日本語一'},
    },
}


def main():
    package = load(SCRATCH / 'pkg-1/package.json')
    fixture = load(SCRATCH / 'fixture-1/fixture.json')
    headless = load(SCRATCH / 'headless-1/headless.json')
    lease = load(SCRATCH / 'lease.json')
    caps = {p.stem: p for p in sorted(SCRATCH.glob('runs/*/caps/*.png'))}

    runs = {}
    for name, plan in RUNS.items():
        exit_record = load(SCRATCH / 'runs' / name / 'exit.json')
        runs[name] = {'pid': exit_record['pid'], **plan, 'exit': {k: exit_record[k] for k in (
            'exit_status', 'signal', 'ended_at', 'native_report_printed', 'new_crash_reports')},
            'native_report': exit_record['report'],
            'captures': {stem: sha(path) for stem, path in caps.items() if path.parts[-3] == name}}
        dump(EVIDENCE / 'runs' / name / 'exit.json', exit_record)
        shutil.copyfile(SCRATCH / 'runs' / name / 'player.log', EVIDENCE / 'runs' / name / 'player-log.txt')
    total = sum(sum(r['retirements'].values()) for r in runs.values())

    failed = load(SCRATCH / 'runs/p1-latin/exit.json')
    heap_text = (SCRATCH / 'runs/p1-latin/heap-00-baseline.txt').read_text(encoding='utf-8', errors='replace')
    heap_rows = [line for line in heap_text.splitlines()
                 if 'TextInputContext' in line or 'ManagedClient' in line or 'TaoView' in line]
    dump(EVIDENCE / 'runs/p1-latin/exit.json', failed)
    shutil.copyfile(SCRATCH / 'runs/p1-latin/player.log', EVIDENCE / 'runs/p1-latin/player-log.txt')
    shutil.copyfile(SCRATCH / 'runs/p1-latin/unified-log-excerpt.txt', EVIDENCE / 'runs/p1-latin/unified-log-excerpt.txt')
    dump(EVIDENCE / 'runs/p1-latin/heap-census-excerpt.json', {
        'tool': '/usr/bin/heap <pid> (suspends the task while it scans malloc zones)',
        'full_listing_sha256': sha(SCRATCH / 'runs/p1-latin/heap-00-baseline.txt'),
        'header': heap_text.splitlines()[:12], 'rows': heap_rows})

    dump(EVIDENCE / 'native-runs.json', {
        'format': 'incant-0058-native-runs',
        'player': 'pkg-1/player/incant_game_player --package data --window (ordinary composed player, exec-launched '
                  'child of tools/launch.sh, which waits and records the real exit status)',
        'event_mechanisms': {
            'keys': 'CGEvent keyDown/keyUp, US virtual key codes, kCGHIDEventTap; Shift/Option as real flagsChanged',
            'live_composition': 'U.S. layout dead key Option-E then E: macOS marked text then commit (not synthetic)',
            'unicode_text': 'CGEvent key events carrying a Unicode string, committed through the managed '
                            'NSTextInputContext; not an Arabic layout or CJK input method',
            'pointer': 'CGEvent mouseMoved/leftMouseDown/leftMouseUp at kCGHIDEventTap (one click, r2)',
            'blur_refocus': 'System Events app activation of the previous front app, then of the player',
            'close': 'AX close button press (CloseRequested), so the player prints its report',
            'capture': 'screencapture -o -x -l <player window id>, window only',
            'input_sources_enabled': ['com.apple.keylayout.US (current)', 'com.apple.CharacterPaletteIM',
                                      'com.apple.PressAndHold']},
        'guards': 'every launch/burst waited for an unlocked awake display and 20 s HID idle (our own events '
                  'count, so the wait is conservative); every event rechecked that the player was frontmost, '
                  'the session unlocked and the pointer unmoved; the previous front app and pointer were restored',
        'runs': runs,
        'retirements_total': total,
        'retirement_definition': ('one event that ends the active native text session of a focused field: a focus '
                                  'change between fields, a window blur, or close with the field focused. Triggers '
                                  'and their consequences are observed; that each one retired and dropped exactly one '
                                  'managed client is inferred from view.rs set_text_input_session (one managed_text '
                                  'per view, old.retire() then drop) and window/ime.rs synchronize.'),
        'stale_text_events': 'discarded_stale_text_events was 0 in all three reports. It counts only stale-session '
                             'TextInput events that reached the active player; callbacks to a retired client are '
                             'dropped inside the client and are not observable from outside.',
        'probe_failure_not_counted': {
            'run': 'p1-latin', 'pid': failed['pid'], 'exit_status': failed['exit_status'],
            'stderr': 'Error: Failed', 'crash_reports': failed['new_crash_reports'], 'input_sent': False,
            'observed': ['heap census started 16:28:24Z while the player sat in the background with no text '
                         'session (0 managed clients, 1 NSTextInputContext)',
                         'the process exited 2 s later with status 1, no native report, no crash report',
                         'unified log at resume: CoreAudio HALC_ProxyIOContext::IOWorkLoop "skipping cycle due to '
                         'overload", window finishing close'],
            'inferred': ['the ~1.7 s task suspension broke a real-time audio/device check and ended the session',
                         'window/mod.rs stores every loop error in `failure`; after the first error drops the live '
                         'session, the end-of-callback text-session sync calls game.host()? and overwrites it with '
                         'GameError::Failed, so stderr shows only "Failed"'],
            'action': 'retained; heap.sh retired; no live process was suspended again'},
    })

    comparisons = []
    for mine, before, region, note in [
            ('p1-01-name-caret', '11-name-click', UI, 'caret at the end of "Ada Lovelace", focus ring'),
            ('p1-02-name-selection', '13-name-selection', UI, '"Byron" selected'),
            ('p2-01-name-deadkey-preedit', '14-deadkey-preedit', UI, 'live marked "´": underline and IME caret'),
            ('p2-02-after-composition-retirements', '15-deadkey-commit', NAME_ROW, 'committed "é", caret; Name row'),
            ('p2-02-after-composition-retirements', '15-deadkey-commit', TOP_ROWS, 'title, body and buttons')]:
        comparisons.append({'state': note, 'native_0058': caps[mine].name, 'native_0058_sha256': sha(caps[mine]),
                            'before_0057': before + '.png', 'before_0057_sha256': sha(BEFORE / (before + '.png')),
                            **diff(caps[mine], BEFORE / (before + '.png'), region)})
    controls = [
        {'control': 'caret vs selection (0058 p1-01 vs p1-02)',
         **diff(caps['p1-01-name-caret'], caps['p1-02-name-selection'], UI)},
        {'control': '0058 caret vs 0057 caret shifted 1 px right',
         **diff(caps['p1-01-name-caret'], BEFORE / '11-name-click.png', (0, 33, 329, 176), (1, 0))},
        {'control': 'Arabic and CJK rows of p2-02 vs 0057 15 (0058 has an extra committed e in each)',
         **diff(caps['p2-02-after-composition-retirements'], BEFORE / '15-deadkey-commit.png', (0, 154, 330, 55))},
    ]
    dump(EVIDENCE / 'pixel-comparisons.json', {
        'format': 'incant-0058-pixel-comparisons',
        'method': 'stored code values of window captures in the same Display profile (1x external display, '
                  '640 x 512 px), tolerance 0',
        'before_source': '0057 run 2 (pid 17610), ~/.codex/incant-evidence/0057/run-2/caps, read only; that build '
                         'used the old NSView client and also carried the separate 0057 zero-coverage shader '
                         'candidate, which 0057 measured as pixel-neutral on Metal; this build carries neither',
        'comparisons': comparisons, 'controls': controls})

    dump(EVIDENCE / 'identities.json', {
        'format': 'incant-0058-identities',
        'brief_sha256_applied': lease['brief_sha256'],
        'acp_record': '~/.codex/incant-evidence/0058/acp.log (root): "Claude session verified: Opus 5.5; thinking: max"',
        'source': {'head': package['source_head'], 'tree': package['source_tree'],
                   'fix_commit': 'f830f49621bf163f266e8bc4ba4cebb9cbb20bc4',
                   'text_input_rs_sha256': sha(ROOT / 'third_party/tao/src/platform_impl/macos/text_input.rs'),
                   'text_input_rs_matches_spike_fixed_hash': sha(ROOT / 'third_party/tao/src/platform_impl/macos/'
                                                                 'text_input.rs') == load(ROOT / 'docs/spikes/evidence/'
                                                                 'macos-text-retirement.json')['source_hashes'][
                                                                 'third_party/tao/src/platform_impl/macos/text_input.rs'],
                   'worktree_clean_outside_handoff_at_build': package['worktree_clean_outside_handoff'],
                   'not_merged': '0057 shader/reference candidate e2551dd2 and caret accessor ca111ccf are absent'},
        'toolchain': {'rustc': package['rustc'], 'cargo': package['cargo'],
                      'rust_toolchain_toml_sha256': sha(ROOT / 'rust-toolchain.toml'),
                      'cargo_lock_sha256': package['cargo_lock_sha256']},
        'cargo_target_dir': package['cargo_target_dir'],
        'binaries': {
            'incant_headless': {'sha256': headless['sha256'], 'bytes': headless['bytes'], 'profile': 'ci',
                                'command': ' '.join(headless['command'][1:])},
            'incant_game_player': {'sha256': package['player_sha256'], 'bytes': package['player_bytes'],
                                   'features': package['features'], 'profile': 'ci',
                                   'command': ' '.join(package['command'][1:]),
                                   'note': 'ordinary player binary; frozen copy in pkg-1/player'},
            'winctl': {'source_sha256': sha(HERE / 'winctl.swift'), 'binary_sha256': sha(SCRATCH / 'winctl')}},
        'fixture': {k: fixture[k] for k in ('project_sha256', 'script_sha256', 'font_sha256', 'revision', 'scene',
                                            'command_bus_only', 'undo_redo', 'journal_reopened', 'binary_sha256')},
        'fixture_ids': {k: fixture['ids'][k] for k in ('name', 'arabic', 'cjk', 'camera')},
        'package': {'data_bytes': package['data']['bytes'], 'data_files': package['data']['files'],
                    'camera': package['camera'], 'scene_id': package['scene_id'],
                    'startup_check': {'route': package['startup_route'],
                                      'ui': package['startup']['render_input']['ui']}},
        'host': {'os': 'macOS 27.0.1 (26A434)', 'machine': 'Mac17,9', 'cpu': 'Apple M5 Pro',
                 'gpu_backend': 'Metal (wgpu), adapter "Apple M5 Pro"',
                 'displays': ['built-in 1512x982 pt at 2x (main)', 'external 3440x1440 at 1x, x -3440..0'],
                 'player_window': 'opened on the external display at (-2040,-158), 640x512 pt, scale 1, in every run'},
        'lease': lease,
    })

    SHOTS.mkdir(exist_ok=True)
    for stem, name in [('p1-01-name-caret', 'native-1x-r1-caret.png'),
                       ('p1-02-name-selection', 'native-1x-r1-selection.png'),
                       ('p1-03-after-retirements', 'native-1x-r1-after-nine-retirements.png'),
                       ('p2-01-name-deadkey-preedit', 'native-1x-r2-deadkey-preedit.png'),
                       ('p2-02-after-composition-retirements', 'native-1x-r2-after-composition-retirements.png'),
                       ('p3-01-rtl-cjk-selection', 'native-1x-r3-rtl-cjk-selection.png')]:
        shutil.copyfile(caps[stem], SHOTS / name)
    # Crop intermediates live in a temporary folder inside the handoff, never in the released scratch.
    scratch = Path(tempfile.mkdtemp(prefix='.review-', dir=PACKET))
    crops = []
    for stem in ('p1-03-after-retirements', 'p2-02-after-composition-retirements', 'p3-01-rtl-cjk-selection'):
        out = scratch / f'{stem}-fields-3x.png'
        png['main'](['crop', str(caps[stem]), str(out), '0', '122', '320', '90', '3'])
        crops.append(str(out))
    final = SHOTS / 'native-1x-fields-after-retirements-r1-r2-r3-3x.png'
    final.unlink(missing_ok=True)
    png['main'](['stack', str(final), *crops])
    rows = []
    for stem in ('p1-01-name-caret', 'p1-02-name-selection', 'p2-01-name-deadkey-preedit',
                 'p2-02-after-composition-retirements'):
        out = scratch / f'{stem}-name-4x.png'
        png['main'](['crop', str(caps[stem]), str(out), '60', '122', '256', '32', '4'])
        rows.append(str(out))
    final = SHOTS / 'native-1x-name-caret-selection-preedit-commit-4x.png'
    final.unlink(missing_ok=True)
    png['main'](['stack', str(final), *rows])
    shutil.rmtree(scratch)

    log = []
    for name in ('processes.log', 'bursts.log', 'images.log'):
        log.append(f'== {name}\n' + (SCRATCH / name).read_text(encoding='utf-8'))
    (EVIDENCE / 'processes.txt').write_text('\n'.join(log), encoding='utf-8')
    if (SCRATCH / 'release.json').exists():
        shutil.copyfile(SCRATCH / 'release.json', EVIDENCE / 'release.json')
    for name, src in (('fixture.json', SCRATCH / 'fixture-1/fixture.json'),
                      ('package.json', SCRATCH / 'pkg-1/package.json'),
                      ('headless.json', SCRATCH / 'headless-1/headless.json')):
        shutil.copyfile(src, EVIDENCE / name)
    print(json.dumps({'retirements_total': total, 'comparisons': [c['differing'] for c in comparisons],
                      'controls': [c['differing'] for c in controls]}))


if __name__ == '__main__':
    main()
