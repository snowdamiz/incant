# Project-owned cooked fonts

Backend/CLI asset imports now accept `.ttf` and `.otf` sources and register `kind:
"font"` assets through the existing prepared `incant_cmd` transaction. Reimport
keeps the source's stable ULID and user name. Undo/Redo and journal reopen retain
the earlier cooked fingerprints. The source watcher uses the same cooker and
32 MiB source-read cap, including its existing debounce, stale-result protection,
diagnostic recovery and policy that an unchanged source does not overwrite Undo.
Explicit mixed import batches still fail atomically before document publication;
watch cycles retain their existing policy of reporting broken sources while
committing unrelated successful reimports together.

`incant_assets::cook_font` preserves the exact supplied bytes. Supported inputs
are one loadable sfnt face, as checked by the existing pinned rustybuzz 0.14.1
parser; TTC collections and malformed faces fail explicitly. No family/style
metadata is inferred or decoded. The borrowed `incant_text::validate_font`
admission helper uses the same size/header/parser policy as engine construction,
which continues to cache every actual cosmic-text font before shaping.

Default caches use checked project-owned `.incant/cache/fonts` directories.
Each recipe fingerprint hashes the versioned cook/parser salt and the existing
source dependency path/content hash. A small JSON manifest (at most 8 KiB) contains
only the version, fingerprint, dependency, byte count and content hash. Unknown
fields, invalid keys, nonportable dependency paths and inconsistent recipes fail.
The content-addressed `.ifont` blob contains:

- Eight-byte magic `INCTFNT\0`.
- Little-endian u32 format version 1 and raw payload length.
- Thirty-two raw SHA-256 bytes for the original font payload.
- Untouched font bytes, at most 32 MiB, with no trailing envelope bytes.

The runtime reader checks ordinary file types, rejects file symlinks, bounds all
reads, checks exact length/header/hash and validates the actual shaping face
before exposing immutable bytes. Content files and manifests are staged and
renamed; they are never modified beneath an already published reader. Failed
runtime batches retain prior versions and generation state. Hashes are integrity
checks, not authentication of external font sources or a font-license guarantee.

`incant_assets_runtime` exposes this reader under its optional `font` feature.
Its default audio-only graph remains unchanged, and the font-only graph has no
authoring or random-ID-generation dependencies. `CookedFont::into_source` moves
bytes into a `FontSource` for an independent `TextEngine`; retained asset-store
versions provide borrowed bytes for callers choosing an explicit copy. Runtime
readers and shaping never need the source files or host font discovery. The
authoring store also accounts for the font payload in its existing staged budget.

The affected CPU suites pass 124 tests, including nine new font tests. Separate
font-only and default-reader suites each pass three tests; all-target Clippy,
standalone wasm32 font-reader compilation, consumer-library compilation, lock and
dependency checks pass. The public round trip imports the existing unchanged OFL
Latin/Arabic/Hebrew fixtures, commits one agent-provenance transaction, performs
Undo/Redo and persistent reopen, deletes all original fonts, reloads cooked data
and actually shapes multilingual text. Corruption, unsupported collections,
oversize sparse files, cache repair, retained versions, stable reimport, watcher
recovery, stale prepared results and source/cache symlink escapes are covered.
[Machine-readable evidence](evidence/packaged-font-assets.json) records inputs and
exact commands/log identities.

This is the font asset/shaping data path. It does not add font atlas creation,
rendering, text widgets/IME, binary scene text bindings or automatic inclusion of
font references in shipped game archives. Fixtures exercise TrueType outline
data under both accepted filename extensions; no CFF-specific numerical result is
claimed. No GPU, window, browser, provider or device execution occurred, and these
CPU/compile checks do not complete the game UI or reference-device gates.

The current editor import form still excludes TTF/OTF in its format allowlist.
That UI entry point remains a separate integration; this slice does not claim it.

## Main107 integration

Independent source review accepted the bounded CPU asset path. The union keeps
the existing skeletal modules, visual glam feature and animated-model refusal
while adding font registration. All 957 lock records retain their versions; only
five internal dependency edges are added. Integrated tests, Clippy, font-only/
audio-only readers, wasm32, renderer/cook/game-runtime consumer compilation and
dependency checks pass. [Review](evidence/packaged-font-assets-independent-review.json)
and [integrated evidence](evidence/packaged-font-assets-main107.json).
