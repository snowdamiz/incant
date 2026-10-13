# Bounded hierarchy-menu accessibility test

Frozen checkpoint25 Windows desktop job `115406538226` failed at the existing five-second deadline in the pointer/context-menu/delete case: 935 tests passed and one timed out. The complete job metadata and raw log remain retained. Later missing gameplay-export artifacts follow that early UI abort, not an executed export failure.

The original case passes locally. Instrumented local execution measured about 362 ms of setup and 827 ms of full-App axe scanning; the original file was restored before applying the repair. Hosted scheduling and scanner cost are an inference, not a reproduced or isolated Windows cause.

Only that existing case receives a bounded twenty-second deadline, consistent with the existing full-page DockPaging audit budget. All original action, focus, accessibility, dispatch and command-bus assertions remain. There is no global timeout, production, CI, retry or skip change. The ordinary UI suite is not a performance gate.

Five repaired-source checks pass: all three focused tests, all 936 UI tests across 45 files, the strict TypeScript/UI build, conventions and whitespace. The original hosted failure is unchanged; a later frozen checkpoint must provide actual Windows evidence. Final independent review is pending. [Manifest](evidence/hierarchy-menu-timeout.json).
