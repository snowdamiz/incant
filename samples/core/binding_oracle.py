"""Numerical assertions for the authored mesa; no rendering or styling decisions."""
import copy
import json
import math
import struct
from pathlib import Path

import bindings
import playtest
from art import ids

HERE = Path(__file__).resolve().parent
RATE = 48000
FRAME_BYTES = 8
TICK_BYTES = RATE // 60 * FRAME_BYTES
PROBE_SCRIPT = ids.make('script', 98)
PROBE_ENTITY = ids.entity(998)
MUTE_KEYS = {'pickup': 'KeyP', 'damage': 'KeyH', 'won': 'KeyV', 'lost': 'KeyL'}

# This is a separate public test attachment. It never replaces a gameplay API or
# edits the production controller. Unused raw keys invoke real stopAudio controls
# solely for paired PCM subtraction tests, in a project that is never exported.
PROBE_SOURCE = """import type { NativeBehavior, NativeInput } from '@incant/runtime/native';
const last = new Map<string,string>();
const mute: [NativeInput['keyboard']['held'][number],string][] = %s;
export default { update(context) {
  for (const w of context.hud.widgets) if (w.text !== undefined && last.get(w.id) !== w.text) {
    context.log('proof.hud ' + JSON.stringify([w.id,w.text])); last.set(w.id,w.text);
  }
  for (const [key,source] of mute) if (context.input.keyboard.held.includes(key))
    context.commands.stopAudio(source);
} } satisfies NativeBehavior;
""" % json.dumps([[key, bindings.AUDIO[role]] for role, key in MUTE_KEYS.items()])


def timeline(frames, ticks):
    grouped = {}
    for row in frames:
        grouped.setdefault(row['tick'], []).extend(row['events'])
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60, 'start_tick': 0,
            'ticks': ticks, 'frames': [{'tick': t, 'events': events} for t, events in sorted(grouped.items())]}


def cases():
    win = json.loads((HERE / 'routes/win-input.json').read_text())
    return {'win': win, 'lose': timeline([], 1500),
            'locked': json.loads((HERE / 'routes/locked-input.json').read_text()),
            'idle': timeline([], 30), 'win-tail': timeline(win['frames'], 2306)}


def plan(receipt, name, recording):
    value = playtest.observe(receipt, recording['ticks'], locked=name == 'locked')
    selected = bindings.validate_hud(receipt.get('hud', bindings.HUD))
    value['entities'] += [selected['won'], selected['lost'], *receipt['samples']]
    if 'prompt' in selected:
        value['entities'].append(selected['prompt'])
    value['entities'] = list(dict.fromkeys(value['entities']))
    if name == 'idle':
        value['checks'] += [{'name': 'initial-' + owner, 'tick': recording['ticks'],
            'expect': {'type': 'exists', 'entity': owner, 'value': True}} for owner in receipt['samples']]
    return value


def game_logs(report):
    return [row for row in report['logs']['entries'] if not row['message'].startswith('proof.hud ')]


def game_state(report):
    health, count, phase = 3, 0, 'playing'
    for row in game_logs(report):
        message = row['message']
        if message.startswith('core.damage '): health = int(message.rsplit('=', 1)[1])
        if message.startswith('core.pickup '): count += 1
        if message.startswith('core.won '): phase = 'won'
        if message.startswith('core.lost '): phase = 'lost'
    return health, count, phase


def labels(texts, health, count, phase='playing', selected=None):
    selected = bindings.validate_hud(bindings.HUD if selected is None else selected)
    result = {selected['health']: texts[f'health.{health}'],
              selected['collected']: texts[f'samples.{count}'],
              selected['objective']: '' if phase == 'won' else texts[
                  'hud.objective.exit' if count == 4 else 'hud.objective.collect'],
              selected['won']: texts['hud.won'], selected['lost']: texts['hud.lost'],
              ids.widget(8): texts['hud.controls']}
    if 'prompt' in selected:
        result[selected['prompt']] = texts['hud.prompt']
    return result


