# Corrected Arabic Core export and CPU replay

A fresh ordinary Arabic shipping package from
`069bbe54dc529511138867dfd4fbe50507a38b6b` now passes public export and all three
same-package CPU routes. This source includes the logical 640×480 initial window
and the [Arabic health-label correction](core-rtl-health-direction.md). No native
window ran for this new package. Arabic appearance, pip reading order and the
[combined locale gate](../gates/phase1-review.md) remain open.

The producing source is main342, preserved as a clean checkout. The documentation
is prepared against main346; later integration changes do not relabel the binary.
The [earlier locale results](core-locale-native-window.md), including old Arabic
pixels, CJK's missed win image, pseudo's partial replay and narrow-layout gap,
remain historical.

## Public authoring and export

A cached current-source headless CLI build produced `8efebd8c…` with actual
`default,physics-deterministic` features. Public Core authoring imported the
retained art, committed through the command bus, exercised history/save/reopen,
and resolved Arabic using the engine localization/font path. The receipt records
revision 32, 46 RPC calls and valid complete authoring. All four health labels
contain actual U+200F marks around the expected three pip characters. These are
source/shaping checks, not visual reading-order acceptance. The 110 authored
files and unchanged 900/2,156/3,056-step recordings were frozen before export.

The unmodified public exporter used Rust 1.99.0, Node 22.23.3, npm 10.9.9 and
macOS SDK 27.0. Its fresh private target was removed before the copied player's
inherent offline startup check: **one gameplay tick and 800 PCM sample frames**,
source parsing false and GPU submitted false. The window-capable package was
never invoked with `--window`. It uses ordinary WGSL, deterministic physics and
no profiling, diagnostic, compilation/presentation observer or cooked-shader
selection. The actual target/runtime script flags are `audio,physics-deterministic,ui`;
the separate build-host script compiler is recorded separately. The normal
runtime graph excludes authoring, command-bus and agent packages.

Player SHA-256:
`6e7994630c36cde871a0dea8f2818e93ad2aa9e459d6a2fd848474b71d6835c7`.
The runtime tree has the executable plus 28 data files. Symbols and compiler
evidence remain outside that runtime tree. All 14 managed export calls returned
zero with independently observed successful cleanup. Private scratch peaked at
4,129,972,255 bytes in sampled observations; minimum sampled free space was
69,070,462,976 bytes. These are resource observations, not performance results.

## Same-package CPU result

| Route | Host steps | Final simulation tick | PCM bytes | Result |
| --- | ---: | ---: | ---: | --- |
| Loss | 900 | 900 | 5,760,000 | Pass |
| Win | 2,156 | 2,156 | 13,798,400 | Pass |
| Loss, restart, win | 3,056 | 2,156, generation two | 19,558,400 | Pass |

The unchanged public verifier checked complete observation membership and
outcomes, pickups, damage/result state, HUD parenting, all localized text sources,
13 logical widgets, finite PCM, hashes and exact loss + fresh-win concatenation.
The winning source label is U+200F + `● ● ○` + U+200F. All three owned CPU groups
were reaped and independently absent. These results establish logical HUD/state
and PCM parity, without pixels, audible-quality or FPS acceptance.

Two initial data-preparation assertions were corrected without native execution:
the retained art contains 30 resources **plus** its manifest, and JSON decoding
produces real U+200F rather than a literal escape string. A harmless supervisor
test initially used macOS's symlinked temporary-directory parent; its fixture was
resolved before testing the no-symlink rule. A post-export summary initially mixed
build-host/compiler and target/runtime units; exact output-path classification
corrected only that summary. Original refusals/logs are retained. No exporter or
player retry was used to obtain the passing results.

## Native appearance still pending

The latest separate pseudo replay remains a failed 120-second attempt: zero host
steps, 2,344 surface deferrals and a blank client while macOS reported both
`IOConsoleLocked=true` and `CGSSessionScreenIsLocked=true`. Its owned player and the
used capture helper were stopped and independently absent. The separate CJK
visual-tail window was not launched after that failure. This environmental
limitation does not relabel prior focus interruptions or supply locale pixels.

The archived dormant Arabic plan binds this new player and successful CPU
baseline, a unique local app copy, the original 3,056-step route and unchanged
120-second child/180-second cooperative bounds. It requires a separate matching
root grant and a fresh **metadata-only** macOS lock check before any native child.
Both lock flags must be present and false; locked, missing, malformed or conflicting
values refuse. Five pure parser controls pass. No new window, CUA or capture ran.
After unlocking, review normal Arabic HUD, damaged pip direction, loss/restart
and win while the independently owned child is live. Pseudo's full/narrow route
and CJK's final win pixels remain separate work.

## Replay

The [compact packet](evidence/core-arabic-corrected-export/manifest.json) retains
small reports, exact source/tool inventories, commands and failure receipts.
Large compiler logs, executables, cooked data and PCM remain external by hash.
Default replay starts no engine, compiler, GPU or UI:

```sh
python3 docs/spikes/evidence/core-arabic-corrected-export/verify.py
```

`--originals` rehashes retained small originals and large logs. `--binaries`
rehashes runtime/headless/PCM artifacts and recomputes PCM finiteness and
concatenation. `--git-source 069bbe54dc529511138867dfd4fbe50507a38b6b --source-root
PATH` checks the 102 producing Git inputs using a local repository. Omitting
`--git-source` with `--source-root` also checks live producing bytes; later main
may correctly refuse that stronger assertion. No locale, renderer or phase gate
is marked passed by this packet.
