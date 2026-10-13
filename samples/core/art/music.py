"""Original audio for the Core Sample, synthesized deterministically (stdlib only).

mesa_loop.wav  An 8-bar ambient loop in D Dorian at 84 BPM: a slow pad, a soft bass,
               a pentatonic pluck figure and light shaker. Echo tails are rendered
               circularly so the last bar flows into the first without a seam.
Event cues (mono): pickup, hurt, won, lost. They are authored as silent sources
for the gameplay binding to start; nothing here plays them.
Peak levels are normalized to -3 dBFS; the mix gain lives in AudioSource/AudioBus.
"""
import math
import struct
import wave

from . import noise as N

RATE = 48000
BPM = 84.0
BEAT = 60.0 / BPM
BARS = 8
LOOP = BARS * 4 * BEAT
A4 = 440.0


def freq(midi):
    return A4 * 2 ** ((midi - 69) / 12)


def _write(path, channels, frames):
    peak = max(1e-9, max(abs(v) for ch in frames for v in ch))
    gain = 10 ** (-3 / 20) / peak
    with wave.open(str(path), 'wb') as out:
        out.setnchannels(channels)
        out.setsampwidth(2)
        out.setframerate(RATE)
        data = bytearray()
        for i in range(len(frames[0])):
            for ch in frames:
                data += struct.pack('<h', max(-32767, min(32767, int(round(ch[i] * gain * 32767)))))
        out.writeframes(bytes(data))
    return {'frames': len(frames[0]), 'channels': channels, 'seconds': round(len(frames[0]) / RATE, 4)}


def _add(buf, start, samples, pan=0.0, circular=True):
    left, right = buf
    gl, gr = math.cos((pan + 1) * math.pi / 4), math.sin((pan + 1) * math.pi / 4)
    n = len(left)
    for i, v in enumerate(samples):
        j = start + i
        if circular:
            j %= n
        elif j >= n:
            break
        left[j] += v * gl
        right[j] += v * gr


def pluck(f, seconds, bright=1.0):
    count = int(seconds * RATE)
    out = []
    w = math.tau * f / RATE
    for i in range(count):
        t = i / RATE
        env = math.exp(-t * 5.5) * min(1.0, i / 120)
        v = math.sin(w * i) + 0.32 * bright * math.sin(2 * w * i) * math.exp(-t * 9) + \
            0.12 * bright * math.sin(3.01 * w * i) * math.exp(-t * 14)
        out.append(v * env)
    return out


def pad_voice(f, seconds, detune=0.0):
    count = int(seconds * RATE)
    out, phase1, phase2, lp = [], 0.0, 0.0, 0.0
    for i in range(count):
        t = i / RATE
        attack = min(1.0, t / 1.4)
        release = min(1.0, (seconds - t) / 1.2)
        phase1 += f * (1 + detune) / RATE
        phase2 += f * (1 - detune) / RATE
        tri = (2 * abs(2 * (phase1 % 1) - 1) - 1) + (2 * abs(2 * (phase2 % 1) - 1) - 1)
        lp += (tri - lp) * 0.06
        out.append(lp * attack * max(0.0, release) * 0.5)
    return out


def bass(f, seconds):
    count = int(seconds * RATE)
    w = math.tau * f / RATE
    return [math.sin(w * i) * min(1.0, i / 400) * math.exp(-i / RATE * 1.3) for i in range(count)]


def shaker(seconds, seed):
    count = int(seconds * RATE)
    return [(N.unit(seed, i) * 2 - 1) * math.exp(-i / RATE * 38) for i in range(count)]


def echo(buf, delay, feedback, mix):
    """Circular stereo ping-pong echo (wraps, so the loop has no seam)."""
    left, right = buf
    n, d = len(left), int(delay * RATE)
    wl, wr = [0.0] * n, [0.0] * n
    for _ in range(4):
        for i in range(n):
            j = (i + d) % n
            wl[j] += right[i] * feedback
            wr[j] += left[i] * feedback
        left_new = [l + mix * a for l, a in zip(left, wl)]
        right_new = [r + mix * b for r, b in zip(right, wr)]
        left, right = left_new, right_new
        wl, wr = [0.0] * n, [0.0] * n
        feedback *= 0.5
    return [left, right]