def verify(report, receipt, name):
    assert report['observations']['passed'] and report['logs']['dropped'] == 0
    assert all(row['passed'] for row in report['observations']['checks'])
    assert all(row['actual'] is row['expect']['value'] for row in report['observations']['checks']
               if row['expect']['type'] == 'exists')
    health, count, phase = game_state(report)
    expected = {'win': (2, 4, 'won'), 'win-tail': (2, 4, 'won'), 'lose': (0, 0, 'lost'),
                'locked': (2, 3, 'playing'), 'idle': (3, 0, 'playing')}[name]
    assert (health, count, phase) == expected, (name, health, count, phase)
    pickups = [row for row in game_logs(report) if row['message'].startswith('core.pickup ')]
    assert [row['message'].split()[1] for row in pickups] == receipt['samples'][:count]
    if name in ('win', 'win-tail'):
        assert [row['tick'] for row in pickups] == [127, 508, 1266, 1842]
        assert [row['tick'] for row in game_logs(report) if row['message'].startswith('core.won ')] == [2066]
    selected = bindings.validate_hud(receipt.get('hud', bindings.HUD))
    for role in ('won', 'lost', *(['prompt'] if 'prompt' in selected else [])):
        shown = phase != 'playing' if role == 'prompt' else role == phase
        parent = selected['result' if shown else 'inactive']
        assert report['observations']['entities'][selected[role]]['parent'] == parent
    if name == 'locked':
        assert playtest.locked_exit_checks(report, receipt, report['tick'])['passed']


def verify_hud(report, texts, selected=None):
    """Observe actual pre-command declarations, one tick after gameplay events."""
    health, count, phase = 3, 0, 'playing'
    previous = labels(texts, health, count, phase, selected)
    expected = [(1, owner, text) for owner, text in sorted(previous.items())]
    changes = {}
    for row in game_logs(report):
        message = row['message']
        if message.startswith('core.damage '): health = int(message.rsplit('=', 1)[1])
        elif message.startswith('core.pickup '): count += 1
        elif message.startswith('core.won '): phase = 'won'
        elif message.startswith('core.lost '): phase = 'lost'
        else: continue
        if row['tick'] < report['tick']:
            changes[row['tick'] + 1] = labels(texts, health, count, phase, selected)
    for tick, current in sorted(changes.items()):
        expected += [(tick, owner, text) for owner, text in sorted(current.items()) if previous[owner] != text]
        previous = current
    actual = [(row['tick'], *json.loads(row['message'][len('proof.hud '):]))
              for row in report['logs']['entries'] if row['message'].startswith('proof.hud ')]
    assert actual == expected, (actual, expected)


def verify_ui(report, texts, selected=None):
    """Exact declared label set and widget count; unknown widgets never disappear."""
    selected = bindings.validate_hud(bindings.HUD if selected is None else selected)
    ui = report['render_input']['ui']
    assert ui['widgets'] == len(bindings.widget_ids(selected))
    assert ui['glyphs'] > 0 and ui['focus'] is None
    assert ui['sources'] == labels(texts, *game_state(report), selected)


def pcm(path, report):
    data = path.read_bytes()
    assert len(data) == report['frames'] * FRAME_BYTES
    assert all(math.isfinite(v[0]) for v in struct.iter_unpack('<f', data))
    return data


def cue_difference(original, muted, report, role, seconds):
    """The isolated cue must affect every event window and nothing before/after.

    Two ticks allow mixer block drainage at start/end. The bound is documented
    here, independent of the controller's commands and actual cue PCM values.
    """
    assert len(original) == len(muted)
    prefix = {'pickup': 'core.pickup ', 'damage': 'core.damage ', 'won': 'core.won ', 'lost': 'core.lost '}[role]
    events = [row['tick'] for row in game_logs(report) if row['message'].startswith(prefix)]
    assert events, role
    windows = [((tick - 1) * TICK_BYTES, min(len(original), (tick + math.ceil(seconds * 60) + 2) * TICK_BYTES))
               for tick in events]
    prior = 0
    peaks = []
    for start, end in windows:
        assert original[prior:start] == muted[prior:start], (role, 'unexpected cue before event')
        a = struct.iter_unpack('<f', original[start:end]); b = struct.iter_unpack('<f', muted[start:end])
        peak = max(abs(x[0] - y[0]) for x, y in zip(a, b))
        assert math.isfinite(peak) and peak > 1e-5, (role, 'silent cue', peak)
        peaks.append(peak); prior = end
    assert original[prior:] == muted[prior:], (role, 'cue did not stop')
    return {'ticks': events, 'peak_differences': peaks, 'outside_windows_exact': True}


