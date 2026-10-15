# Renderer CI catalog selection repair

Release checkpoint42, candidate `6225d2f7`, failed its Linux Vulkan cache
lifecycle in [job116248141970](https://github.com/snowdamiz/incant/actions/runs/38681918072/job/116248141970).
The harness enabled every optional effect, but its command selected cooked
output without the complete bloom family. `enable_bloom` correctly refused it.
The same command also selected cooked materials, which deliberately refuse the
currently uncooked SSAO implementation; adding bloom alone would leave that
second mismatch.

The fresh-process cache lifecycle now selects only `persistent-pipeline-cache`.
Its assertions remain unchanged: the original 31-render/1-compute catalog, all
71-render/2-compute optional pipelines, reopen, checksum corruption, recovery,
failed-preparation identity, repeated configuration and no write on Drop.
The separate Metal/Vulkan/Dx12 cooked suites select the complete
`cooked-native-shaders` family plus `cooked-temporal` and caching, so they now
exercise actual cooked bloom too. Default suites and partial-cooked admission
unit controls remain in place. No configured job or required context is removed.

A regression check resolves the actual renderer Cargo feature closure for each
workflow command. It requires every cooked family in the cooked suite and keeps
cooked output/materials out of the all-effects cache harness. All32 focused CI
configuration tests, generated conventions and diff checks pass locally.
[Exact source and failure identities](evidence/ci-renderer-catalog.json) record
this qualification. No native or GPU test was executed locally for this workflow
repair. Fresh hosted cache and complete cooked-bloom results remain pending;
this change alone does not establish cache hits or any shader-stall gate.
