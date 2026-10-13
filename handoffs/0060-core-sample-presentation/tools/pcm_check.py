#!/usr/bin/env python3
"""pcm_check.py LOOP_WAV RUN_PCM RATE OUT_JSON: compare a player's mixed PCM with the music loop.

RUN_PCM is the raw stereo f32-le output the packaged player writes (`--pcm-output`) at RATE.
Reports per-second RMS, the peak, whether left and right differ, and the normalized
correlation of the left channel with the loop's left channel (box-filtered and decimated to
RATE) once 1 s into the run and once 30 s in, after the 22.857 s loop has wrapped. This is
mixer output, not audible-device playback. Standard library only.
"""
import json
import math
import struct
import sys
import wave


def ncc(a, b):
    sa = math.sqrt(sum(x * x for x in a))
    sb = math.sqrt(sum(x * x for x in b))
    return sum(x * y for x, y in zip(a, b)) / (sa * sb)


def main():
    loop_path, pcm_path, rate, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    with wave.open(loop_path) as w:
        channels, source_rate = w.getnchannels(), w.getframerate()
        frames = struct.unpack(f'<{w.getnframes() * channels}h', w.readframes(w.getnframes()))
    step = source_rate // rate
    left = [frames[i] / 32768 for i in range(0, len(frames), channels)]
    loop = [sum(left[i:i + step]) / step for i in range(0, len(left) - step + 1, step)]
    raw = open(pcm_path, 'rb').read()
    count = len(raw) // 8
    pcm = struct.unpack(f'<{count * 2}f', raw[:count * 8])
    run_left, run_right = pcm[0::2], pcm[1::2]
    rms = []
    for s in range(count // rate):
        seg = pcm[s * rate * 2:(s + 1) * rate * 2]
        rms.append(round(20 * math.log10(math.sqrt(sum(v * v for v in seg) / len(seg))), 1))
    window = rate // 4
    checks = []
    for seconds, lags in ((1.0, range(0, rate // 10)), (30.0, range(0, rate // 10))):
        start = int(seconds * rate)
        if start + window > count:
            continue
        segment = run_left[start:start + window]
        best = max((ncc(segment, [loop[(start - lag + k) % len(loop)] for k in range(window)]), lag)
                   for lag in lags)
        checks.append({'at_seconds': seconds, 'window_samples': window, 'correlation': round(best[0], 4),
                       'lag_samples': best[1]})
    report = {'pcm': pcm_path, 'rate': rate, 'frames': count, 'seconds': round(count / rate, 3),
              'peak': round(max(abs(v) for v in pcm), 4), 'rms_dbfs_per_second': rms,
              'stereo_differs': any(abs(a - b) > 1e-6 for a, b in zip(run_left, run_right)),
              'loop': loop_path, 'loop_rate': source_rate, 'loop_seconds': round(len(left) / source_rate, 4),
              'correlation_with_loop_left': checks}
    with open(out, 'w', encoding='utf-8') as f:
        json.dump(report, f, indent=1)
    print(json.dumps({k: report[k] for k in ('seconds', 'peak', 'stereo_differs', 'correlation_with_loop_left')}))


if __name__ == '__main__':
    main()
