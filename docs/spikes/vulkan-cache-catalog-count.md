# Correct the Vulkan persistence harness catalog count

Checkpoint19 Linux desktop failed at an exact test assertion: the Vulkan adapter
allocated 31 render pipelines, while the persistence harness expected 30.
The adapter was llvmpipe (LLVM 20.1.2), Mesa 25.2.8, backend Vulkan. The full
hosted failure log, job identity and frozen candidate are retained in
[the evidence packet](evidence/vulkan-cache-catalog-count.json).

The fixed catalog is 12 materials + 8 shadows + 1 diagnostic + 4 outputs +
1 sprite + 1 text + 4 UI pipelines = 31 render pipelines, plus exactly one
compute pipeline. UI already constructs all four accepted output formats;
the test comment incorrectly counted three. This repair changes only the test
comment, exact expected render count and diagnostic text. It preserves the
Missing → Supplied → Rejected → Supplied cache sequence, Stored assertions,
checksum corruption, resource stability checks, bounded children and actual
Vulkan backend requirement.

Five scoped checks pass: full cooked persistence-harness compile, Clippy,
format, conventions and base diff. They were run on macOS without GPU or
window execution. The test must still execute all four fresh Vulkan processes
successfully in a later frozen checkpoint. Checkpoint19 remains unchanged and
unpromoted while its remaining jobs finish.

The evidence collector verifies all seven production constructor files against
main200; six also match checkpoint19 exactly. The renderer constructor differs
only by the previously merged cooked-cluster backend/cache call. An initial
whole-file comparison of lib.rs was too broad because it also includes DRS;
the failed collector assertion and precise correction are retained. No source
or compiled-check result changed during that qualification.

[Independent final source and evidence review](evidence/vulkan-cache-catalog-count/final-review.json) is accepted. Actual Vulkan lifecycle execution remains required in the successor checkpoint.
