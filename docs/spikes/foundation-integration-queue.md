# Foundation integration queue

PR76, PR79 and PR80 were merged on 2026-10-11 after all 21 applicable checks
passed on each exact head. The prospective merge tree and fetched merge tree
matched the tested tree; merge parents and `Built-by: astra` were verified.
The resulting main is `611d867bee1ff3c8f33da8c221c82867408e96c6`.

| Integration | Tested head | Merge |
| --- | --- | --- |
| [PR76](https://github.com/snowdamiz/incant/pull/76) | `ee2bc1e70acd02aadb5a470fb5d3610f7fe79901` | `8ed6523090c8685e43b8f9693c9511f8e9d07aa9` |
| [PR79](https://github.com/snowdamiz/incant/pull/79) | `74238785fe92c50daea1d8dd056492359def61f2` | `ecf61d49bf781e337d2318f2b8492cfe5a37c4e4` |
| [PR80](https://github.com/snowdamiz/incant/pull/80) | `b39d7d03bf2af6f139c4a2097f6c756f4f8064a2` | `611d867bee1ff3c8f33da8c221c82867408e96c6` |

PR76 preserves the exact reviewed heads of PR65, PR66, PR69, PR70, PR71,
PR72, PR74 and PR75. PR80 preserves PR63, PR68, PR73 and PR78. After actual
main ancestry was verified, GitHub marked some source PRs merged and the
remaining included PRs were closed with the exact ancestry evidence. Original
authored commits were preserved. Canceled superseded runs were not counted as
passing checks; the complete cumulative matrices provide the integration
coverage.

The [machine-readable record](evidence/foundation-merge-queue-2026-10-11.json)
retains exact heads, merge parents/trees/messages, check identities/results and
source-PR retirement states. It also records the hashes of the local verification
records from which this checkpoint was assembled.

Immediately after PR80, only PR77 remained open. Its old published source is
held for the reviewed correction and actual-main integration. The next ready publication combines that corrected static-FBX import with
the renderer prewarming/flipbook change before one integrated validation and
hosted matrix. It must be based on actual main and pass its final applicable
checks. The standing limit remains two independent ready integration PRs. One coordinator writes main; unfinished
services, TypeScript/RPC and shipping tooling remain local until capacity.

This is a queue and evidence checkpoint. Phase 1 remains active; complete
engine/game, reference-phone, performance and human-owned gates remain open.
