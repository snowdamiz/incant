# Release checkpoint38 — exact main290 promoted

[PR291](https://github.com/snowdamiz/incant/pull/291) merged on 2026-10-14 after
all 23 required contexts and eight exact-candidate push workflows succeeded.
The [retained promotion record](evidence/release-checkpoint-38.json) includes
the frozen identities, required checks, workflow/job outcomes, integration
review and verified post-merge tree and author trailer.

- Selected main: `91f0ce464f97795f2114f03fc0f43d06008a082e`.
- Frozen candidate: `cf54bc82bb3485c6b0c3ab045f62ebd31ad318a8`.
- Promoted release: `a453d82d853dc3a8648ad27e612e29eda7a3d46d`.
- Shared tested tree: `5f4bf46ffd1af56b36c85f76dcdbe0b2c5bcd122`.

The candidate remained frozen while main development continued. All eight
workflows completed on their first attempt: 27 jobs succeeded and only the
nonrequired website deployment was skipped. No missing, pending, skipped or
neutral required result counted as success. Release protection and its 23
required contexts were unchanged. The merge preserves prior release/candidate
history and ends with `Built-by: astra`.

This batch includes actual Mac texture and compatible static-model refresh
workflows, their numerical Metal controls, and the default-off MetalFX native
foundation. Its negative material/motion appearance review remains explicit;
promotion does not make that renderer mode visually acceptable or enable it.
Later profiled shipping-window exports and ordinary-editor antialiasing are
outside this tested tree, as are the private reconstruction experiments.

This is an integration baseline, not a performance, rendering-quality or phase
gate approval. The script-time requirement remains unmet. Other-device execution
is deferred, and no distribution was signed or published.
