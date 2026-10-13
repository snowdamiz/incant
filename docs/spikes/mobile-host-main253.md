# Retained mobile host on main253

The current composition adds the reviewed default-off `mobile-host` Rust owner
to main253's persisted physics/session/player APIs. The mobile owner still
exposes no checkpoint restore, haptics or platform surface API. Its seven
lifecycle controls use the actual cooked game, native render-input consumer and
offline mixer; they do not run a GPU or device audio.

The producer's three mobile implementation/test files are byte-exact. The player
manifest and library join only add the optional feature, module and existing
development dependency; main253's native-checkpoint feature, bounded file sizes
and save preflight are preserved. The lock adds one edge to the existing
`incant_render_data` package, with no package identity or version change. An
explicit unchanged-tree ancestry merge preserves the accepted producer objects.

Fresh qualification binds all 3,750 implementation/build/tool inputs at
`3e257acdb7b8173dad70f63675ab39e04cd2fd97`. The mobile-host plus mobile-ffi library
passes 31 tests. With native checkpoints enabled, deterministic and fast8 player
suites each pass 34 cases across six harnesses, with zero failures or ignored
cases. Each mode retains its immediate standalone player and test executables;
the separate-process saved physics and fractional PCM controls use that matching
CLI. These counts overlap and are not summed. Fifteen exact binaries are retained.

All 26 commands pass: three builds, thirteen test harnesses, four strict all-target
Clippy configurations, an iOS library check, and five dependency/manifest/convention/
format/diff guards. The iOS result is compilation only. The target was released
after every child terminated. The producer's seven focused controls within 28
library cases and nine checks remain attributed to their original source, with
the earlier misspelled evidence-driver argument preserved.

The portable verifier checks archive payloads, current and tested source hashes,
the exact additive composition, Cargo artifact targets/features, matching CLI
selection, actual test summaries and producer provenance. Optional flags also
rehash original files and immediate executable copies. Independent current-main
final review is pending; publication requires the exact-head PR fast gate.

Concrete platform surfaces, scheduling, touch/IME callbacks, real-time audio and
physical-device measurements remain separate work. Caller output and external
GPU side effects cannot be rolled back after a late consumer failure. No mobile
appearance, device execution, shipping performance or phase gate is claimed.

[Manifest](evidence/mobile-host-main253/manifest.json) and
[verifier](evidence/mobile-host-main253/verify.py).