def mesa_loop():
    n = int(round(LOOP * RATE))
    buf = [[0.0] * n, [0.0] * n]
    # D Dorian: Dm9 | Bbmaj7 | C6/9 | Am7, two bars each.
    chords = [(50, [62, 65, 69, 72, 76]), (46, [58, 62, 65, 69, 74]), (48, [60, 64, 67, 69, 74]),
              (45, [57, 60, 64, 67, 72])]
    bar = 4 * BEAT
    for k, (root, notes) in enumerate(chords):
        start = int(k * 2 * bar * RATE)
        for m, note in enumerate(notes):
            _add(buf, start, [v * 0.16 for v in pad_voice(freq(note), 2 * bar + 1.0, 0.004 + 0.001 * m)],
                 pan=-0.5 + 0.25 * m)
        for half in range(4):
            _add(buf, start + int(half * 2 * BEAT * RATE), [v * 0.42 for v in bass(freq(root), 2 * BEAT)], 0.0)
    # Pentatonic pluck figure (D F G A C), sparse and syncopated, varied per bar.
    scale = [62, 65, 67, 69, 72, 74, 77, 79]
    pattern = [0, None, 2, 4, None, 3, None, 5, 1, None, 4, None, 6, None, 3, None]
    for b in range(BARS):
        for s, step in enumerate(pattern):
            if step is None or N.unit(91, b, s) < 0.18:
                continue
            note = scale[(step + (b % 4 == 3) * 1) % len(scale)]
            t = (b * 16 + s) * BEAT / 4
            _add(buf, int(t * RATE), [v * 0.22 for v in pluck(freq(note), 1.6)], pan=0.55 if s % 2 else -0.45)
    for b in range(BARS):
        for s in range(16):
            if s % 4 == 2 or (s % 8 == 7 and b % 2):
                _add(buf, int((b * 16 + s) * BEAT / 4 * RATE), [v * 0.035 for v in shaker(0.12, b * 31 + s)],
                     pan=0.3)
    return echo(buf, BEAT * 0.75, 0.32, 0.6)


def cue(notes, seconds, decay=4.0, noise_level=0.0, seed=0):
    count = int(seconds * RATE)
    out = [0.0] * count
    for offset, midi, level in notes:
        start = int(offset * RATE)
        for i, v in enumerate(pluck(freq(midi), seconds - offset, 1.2)):
            if start + i < count:
                out[start + i] += v * level
    if noise_level:
        for i in range(count):
            out[i] += (N.unit(seed, i) * 2 - 1) * noise_level * math.exp(-i / RATE * decay)
    return [out]


def hurt():
    count = int(0.36 * RATE)
    out, phase = [], 0.0
    for i in range(count):
        t = i / RATE
        f = 120 * math.exp(-t * 6) + 45
        phase += f / RATE
        thud = math.sin(math.tau * phase) * math.exp(-t * 11)
        crackle = (N.unit(77, i // 3) * 2 - 1) * 0.35 * math.exp(-t * 18)
        out.append(thud + crackle)
    return [out]


def build(out):
    files = {
        'mesa_loop.wav': (2, mesa_loop()),
        'cue_pickup.wav': (1, cue([(0.0, 81, 0.8), (0.09, 88, 0.7), (0.18, 93, 0.45)], 0.9)),
        'cue_hurt.wav': (1, hurt()),
        'cue_won.wav': (1, cue([(0.0, 62, 0.6), (0.14, 69, 0.6), (0.28, 74, 0.6), (0.42, 78, 0.7),
                                (0.56, 81, 0.8)], 1.8)),
        'cue_lost.wav': (1, cue([(0.0, 69, 0.6), (0.22, 65, 0.6), (0.44, 62, 0.7)], 1.5, 2.0, 0.08, 5)),
    }
    return {name: _write(out / name, channels, frames) for name, (channels, frames) in files.items()}
