`tone-440hz.ogg` is an Incant-generated 0.5-second, 48 kHz stereo (identical channels) sine tone.
It contains no sampled third-party material. The source is 24,000 signed 16-bit
samples: `round(12000 * sin(2*pi*440*i/48000))`. It was encoded locally using
FFmpeg native Vorbis (experimental) quality 5, expanded to two identical channels, with input metadata removed. Tests require the checked-in
fixture, not an FFmpeg installation. It is test data, never played to a device.
