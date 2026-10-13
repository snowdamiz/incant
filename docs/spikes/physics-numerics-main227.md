# Numerical physics admission on main227

The accepted default-off pinned-kernel numerical preflight is composed on actual
main227 (`d9d9c4e`). This checks a joined inert primitive world without constructing
or installing a live native owner. `PhysicsUnsupported` remains unchanged.

The current composition was actually rebuilt from `56a63757` using an exclusively
owned target. Deterministic and fast SIMD8 suites each pass 214 tests in 23 nonempty
groups, with zero failures or ignored cases. Each mode observes 72 completed
primitive steps in each dimension, including exact derived mass and unchanged
saved bits. These observations do not execute a decoded restoration.

All fifteen checks pass: both test modes and strict snapshot Clippy, explicit
selection of both vendored Rapier libraries and the trusted native test for strict
Clippy, wasm32, the std-only normal snapshot graph, runtime dependency guard, four
independent probe-lock resolutions and actual default runtime graph, 31 Python
guards, workspace/vendor formatting, conventions and diff checks. The initial
scoped target clean removes only this lane's snapshot/Rapier package products.
All 3,788 frozen files remain unchanged through execution. No GPU was run.

All 1,606 selected physics/vendor/root input bytes match immutable producer
`7afb2b8b`, except the previously accepted current-main ordering of three `Error`
variants; their definitions and codecs are unchanged. The root lock adds only
Rapier2d/3d optional local edges to the std-only snapshot crate. All 968 package
identities remain unchanged. Normal runtime dependencies exclude the optional
admission feature and snapshot crate; no runtime or player enables it.

The [producer proof](physics-native-admission.md) and its independent final review
are retained unchanged. That proof includes five historical fixture/lint invocation
failures and their separately successful corrections. Current integration checks
all succeeded; old producer binaries were not substituted for current builds.

The [manifest](evidence/physics-numerics-main227.json),
[read-only verifier](evidence/physics-numerics-main227/verify.py), source inventory,
lossless raw logs and four retained producer/current executable identities bind
these claims. Current-main independent final review is pending.

This preflight does not establish complete solver arithmetic/work bounds, capture
phase authentication, compound/hook admission, native construction, decoded field
installation, runtime owner/event identity or live physics saves. The cumulative
logical-work counter is not a CPU-time or full allocation budget. No device,
performance or phase gate is claimed.
