# Windows starter destination repair

Checkpoint42's Windows job failed all three headless starter tests at
`incant_templates::create`, returning bare `Io` OS1, “Incorrect function.”
The original failure is retained. The destination validator inspected each path
component immediately, including a canonical Windows verbatim drive prefix
before its root separator. That intermediate prefix is not a rooted directory.
Rust's [component contract](https://doc.rust-lang.org/std/path/enum.Component.html)
separates `Prefix` from the following `RootDir`.

Source `52468867f2e5af99c7cfaa442bb57e8eb11779ee`, based on main336 `a72a76f3`,
checks complete absolute ancestors from the root down. Prefix and root therefore
form one metadata anchor. Every normal ancestor still receives its no-follow
metadata check before descendants; unresolved drive-relative paths and `..`
remain refused. Directory creation, raw-file sync, source-aware command journal,
final no-replace publication and propagation of durability failures are unchanged.
Failures after creation still retain an explicitly incomplete directory.

On this Mac, Rust1.99 arm64 CI qualification passed all seven template unit tests,
all three actual headless template gameplay tests, and strict all-target Clippy.
The existing five-starter create/source/Undo/Redo/reopen, no-overwrite, injected
publication failure and symlink controls remain intact. New coverage checks absent
and non-directory ancestors, no writes on refusal, and a Windows-only canonical
verbatim destination through real creation and journal reopen. That Windows-only
control was **not run on the Mac**; hosted Windows execution remains required.

The three managed calls and supervisor exited successfully and were reaped.
Their process groups were independently absent afterward. Three qualified
executables were copied outside the released FONT target; none is checked in.
Peak sampled target growth was 107,057,307 bytes under the 2GiB cap; sampled free
space stayed above the 6GiB floor. No GPU, native window, browser, capture or CUA
was used. The initial formatting check requested one test line wrap, corrected
before compilation; no native qualification attempt failed.

[The compact packet](evidence/windows-template-destination/manifest.json) retains
exact commands, source hashes, original Windows error, raw CPU logs and cleanup.
Replay is data-only:

```sh
python3 docs/spikes/evidence/windows-template-destination/verify.py
python3 docs/spikes/evidence/windows-template-destination/verify.py --originals --binaries --git-source 52468867
```

## Checkpoint43: production repair confirmed; malformed fixture corrected

Hosted Windows passed the canonical verbatim destination and the complete
five-starter create/source/Undo/Redo/reopen tests. The seven-test template suite
still failed its malformed-path fixture: chained `PathBuf::join` calls had
removed the literal parent segment before validation. Rust documents that
[appending to a verbatim Windows path normalizes `.` and `..`](https://doc.rust-lang.org/std/path/struct.PathBuf.html#method.push).
The function therefore received a valid destination, not the intended refusal input.

The fixture now appends native separator strings and components to `OsString`,
constructs the path without joining, and asserts that `ParentDir` is present
before checking the unchanged refusal. Production code is byte-identical.
All seven Mac unit tests and strict all-target Clippy pass. Independent source
review found no blocker. The corrected Windows fixture still needs the next
hosted checkpoint; the failed checkpoint43 result remains recorded.

Both bounded local processes exited successfully and were reaped. No GPU,
window or capture was used. [Compact source, failure and qualification evidence](evidence/windows-parent-fixture.json).
