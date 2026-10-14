# Native-window texture workflow

The actual Mac editor passed manual texture import and binding, source-watcher
reimport, Undo, Redo, malformed-source refusal and recovery while retaining the
same paused Play world and script module state. This exercises the real native
handlers, command journal, watcher and surface presentation path. Import/history
GUI clicks, texture appearance, performance and phase approval are not claimed.
The separate [numerical Metal control](native-texture-refresh.md) remains the
pixel evidence.

Root executed window004 on Mac17,9 / macOS 27.0.1 with Apple M5 Pro Metal. The
unchanged source006 app was normally activated through Finder Open while its
owned PID 7779 was live. It passed and all six native children were reaped. The
paused tick stayed at 3 across all catalog stages, then advanced to 4. Its persistent
module counter produced translation 12 and next delta 8, detecting a reset module.
The final independent RPC reopen recovered journal revision 6 and the blue recipe;
unrelated checkpoint/script/source bytes stayed unchanged.

| Native stage | Revision | Accepted surface presentations |
| --- | ---: | ---: |
| Manual import/bind red | 3 | 9 |
| Watched blue source | 4 | 146 |
| Undo red | 5 | 151 |
| Redo blue | 6 | 247 |
| Malformed source retains blue | 6 | 296 |
| Identical blue source recovery | 6 | 326 |

These counters record successful native surface presentation calls, not a pixel
review. The private read-only `play-probe` observer checked world/catalog identity;
mutations used existing handlers. Stale generations refused and Stop invalidated
the old world. The watcher made one import-origin transaction and preserved Undo.

The executed source is `897912be9b51a1856bfd804cb90bc487ae696643`, based on
main284 `9db293541ea7bd0ba03089a395c934c3bf2e2264`. The copied editor SHA256 is
`d7383ce27acb0d90e2872f6e8a1caa9b2b0780e86222df2ca581f5d8c620a4f3`.
A minimal local `.app` identity wrapper retained those executable bytes. The
headless binary was produced at source003 (`ff8c4365…`); the genuine frontend was
produced at source002. Their subsequent build inputs stayed equal; they retain their producing labels. Main285 `1eca995cb36d9f9cada732dce7d52447569e63ac`
adds only STATUS and four navigation-outcome docs/evidence paths to main284;
that source-relevance join requires no claim of a new execution or broad rebuild.

Rust 1.99, SDK 27.0, locked/offline CI arm64 builds used two Cargo/Rayon workers and
an explicit retained target. Final source006 editor build, 20 scoped Play CPU
controls and strict scoped Clippy passed. The unchanged Python controls passed 8
at source004; root separately confirmed 8 on the final source. Genuine frontend
build and bridge verification passed at source002. These scoped checks are distinct from the actual window result.

Retained failures: source001 was unexecuted and its constant sentinel was
strengthened; source002 compilation refused iterator `.len()`; source004 lint
refused test-module placement. Both were repaired in successors. Window001
timed out at 120s because its dispatcher omitted `--texture-probe`; its original
streams remain unavailable. Window002 missed the 20s initial live predicate
without status. Source006 diagnostics showed window003 rendered 1193 frames but
held tick 0 because native focus was false. Window004 changed no source, deadline
or eligibility guard; prompt normal Finder activation resolved startup.

After success, CUA reopened default no-args PID 7870. Its SurfaceSpike screenshot
and the earlier default 3050 screenshot are excluded; root closed both. Neither
is fixture or appearance evidence.

The app child bound remains 120s and the overall runner uses 180s cooperative
checkpoints plus mandatory owned cleanup. Blocking filesystem work and cleanup
can finish later; this is not a hard wall-clock termination guarantee. The supervisor imported the unchanged runner in-process, emitted its owned live
PID and captured bounded output without a nested outer runner group.

Portable replay starts no engine process:

```sh
python3 docs/spikes/evidence/native-window-texture-workflow/verify.py
python3 docs/spikes/evidence/native-window-texture-workflow/verify.py --originals --binaries --source-root .
```

The [manifest](evidence/native-window-texture-workflow/manifest.json) binds raw
outcomes, commands, source freezes, failures, CUA exclusions and external artifact
hashes. No executable/frontend is duplicated. `--source-root` checks the seven
reviewed paths; it does not assert whole-runtime equality.
