# Release checkpoint 14

[PR133](https://github.com/snowdamiz/incant/pull/133) promoted selected main160
`d1ef35ee32b92e6756b528463ab092fdc79c7ef4` at **2026-10-12 20:49:50 UTC**.
All **23 required contexts** and eight comprehensive push workflows passed on
candidate `e9c0b288489217500fed8b17ba22fbc0e677fedd`. Release is now
`2444d81f2dbc72a0e487f361d4614d4476bdddb1`; its tree
`c0137be6889607ea302d6cd3312a5a9f78f2c938` equals both the tested candidate and
selected main. Both parents, unchanged branch protection and the merge's
`Built-by: astra` trailer were verified. Later main161+ features are excluded.
[Complete receipts, checks and raw-log hashes](evidence/release-checkpoint-14.json).

The general macOS job executed the repaired invisible AppKit probe successfully:
256 cycles, 768 native contexts, 256 stale-session refusals and active-owner
destruction. It submitted no GPU work, injected no input and displayed no window.
The original checkpoint13 late-event failure remains in the retained predecessor
receipt; its candidate was never promoted.

The numerical UI tests now have actual hosted adapter identities in their logs:

| Job | Selected adapter | Backend |
| --- | --- | --- |
| Windows desktop | Microsoft Basic Render Driver, driver 10.0.26100.33438 | DX12 |
| Linux desktop | llvmpipe LLVM 20.1.2, Mesa 25.2.8-0ubuntu0.24.04.4 | Vulkan |
| macOS general | Apple Paravirtual device | Metal |

These identify this successful hosted run and supersede the earlier unverified
Linux/Windows backend hypothesis for this run. They do not establish physical
GPU, reference-phone, appearance or performance gates. Shader/reference arithmetic
and all required checks were retained.

Windows desktop completed in 75m07s under its reviewed 120-minute allowance;
Linux desktop took 26m45s and general macOS 70m03s. These are individual job
execution durations, not a matched cache/profile speedup. The original timeout
and earlier failed checkpoints remain historical. A local GitHub CLI log-cache
error and subsequent ANSI-output refusal were resolved by downloading the exact
job logs directly; no hosted job was rerun for that retrieval.

This promotion updates the comprehensively tested integration baseline. It does
not approve a phase gate, signing/distribution release, device result or completion
of the engine/game. The next batch remains governed by ADR0014's periodic frozen
candidate policy; feature PRs continue independently on main.