def restarted(one, two, *, pad=False, held=False):
    split = one['ticks']; frames = copy.deepcopy(one['frames'])
    def topology(rows):
        connected = set()
        for row in sorted(rows, key=lambda x: x['tick']):
            for event in row['events']:
                if event['type'] == 'gamepad_connected': connected.add(event['id'])
                elif event['type'] == 'gamepad_disconnected': connected.discard(event['id'])
        return connected
    connected = topology(frames)
    if pad and 0 not in connected:
        frames.append({'tick': 1, 'events': [{'type': 'gamepad_connected', 'id': 0}]})
        connected = topology(frames)
    button = {'type': 'gamepad_button', 'id': 0, 'button': 'start', 'value': 1} if pad else {
        'type': 'key', 'code': 'KeyR', 'down': True}
    frames.append({'tick': split, 'events': [button]})
    if held:
        frames.append({'tick': split - 1, 'events': [{'type': 'key', 'code': 'KeyD', 'down': True},
                                                  {'type': 'key', 'code': 'Space', 'down': True}]})
    else:
        frames.append({'tick': split + 1, 'events': [{**button, **({'value': 0} if pad else {'down': False})}]})
    # Session replacement preserves physical device topology. A new run's
    # recording must still connect a pad absent from the first run, while an
    # already connected pad must not receive a duplicate connection receipt.
    for row in two['frames']:
        events = []
        for event in row['events']:
            if event['type'] == 'gamepad_connected':
                if event['id'] in connected: continue
                connected.add(event['id'])
            elif event['type'] == 'gamepad_disconnected': connected.discard(event['id'])
            events.append(copy.deepcopy(event))
        frames.append({'tick': split + row['tick'], 'events': events})
    return timeline(frames, split + two['ticks'])


def restart_cases():
    # A full win is 2,156 ticks and loss occurs at 616. Retain the complete win
    # and a 900-tick loss with its result cue drained; both directions fit the
    # ordinary player's existing 3,600-host-tick admission. Two full wins do not.
    recordings = {**cases(), 'lose-short': timeline([], 900)}
    routes = [
        ('win-restart-lose', 'win', 'lose-short', False, False),
        ('lose-restart-win', 'lose-short', 'win', False, False),
        ('gamepad-restart-lose', 'win', 'lose-short', True, False),
        ('held-restart-idle', 'win', 'idle', False, True),
    ]
    assert all(recordings[a]['ticks'] + recordings[b]['ticks'] <= 3600
               for _, a, b, _, _ in routes)
    return recordings, routes


def restart_plan(receipt, first, second, one, two, *, held=False):
    a, b = plan(receipt, first, one), plan(receipt, second, two)
    for row in a['checks']: row['name'] = 'before-' + row['name']
    for row in b['checks']:
        row['name'] = 'after-' + row['name']; row['tick'] += one['ticks']
    b['checks'] = a['checks'] + b['checks']
    for tick in (1, one['ticks'] + 1):
        b['checks'].append({'name': f'fresh-pickup-{tick}', 'tick': tick,
            'expect': {'type': 'exists', 'entity': receipt['samples'][0], 'value': True}})
    if held:
        for action in ('move', 'jump', 'restart'):
            b['checks'].append({'name': 'held-neutral-' + action, 'tick': one['ticks'] + 1,
                'expect': {'type': 'action', 'name': action, 'active': False, 'pressed': False}})
    return b
