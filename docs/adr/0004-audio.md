# ADR 0004: Kira with a thin engine mixer

Date: 2026-10-08. Updated: 2026-10-10. Status: decided by director instruction
in the PLAN.md revision 5 decision record. This does not approve the Phase 0
gate or claim later-phase work is implemented.

## Decision

Retain Kira and an engine-owned bus abstraction. Persistent document references describe clips and buses; runtime handles do not appear in documents or agent tools.

## Evidence and implementation boundary

Phase 1 now has WAV/OGG Vorbis cooking, typed AudioSource/AudioBus/AudioListener
components and a Kira 0.12.5 mixer. Shared commands drive isolated gameplay audio;
the headless runner exports bounded stereo float WAV without opening a device.
Native streaming uses Kira's decoder worker. Offline streaming uses two validated
PCM chunks per voice and Kira's public Hermite interpolator so fast exports do not
depend on worker scheduling. Both paths use Kira routing and spatialization.

Sample tests and the public TypeScript/import/RPC/export probe pass; see
[audio runtime evidence](../spikes/audio-runtime.md). This establishes cooked
playback and script control, not production audio completion. Native app/device
wiring, browser resource streaming, platform latency, interruptions and hardware
measurements remain open. Offline export is not evidence of device playback.

## Consequences and revisit trigger

Spatialization, interruption handling, device changes and mobile audio sessions require target tests. FMOD is an explicit optional adapter for a later studio requirement, subject to licensing review.

Source: [PLAN.md](../../PLAN.md), sections 2, 3, 5 and 6.
