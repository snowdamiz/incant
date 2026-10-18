# Rust input views across the native Wasm hosts

This slice adds actual Rust guest test sources for existing input APIs. It adds
no import, ABI version, host permission or production input behavior. Eight
selected CPU tests and two scoped strict lints pass on the identified Mac
producer, including the real Rust guest on both Wasmtime and build-linked
wasm2c. Synthesized input is not hardware or native-window delivery evidence.

The Phase 1 SDK deliverables and section 12 decisions are in PLAN.md. The audit compared
`sdk/ts/src/native.d.ts`, `sdk/rust/src`, the generated ABI and both host dispatch
routes. Existing public command/query families already have Rust routes; adding
more wrappers would not close a demonstrated capability gap. The useful small
gap is typed input conformance: `input_parity.rs` executes WAT that hashes the
entire raw input frame, while the previous real Rust gameplay sample reads only
the named `toggle` action. Previous SDK snapshot tests use empty gamepad and
gesture collections.

## Method-level input coverage

| Rust API | Existing evidence/test source | Added actual Rust guest contract |
| --- | --- | --- |
| `Tick::input`, `InputSnapshot::parse` | Raw two-host WAT copy/length/epoch controls; SDK unknown/truncated/unaligned-layout tests | Full accepted snapshot consumed with initialized fixed scratch; ignored short-scratch failure rolls back |
| `actions`, `action` | Real Rust gameplay sample observes one button action | Every named row and lookup agree; missing lookup is absent; button, axis1, axis2 and gesture actions/edges match the native frame |
| `keyboard`, `mouse` | Full raw-frame hash and native TS snapshot tests | Exact held/pressed/released masks, high key tag F12, optional position, motion, both wheel units and focus cancellation |
| `gamepads`, `Gamepad::value` | Raw-frame hash; prior Rust view tests use empty collections | All 17 values, raw slots, presence mask, both sticks and button edges; explicit zero differs from absence |
| `connected`, `disconnected` | Raw-frame hash | Sorted nonempty IDs including `u32::MAX`; disconnect/reconnect discards prior values/edges |
| `touches`, `touch_changes` | Raw-frame hash | Position/delta and ordered Down/Move/Up/Cancel rows; high IDs, focus cancellation without End, ignored unfocused movement and same-ID reuse |
| `gestures` | Raw-frame hash; prior Rust view tests use empty collections | Nonempty Tap, LongPress, Swipe and Pinch, with exact IDs/coordinates/duration/direction/scale/rotation |

The new `rust_input` test projects the independent native `InputFrame` into
typed expected rows. The guest logs decoded SDK fields, never raw ABI bytes or
a checksum of them. Floating values use hexadecimal bit patterns, retaining
signed zero. Assertions additionally require that every gesture kind is reached,
all 17 gamepad values are present, cancellation occurs, and reused IDs are clean.
The existing TypeScript snapshot test independently compares its complete
public projection against that same native frame representation; the new test
does not claim to compile an additional TypeScript guest.

Three adversarial Rust variants stage numeric writes and a log first, then
ignore an input scratch failure, return callback failure, or use an expired
epoch. Both hosts must reject the complete tick and retain only the previously
committed prefix after development reload. Existing WAT missing-input,
out-of-bounds, stale-token and startup/namespace refusals remain unchanged.

## Other SDK families and remaining boundaries

The source audit found existing routes and focused actual-host test sources for
numeric IDs/transforms/velocities; structural spawn/despawn/parent/velocity;
flipbook and skeletal playback; graph/pose/IK/root motion; 2D/3D physics queries
and character movement; navigation path/grid/steering; audio transport/gains;
HUD/world text/disabled state; locale/message/number/date/report; haptic
capabilities/play/stop; timers, coroutines, initial instance properties,
persistence and restart. See the corresponding `crates/incant_wasm_dev/tests`
and `sdk/rust/samples` files. This is a source inventory, not a statement that
every argument combination or release target is qualified.

Larger gaps rank after this bounded conformance addition:

1. Multiple authored TypeScript source/instance callbacks versus one scene-wide
   Wasm module with an immutable initial-owner catalog. Dynamic attachment,
   mixed TS/Wasm and multiple module ownership need an explicit architecture
   and lifecycle contract; they are not missing forwarding methods.
2. Browser Wasm module-host parity needs a browser runtime and export/conformance
   route, beyond the current native Wasmtime/wasm2c pair.
3. TypeScript source/instance JSON persistence and Rust explicit module state
   have deliberately different ownership contracts. Arbitrary VM memory, Rust
   stacks and borrowed views must not become implicit checkpoint state.

The added helper `input_guests` prepares four fixed source-bound guests through
the existing pinned compiler and translation machinery. The qualification below
prepares those guests and five unchanged input WAT controls, without invoking
the broad shipping-Wasm suite.

Source preparation passed 29 Python controls (`test_wasm_input`,
`test_wasm_sdk`, `test_rust_guest`), including two new fixed-input preparation
controls. Rust formatting, generated ABI/conventions checks and `git diff
--check` also pass. Those preparation checks are separate from actual guest
execution below.

