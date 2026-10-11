# Shared native Play startup

The headless CLI and retained RPC now obtain their cooked scene, compiled
behavior, input actions, clip bindings and render settings from `incant_play`.
This authoring-only crate extracts the existing startup boundary so the native
editor can use the same implementation. It does not add editor Play controls.

`PreparedPlay::load` validates scene/camera selection and sample rate, cooks
attached TypeScript through the existing fixed compiler and source-recovery
guard, checks referenced caches/clips and an optional behavior digest, and
returns owned runtime inputs. A scene without attached scripts uses a fallback
supplied by the trusted host. The headless host passes its existing embedded
no-op behavior. The native session receives no project document, source text or
command bus. Render-cache paths remain inputs for the consuming render host;
this is not a claim that every asset has been loaded at preparation time.

The shipping graph and actual target-codegen audits explicitly reject
`incant_play`. The new library is a development authoring boundary, not a
dependency of the shipped player or a per-tick authoring path.

## Validation

- 51 ordinary Rust tests passed across `incant_play` and `incant_headless`.
  Four new cases run actual native simulation after the authoring document and
  temporary source root are deleted, transfer prepared inputs to another thread,
  compile attached TypeScript without invoking the fallback, and check refusal
  ordering and the requested behavior digest. The initial untyped test source
  was rejected by the strict compiler and corrected to use `NativeBehavior`.
- Both existing headless GPU tests passed separately. These are regression
  checks, not editor rendered-pixel acceptance.
- Scoped all-target Clippy, the native headless build, formatting, 13 Python
  package/dependency checks and the actual player graph with `project` and
  `native-window` features passed. Removing an obsolete private ID helper fixed
  the initial Clippy dead-code failure.
- The public source transaction probe made 32 RPC requests in two fresh native
  processes. Authored motion, edits, Undo/Redo and journal recovery produced X
  positions 2, 4, 2, 4 and 2 after twenty ticks each, without changing authoring
  state through Play. All source writes used the command bus.
- Independent Astra review compared the extracted factory with the previous
  startup body and accepted all ten production/test identities. Exact source,
  binary, log and review hashes are retained in the
  [evidence receipt](evidence/shared-native-play-startup.json).

Native editor controls, viewport/input integration, device playback acceptance,
cross-platform execution and the remaining engine gates stay open. This change
does not establish phone performance or a complete gameplay session save.
