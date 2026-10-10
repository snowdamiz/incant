# Plain editor release union

The current code union is a20d258: the plain editor and accessibility corrections
through ce06c31, plus reviewed sprite/atlas head8a7fbaf. The authored calendar-free
PLAN revision7 commit8fe00eca is preserved by merge65129c8. Only the STATUS merge
needed resolution; the latest implementation notes and revision7 reference remain.
Editor, runtime, schemas, dependencies and workflow bytes match a20d258 exactly.

[Local evidence](evidence/editor-release-union-2026-10-10.json) records source
objects and log hashes:451 release workspace tests,56 ignored GPU tests, release
all-target Clippy, generated bridge,503 UI tests/build and the custom-protocol
native editor build pass. After the PLAN integration, all40 Python tools tests,
generated conventions and whitespace checks pass. The staffing gate remains open
under its new `initial_staffing_confirmed` key; no gate is approved.

Claude Opus5.5 at explicitly verified Max reviews the real-schema Camera and
sprite/atlas UI union through [handoff0035](../../handoffs/0035-editor-union-review/brief.md).
The five plain-design requirements and prior behavioral corrections remain in
force. This review is pending; these build checks do not approve rendered pixels.

The director's running native preview is the older0d14f4f binary. It remains
untouched. The newly built union has not been opened or visually accepted; older
captures do not certify its native menus, dialogs, minimum size, paging or focus.
Hosted checks and the exact current-source rendered review remain pending before
release integration. Device/performance gates and full engine completion remain open.
