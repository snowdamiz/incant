# Corrected bloom: actual GPU verification and visual review

Use Claude Opus 5.5 with thinking explicitly Max, verified through ACP before this prompt. You own visual review. Astra owns numerical implementation. A separate Claude0070 session continues SSAO design; this packet executes the already-built bloom correction without waiting for that design.

## Scope and frozen inputs

Root accepted output-space bloom correction5d02ad6c. A prior combined SSAO/bloom run had26/27 numeric gates pass; bloom DRS centroid failed. The correction preserves original settings/thresholds and fixes coordinate mapping. Nothing is accepted by this briefing.

The immutable execution checkout is /Users/andreyyurlov/.codex/worktrees/native-fbx-import/incant at f91266b94bb3c7e8dcf8ad30e4becec9cd957e2a, whose engine equals reviewed d2d2294e. Do not edit or commit that checkout. It holds the existing, reviewed handoffs/0070-ambient-occlusion/tools runner and analyzer; run their ABSOLUTE paths so ROOT remains the immutable checkout. Your own child worktree holds this new packet's report only. Root authorizes existing guarded tools to write their external evidence namespace; all your authored files belong to handoffs/0075-bloom-validation.

Read only needed context: the immutable checkout's corrected-bloom-preparation.md and corrected-gpu-result.md; root build review /Users/andreyyurlov/.codex/incant-reviews/0070-bloom004-build-root-20261013.json SHAfc5207779e4915f115734a07b4d10d4bb9df55d749bb05d6686a9f31a9af36ff; build/build.json in the evidence below. The source freeze, five binaries and default feature identity were verified. No Cargo/build/target use is authorized. Do not use your own engine source to infer what was built.

Evidence E=/Users/andreyyurlov/.codex/incant-evidence/0070-effects-build-004. Grant E/gpu-grant.json, SHA256 5716de8327acad53940e0db8b1e886ee54c80bd10b06f3e4766b60645dacb877. Build record794464ff8cf8bfaa3b9fdede86377e6f158d67be288f98cbf19fb2f6a3648424. Helper digest5ec4955232b6e7b41856fa300a3b2776a5ab8b2dc487178d61853196c429cd48; planfa6067e518204e8cd5b9fdae7223ec7e1eb4981d3dce2e5d38c9a5078729a028. GPU is exclusively assigned after0074 release10:36:19. Its release is pinned by the grant. Use the global guarded lock, never bypass it.

## Exact bounded execution

Use /opt/homebrew/bin/python3 -I -B and absolute tools/runner.py path from the immutable checkout. Each command has --grant E/gpu-grant.json BEFORE its optional final case name. Serial, foreground, once:
1. runner.py list --grant ... lib
2. runner.py suite --grant ... lib (37 actual GPU tests;445 submissions=425 existing+20 new)
3. runner.py case --grant ... ssao_core (20)
4. runner.py case --grant ... ssao_materials (10)
5. runner.py case --grant ... ssao_geometry (17)
6. runner.py case --grant ... lifecycle_drs (18)
7. runner.py case --grant ... bloom_numeric (8)
8. runner.py appearance --grant ... (16frames,16PNGs)
Exactly8GPUchildren,534submissions,16PNGs,max960x540. No model_gpu or host suites; original unaffected results remain prior-source evidence, not fresh execution. Existing runner is the smallest supported unit; do not edit it to create a new framework. Actual failing child ends device work with stop latched; no retry. Always runner.py release --grant ... and verify all children gone.

Run analyzer only AFTER appearance collection and GPU release, so known historic gate failures cannot hide visual evidence. Command absolute tools/analyze.py E, retain exact exit and analysis.json; never loosen old gates. Personally inspect16paired PNGs, original physical bloom centroid checks and visual halo placement/footprint at100/75/50%. SSAO's known uncorrected quality defects remain explicit; this packet does not repair or accept them. Bloom is being considered as an independent merge scope. No new captures, art tuning, radius/bias changes, private memory or background tasks.

Return result.md with actual adapter, model/effort, source/binary/grant identities, current versus reused checks, counts, failures and separate numeric/appearance verdicts for bloom. Copy only bounded original evidence/reports/screenshots needed to retain review, hash exact copies, leave external originals unchanged. Commit scoped packet after GPU release with Built-by: claude, no push or shared refs. Root handles integration and phase gates.
