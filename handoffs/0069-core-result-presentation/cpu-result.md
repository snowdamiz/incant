# 0069 CPU verification — 2026-10-13

Source correction commit: `15a59060c06ea249c66740b5940b92721d032970`, direct child of Claude's `0379cfc2b765fe1e7efeb6d53b55cc9749f54282`. The final build source is clean, generation 5 `acb68a621b4b74acd4c4e14280171451de81f53d2feaa4f8585a9fc6d12d1f1d`. These packet files are excluded handoff additions; root may retain them separately without relabeling the generation of any binary. Root's brief and `root-helper-review.json` are untouched.

CPU correctness checks passed. Appearance, clipping/readability approval and the ignored GPU cases remain for Claude. This lane created no GPU device, frame, screenshot, native window or device-audio output. PCM below refers to bytes produced by the offline mixer.

## Corrections

- Regenerated the existing approved Core Sample SC subset mechanically using the unchanged recipe, pinned Noto source and fonttools 4.60.1 wheel. The old strings reproduce the old subset byte-for-byte. Two runs with the current authored strings produce the same new 13,764-byte font, SHA-256 `9ec258961c8800c709f05586b6fca8a47061f3f776ac088bb87fbb2a462c116e`, covering the authored 按 and 或. Font family, text, styling and layout remain Claude's.
- Added explicit CPU-only lease admission and its no-launch regression; holder identity comes from the trusted grant. Startup omits HOME instead of substituting another directory. The adapter import changed only by rustfmt.
- Adopted exactly the two Python guard/test files from accepted commit `c08c8b6790c1d1a4503f6883cff88447c3ad4cee`, allowing only the repository's pinned Parry artifact paths and matching manifests. No other exporter flags or engine behavior changed.

## Checks and execution

- 133 Rust CPU tests passed, zero failures and no ignored test executed: 122 text/UI/CPU-consumer cases, ten renderer UI arithmetic cases, and one reference-mutation test. The initial `ui_tests::` filter matched zero and is explicitly not coverage; `ui::tests::` subsequently ran ten.
- 63 focused Python cases passed: 21 lifecycle/lease, 20 sample binding/playtest and 22 physics artifact audit tests. Five affected packages passed all-target Clippy and scoped formatting; each of the four built adapters passed Clippy; final adapter rustfmt and diff checks passed.
- Four locales were freshly authored through public commands and validation (`en`, `ar`, `zh-Hans`, `en-XA`). Native binding checks covered public Undo/Redo/reopen, typed refusals, locale/readback and cue isolation. The separate test observer is absent from the production export. Native Play's explicit restart refusal remains recorded; packaged replacement ownership is tested separately.
- One successful fresh ordinary headless shipping export, `shipping-002`, passed ten fresh processes: win, loss, locked exit, idle, win tail, short loss and four restart routes. Logical observations/logs and PCM match native Play, including exact PCM concatenation across restart. The successful exported player SHA-256 is `b5b837292654eca3268dc633e122cb221f1dbc318fd4a6a0701fb870576e3b3b`. Its own authoring tree, private build/cache and source maps were removed before the accepted runs. The real five-solver target features, selected normal graph and shipping codegen were audited; no document/cook/command-bus/Wasmtime runtime edge is admitted.
- Four separate CI-profile packages/adapters were compiled for layout verification; these are **not** shipping exports. All twelve locale × viewport (960×540, 640×480, 420×320) runs passed at 3,056 host steps. Each read five snapshots of all thirteen widgets. The optional prompt's parent, cleared won objective, exact restart restoration and zero missing clusters passed. Each report states `gpu_adapter: null`, zero rendered/written frames and no RGBA output. Every adapter's PCM and restart boundary match the ordinary shipping gamepad route exactly.
- Two GPU regression binaries were compiled only under generation 5 and copied outside the target. Their eight ignored GPU cases were not run. `regression/frozen-build-g5.json` binds source, manifest and executable hashes; the binaries are byte-identical to the earlier generation-2 compile because the engine code is unchanged. This does not relabel their earlier receipt.

## Provenance and retained failures

Generation 1 binds the initial CPU helper correction. Generation 2 adds deterministic font regeneration and the CPU unit/build checks. Generation 3 adds adapter import formatting and the public native proof. Generation 4 adds the accepted Parry observer correction and successful shipping proof. Generation 5 commits the identical final source and binds the CI locale packages, layout runs and final GPU-test compile receipts. The final read-only audit compared all 16,977 frozen tracked files against the live final source; the source and helper identities of earlier generations remain retained.

The first shipping attempt's Cargo compilation returned zero, but the older observer rejected the local Parry package IDs. Exporter cleanup removed its temporary compiler output, so that attempt is **not** a packaged proof. The original redirected compiler streams were not retained before removal; its command/exit receipts and the explicitly post-terminal diagnostic summary survive. The corrected rerun is a fresh build, retains both redirected streams, and passed. The initial formatting refusal and zero-test filter are retained too.

The CPU lease was released at `2026-10-13T07:59:43.157279+00:00`. All 70 lease-managed child groups were reaped; target-user and owned-process lists were empty. Both foreground shipping runs and all foreground tool handles were terminal/reaped. No frame debit occurred. `/tmp/incant-agent-font-target` is returned to root as a warm cache; frozen binaries remain under `~/.codex/incant-evidence/0069/frozen-binaries/generation-5`. The old CPU lease is irrevocably released and refuses further launches. A later GPU handoff needs root's new authority; this packet does not grant it.

## Retained packet and read-only re-audit

`cpu-evidence/manifest.json` binds 558 lossless records, 315 unique payloads and 564 existing external binary/asset/PCM files. `artifacts.json.gz` retains the textual bytes; large external files remain in durable `~/.codex/incant-evidence/0069`, with size and SHA-256 recorded. `verification.json` is a separate read-only rehash of the saved archive, every retained/original record and every external file. `observation-verification.json` independently recomputes the source/package/report/PCM assertions without starting a runtime.

Canonical read-only commands from this checkout:

```sh
python3 -B handoffs/0069-core-result-presentation/cpu-evidence/verify.py
python3 -B /Users/andreyyurlov/.codex/incant-evidence/0069/audit-cpu.py
```

The archive contains exact historical commands, raw outputs, five freeze generations, lease/grant/release, source inventories, compiler outputs, font reproduction, package inventories and all numerical observations. Do not rerun build/capture commands under the released CPU lease. No current-main integration, phone/performance, physical-device or phase-gate claim is made.
