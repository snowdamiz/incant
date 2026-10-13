# CCD observer return alias

The default-off CCD observer now uses a private transparent lifetime alias for
its existing three-state borrowed return value. Its body, feature gate, visibility
and native behavior are unchanged. Both vendored dimensions have byte-identical
source. This closes the strict Clippy `type_complexity` failure retained by the
[partial constructor producer](physics-native-construction.md).

Current CCD capture pins bind the new source. Historical pins/source are retained;
the new upstream audit removes both the alias and getter and recovers exact
upstream SHA `0543f497406c6d404b6b214d42333532bfe97fc07180da2c946d196d60b7fee8`.
The earlier getter-only audit remains historical. The copied native test workspace
retains the pre-pin-update metadata; its actual two compiled CCD files match the
new source, and all other 666 copied inputs match the accepted constructor base
(apart from the documented optional dependency path redirection).

Six recorded commands pass: preparation, offline metadata, strict native library
Clippy with observer plus constructor/serde/parallel/deterministic features,
20 actual native observer tests, formatting and diff checks. The final JSON pin
correction changes no compiled source and requires no repeated build. This is a
scoped native library check, not an all-target or all-feature claim. No live save
admission, constructor expansion, GPU, device or performance result is implied.

[Lossless evidence](evidence/physics-native-observer-lint.json) retains commands,
raw logs, source generations, upstream reconstruction, original failed lint and
independent final acceptance `ca02a3658214fe297e040dcf62373cf170b82196c4e1759641da207b08bb8484`.
The accepted constructor final root receipt is retained alongside it.