## Actual local qualification

Native producer `0d672757b8fb1c7108116097341d64470aee8b83`, based on main414
`04272288c08095cc0eb9bfcd88d3bae23324a994`, uses pinned Rust 1.99.0 on
`aarch64-apple-darwin`, the ordinary `ci` profile, locked/offline Cargo, jobs 2
and Rayon 2. Per-package overrides retain renderer opt2 and sha2 opt3. This is
not a shipping-profile or timing measurement. The exact commands, environment,
tools and source hashes are in the [portable packet](evidence/native-wasm-input.json).

| Stage | Actual result | Seconds |
| --- | --- | ---: |
| Build scoped Wasm cooker | Exit 0; independent executable copy retained | 2.458 |
| Prepare four Rust and five unchanged WAT guests | Exit 0; all nine validated/metered and translated | 2.197 |
| Ordinary SDK `snapshots` | 3 passed, 0 failed/ignored | 0.861 |
| `rust_input` and unchanged `input_parity`, `static-conformance` | 2 new plus 3 existing tests passed, 0 failed/ignored | 73.573 |
| Strict Clippy for the two selected host-test targets | Exit 0 | 11.072 |
| Strict ordinary SDK all-target Clippy | Exit 0 | 0.638 |

The managed episode completed in 94.052508 seconds. Target allocation peaked
and finished at 7,691,522,048 bytes under the existing 12 GiB cap; sampled owned
RSS peaked at 1,643,134,976 bytes. The prior 1,498.691294 seconds remain charged,
for a cumulative target account of 1,592.743802 seconds. Fifteen previous copies
were preserved and rehashed; four independent Cargo-reported executables add
68,072,256 bytes. All 19 protected copies total 466,462,456 bytes. No target
cleanup or dependency deletion occurred.

The launch adapter accidentally omitted the outer interactive tool session ID
from its response. Its exact outer tool terminal exit remains unconfirmed; no
rerun was performed to repair that receipt. The recorded six native-owner child
exits are all zero, the supervisor result is complete, and independent checks
confirm wrapper/group 34936 plus all 58 observed native identities/groups are
absent. The root reviewer independently rehashed all 12 raw streams and 19
copies, checked the exact eight test names and repeated the 59 absence checks.

Separately, root's integration producer `865ee031d198a41218f1a262dccd7c13da7f8358`
passes all ten local development checks in 79.515127 seconds, actual tool 91513
exit 0, with its wrapper and ten child groups absent. These checks cover the
main415 composition; they do not relabel the native producer above. Root source
and native review receipts, integration logs and both closeouts are retained in
the same inert portable packet.

No device, UI routing, audio-device, window, rendering, shipping source-deletion,
performance or full-SDK parity gate is claimed. No window, computer-use, capture
or physical-audio helper was launched by this episode.

## Data-only replay

The packet contains 48 UTF-8 records (2,141,782 expanded bytes, 351,542 gzip
bytes), including the exact source inventory, plan, preparation identities,
actual native logs/results, closeout, reviews and integration logs. It contains
no executable or compiler bytes. This bounded standard-library command verifies
the saved snapshot without executing archived content or starting native work:

```sh
python3 - <<'PY'
import gzip, hashlib, io, json
from pathlib import Path
root = Path('docs/spikes/evidence')
def bounded(path, cap):
    with path.open('rb') as stream: data = stream.read(cap + 1)
    assert len(data) <= cap
    return data
sha = lambda data: hashlib.sha256(data).hexdigest()
m = json.loads(bounded(root / 'native-wasm-input.json', 65536))
assert m['archive'] == 'native-wasm-input.json.gz'
packed = bounded(root / m['archive'], 524288)
assert (len(packed), sha(packed)) == (m['packed_bytes'], m['packed_sha256'])
with gzip.GzipFile(fileobj=io.BytesIO(packed), mode='rb') as stream:
    raw = stream.read(4194304 + 1)
assert len(raw) <= 4194304
assert (len(raw), sha(raw)) == (m['expanded_bytes'], m['expanded_sha256'])
p = json.loads(raw)
assert len(p['records']) == m['record_count'] == 48
for record in p['records'].values():
    data = record['utf8'].encode('utf-8')
    assert (len(data), sha(data)) == (record['bytes'], record['sha256'])
read = lambda name: json.loads(p['records'][name]['utf8'])
n, c, f = map(read, ['native/result.json', 'native/closeout.json', 'fast/result.json'])
assert n['status'] == 'complete' and len(n['stages']) == 6
assert all(s['exit'] == 0 for s in n['stages'])
assert sum(len(s.get('tests', [])) for s in n['stages']) == 8
assert c['all_absent'] and len(c['checks']) == 59
assert c['actual_outer_tool_exit'] is None
assert f['passed'] and len(f['stages']) == 10 and all(s['exit'] == 0 for s in f['stages'])
print('48 saved records verified; native and integration producer scopes remain separate')
PY
```
