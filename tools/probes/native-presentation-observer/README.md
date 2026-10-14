# Exact-drawable presentation observer probe

CPU-qualified private package: ten collector tests, strict host/HAL lint and
feature-off checking pass. No native window, GPU, listener or timestamp capture
has been executed for this package. It does not modify the
engine dependency graph or normal player. See
[design and qualification](../../../docs/spikes/native-presentation-observer.md).

`prepare.py` verifies the exact cached `wgpu-hal` 29.0.4 registry archive and
creates a separate standalone source package. Its manifest records every original
and patched HAL file. The three-file patch adds a default-off macOS observer;
three new Rust files implement its collector, native callback and CPU controls.
`host.rs` provides the callable bridge from an existing `wgpu::Surface`.

The generated package's `observer` feature enables the HAL seam. No feature is on
by default. Preparation does not compile, resolve Cargo dependencies or run a
window. It copies the engine lock as the starting resolution; the qualification
step must separately audit the resulting standalone lock before any build.

Never prepare into a registry, repository or existing destination. Do not add a
second `nextDrawable` call, retain a drawable in a callback, infer zero timestamps
from missing callbacks, or treat the collector's raw intervals as a whole-window
performance pass.
