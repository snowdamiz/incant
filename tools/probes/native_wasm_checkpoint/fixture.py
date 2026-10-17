"""Fixed public input and independent arithmetic oracle; no executable guest input."""
import struct
import wave

ENTITY = '00000000000000000000000001'
AUDIO = '00000000000000000000000002'
TICKS, SPLIT, FRAMES = 40, 13, 800
SCHEMA = bytes([23]) * 32
EVENTS = [(2, 'ArrowRight', True), (4, 'Space', True), (5, 'Space', False),
          (7, 'ArrowRight', False), (16, 'ArrowRight', True), (17, 'Space', True),
          (18, 'Space', False), (21, 'ArrowRight', False), (29, 'ArrowRight', True),
          (31, 'Space', True), (32, 'Space', False), (34, 'ArrowRight', False)]


def state(tick):
    held, moves, score = set(), 0, 0
    for n in range(1, tick + 1):
        for at, key, down in EVENTS:
            if at == n:
                score += int(key == 'Space' and down and key not in held)
                held.add(key) if down else held.discard(key)
        moves += int('ArrowRight' in held)
    return tick, moves, score


def position(tick):
    count, moves, score = state(tick)
    return [moves / 16, score, count / 64]


def recording(start, ticks):
    return {'format': 'incant-input', 'version': 1, 'tick_rate': 60,
            'start_tick': 0, 'ticks': ticks,
            'frames': [{'tick': at - start, 'events': [{'type': 'key', 'code': key, 'down': down}]}
                       for at, key, down in EVENTS if start < at <= start + ticks]}


def observation(start, end):
    return {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
            'checks': [{'name': f'persistent-{tick}', 'tick': tick,
                        'expect': {'type': 'vector', 'entity': ENTITY, 'property': 'translation',
                                   'value': position(tick), 'tolerance': 0}}
                       for tick in range(start, end + 1)]}


def tone(path):
    # Integer triangle, with a non-dividing sample-rate/transport rate. No floats,
    # external media, random state, or generated executable content.
    samples = [((n % 64) if n % 64 < 32 else 63 - n % 64) * 512 - 7936
               for n in range(1024)]
    with wave.open(str(path), 'wb') as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(22050)
        output.writeframes(struct.pack('<1024h', *samples))
