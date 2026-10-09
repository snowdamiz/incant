# Native keyboard focus and project-load states

Use Claude Opus 5.5 through the configured ACP adapter. The director requested
continued editor polish while Astra works on logic. Computer use and screen
capture are permitted again. Native capture/input must be performed by Astra
through Codex CUA; no native helpers, screencapture, CGWindow, AppleScript or
synthetic native input. Your ACP session has no CUA. Browser fixture review is
permitted. Do not read credentials or perform account actions. Keep private
native captures and real account labels outside Git. Preserve the neutral
charcoal connected panels and restrained accents. No new redesign or site edits.

## Scope

1. Fix the native keyboard-opened account dialog's missing focus ring. Handoff
0008 confirms Close receives focus but WebKit does not draw :focus-visible on
that programmatic focus; Chrome does. Use robust input-modality handling so a
keyboard-opened dialog clearly identifies Close, and keyboard navigation keeps
focus visible. Pointer-open should retain existing behavior. Do not alter the
safe initial Close/Keep signed in focus, escape behavior, account auth/storage,
or sign-out semantics. Cover observable keyboard versus pointer behavior in a
meaningful regression test. Implement all related visuals yourself.

2. Present actual project loading/error states truthfully throughout the editor.
Astra moved project read/validation/journal recovery off the main event thread.
Native engine responses now include loading, ready and error; the adapter maps
loading to connection=connecting and hierarchy=loading, and error to both error
states plus an error diagnostic. Failed/obsolete loads never retain entities or
history. Saved account metadata is independent and stays visible. Fix panels
that still claim 'No problems', current values or success while connecting or
failed. Use concise existing visual language, avoid redundant walls of error
text and retain accessible live status. A project file can time out after 15s;
reopening is the current retry mechanism, so don't invent an unconnected retry
button. No new engine commands or runtime dependencies. Do not hide errors.

## Paths

Edit editor/ui/src/**, relevant fixture/tests and docs. Read editor/bridge/native.ts
and editor/app/src/project.rs for the state contract, but Astra owns those files.
Previous review: handoffs/0008-native-and-site-review/result.md and native-requests.md.
Read TITLEBAR.md and existing account focus behavior before changing it.

## Acceptance and verification

- All editor tests, strict TS and production build pass.
- Review browser pixels at 1000x650 and 1440x900 for loading, failed, ready and
  keyboard/pointer-opened account states. Keep only synthetic fixture screenshots
  in the committed packet. Record exact commands, model and measured results.
- Do not claim browser tests prove native WebKit behavior. Request specific CUA
  captures in native-requests.md and leave that native verification open until
  Astra supplies them. The Mac was locked at the last CUA attempt.
- Existing F2 selection, project/text Undo routing and account restoration must
  remain intact. No edits outside your worktree, no external publishing/merging.
- Bundled fonts/icons and UI gzip budgets unchanged; no dependency upgrades.
- Use npm test --workspace editor/ui and npm run build --workspace editor/ui.
  Browser fixture capture tooling from 0008 can be reused on a separate port.
- Commit with Built-by: claude. Return result.md with code, tests, evidence and
  limitations. Astra integrates and merges after passing checks.
