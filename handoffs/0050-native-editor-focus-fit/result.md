# 0050 focused native editor verification: result

## Status

- **Both 0048 UI fixes pass natively** in the actual WKWebView of the granted rebuilt editor.
  - **Focus after ⌘P (D1): pass.** ⌘P on the picker leaves focus on Play, which becomes Pause.
    Focus elsewhere stays where it is. The no-camera refusal and keyboard Dismiss keep focus on
    Play.
  - **Model field (D2): pass.** It shows the whole `gpt-6-astra` at 280 and 380 px, in both
    1440 × 875 and 1000 × 650, and nothing around it overflows.
- **Account and usage: retained.** The reopened editor is signed in, with no sign-in step, and
  shows 22,044 of 64,000 tokens.
- **"No network request": not met.** The editor made one short HTTPS exchange at startup,
  before any of my input (details below). This is backend behaviour, so root owns it. No UI
  change follows from it.
- **No source change.** No UI defect was found, so no UI file was edited. No provider turn was
  sent, and no account dialog was opened.
- **Native and GPU ownership: released.** The editor exited at 23:58:39Z, and every owned
  helper has exited.
- **No gate claim.** No phase-gate, cross-platform, device-performance or full-accessibility
  claim is made.

## Model and effort

- **Model:** Claude Opus 5.5, model ID `claude-opus-5-5`.
- **Thinking effort:** Max.
- **How it was set:** the runner `tools/handoff/main.py run 0050-native-editor-focus-fit`
  (PID 64275) started this session through `claude-agent-acp` (PID 64327). `tools/acp/config.json`
  pins `claude-opus-5-5` and `max`. Its `configure_model_and_effort` raises before prompting
  unless ACP confirms both settings.
- **What I could not see:** the runner's confirmation line is not visible from inside the
  session.
- **No substitution, no routing exception, no auxiliary agent.**

## Packet

- **Revisions:** the brief has no priority revision and no new director feedback. It was
  applied as written.
- **Starting state:** the worktree was clean at `5970830`, with no interrupted attempt.
- **Root's files:** `brief.md` and `build.json` were not changed.

## Build, fixture and window

| Item | Value |
| --- | --- |
| Source | `4589c2ba5edf2d89587fef8b815bdca1cd4af01e`. All 897 manifest source hashes match its blobs. The later commits change only `STATUS.md`, docs evidence and this packet |
| Manifest | `/tmp/incant-native-review-0050-bin-v1/manifest.json`, `2d69d15c…70a3` |
| Editor | `…/incant_editor`, `89f6b0d990cbaf1f275b8b0e7d3c448a543ef7b4199239ec7e57c419c27623fd`, 62,675,088 bytes. Hash checked again immediately before launch. Not rebuilt or replaced |
| Headless | `e6189800…b564`. Verified, never launched |
| Fixture | `/tmp/incant-native-0048-fixtures/saved/project.json`, project `01M4PEJQNPMDQYEDGT0PRWN78X`. `project.json` is `3ff0ded1…`; the journal is `0fd36d3e…` with 23 records. Both were identical before launch and after quit, and no project edit was made |
| Usage ledger | `~/Library/Application Support/Incant/agent-usage/01M4PEJQNPMDQYEDGT0PRWN78X.json`, `91cecc82…`: `charged_tokens` 22044, `accounting_uncertain` false, `pending` null. Identical before and after. Read by exact path only; no credential, keychain or account store was read |
| Host | macOS 26.6 (25G72), arm64, built-in display 1512 × 982 pt at 2×. The visible desktop is 1512 × 875 pt |
| Window | Largest available: **1440 × 875** (1440 × 900 does not fit). Smallest: **1000 × 650**, set exactly through AX |
| Column | Right column **380** (dragged exactly −20 and −100 pt) and **280** (Home). Values read back from the separator |

## Results

The numbers come from accessibility reads of the owned PID: `evidence/native-review.json`,
`focus-sequences.jsonl` and `states.jsonl`. Focus was read every 0.2–0.4 s.

| Check | Result |
| --- | --- |
| ⌘P from the picker | **Pass.** I opened the picker, chose Main · Stationary Camera with ↓ and Return, and focus came back to the picker. After ⌘P, focus was on the primary transport button at +0.38 s (named "Play" while preparing) and on the same button named "Pause" from +1.29 s. The state was Playing · Main, with the picker hidden. Capture: `play-01` |
| Focus kept in another panel | **Pass.** ⌘P with Pause focused stopped Play, and focus stayed on that button, now Play. I then focused the Stationary Imported Mesh row in the Hierarchy. ⌘P started Play and ⌘P stopped it, and every read in between found the row focused |
| No-camera refusal and Dismiss | **Pass.** I chose Spatial Without Camera, then pressed ⌘P from the picker. The header showed "Play failed", with the notice "Play could not start. …" and Dismiss. Focus was on Play from +0.34 s; in 0048 this flow left nothing focused. Tab went Step → Stop → Dismiss, and Space returned focus to Play, with the notice gone. Capture: `play-02` |
| Model field, all four layouts | **Pass.** The field is 91 pt wide everywhere (`calc(11ch + 12px)` = 91.2). `AXBoundsForRange` puts all 11 characters inside the field's content box: the text ends 1 pt before the edge. In each capture, the field holds 11 glyph runs, and the last is a full-height "a", not an ellipsis's three dots. `AXVisibleCharacterRange` returned {0,0}, so it was not used. Captures: `play-01`, `composer-01` to `-04` |
| No surrounding overflow | **Pass.** Model, Approval ("Ask before edits", the longest label) and Send keep their widths. Send ends exactly at the bar's inner edge. At 280 px, 5 px separate Approval and Send: the 2 px gap plus 3 px spare, so the field is not squeezed. There is no scroll bar, and no agent-pane control extends past the pane |
| Account and usage retained | **Pass for state.** At startup, before any input, and again before quit: "ChatGPT account, signed in as [redacted]", Agent Ready, "22,044 of 64,000 tokens used". There was no sign-in step. The identity itself was not compared: it is redacted, and the account store is off limits |
| No network request | **Not met.** See the next section |

### The startup network exchange (for root)

- **What happened:** about 1 s after spawn, `lsof` showed the editor process with
  `TCP <local>:49832 -> 172.64.146.15:443 ESTABLISHED`, a Cloudflare-range address. It was seen
  in 3 samples between 23:42:27 and 23:42:28Z.
- **Bytes:** `nettop`, attached to the PID at spawn, counted 4,932 B in and 2,304 B out in its
  first second, then 18,756 B in and 2,509 B out. The totals did not change again until exit,
  across 927 one-second rows.
- **Nothing else:** no other socket appeared in 1,681 samples, children included. The first
  input of the review came at 23:44:42Z, two minutes later.
- **Likely cause, from source:** this is unverified. The editor's `restore` provider action
  (`editor/app/src/provider.rs`) returns `public_state(true)`. That calls
  `incant_agent::auth::access_token`, which renews the saved session when the access token
  expires within 60 s. Renewal is a token POST, then GETs of the OpenID configuration and JWKS,
  all on `auth.openai.com`.
- **Why 0048 saw no socket:** these files are unchanged since 0048's build. 0048 reopened five
  minutes after its turn and saw no socket; this run came about 41 minutes later. Both fit a
  renewal that depends on token expiry.
- **Not verified:** I did not read the token's expiry, which is forbidden here, and I did not
  observe the host name.
- **Decision for root:** whether startup should renew eagerly, or the expectation should be
  "no provider (inference) request".
- **Method:** a local loopback test first showed that `nettop` counts a short flow only if it is
  attached while the flow exists. These are still samples, not proof of zero traffic. Details
  are in `evidence/network.txt`.

## Captures

The six images are in `screenshots/native/`, all reviewed: five window captures and one 2×
crop, the packet's limit. The README describes each.

- **Redaction:** the titlebar account avatar is filled with the canvas colour in the
  full-window images. Each differs from its raw capture in exactly 2,040 pixels, all over the
  avatar. The images keep the captures' Display P3 profile.
- **Raw captures:** they stay local in the git-ignored `artifacts/0050/`. Hashes are in
  `evidence/native-screenshots.sha256`.

## Processes and cleanup

| PID | What | End |
| --- | --- | --- |
| 71838 | `incant_editor` with the fixture, launched 23:42:26Z | ⌘Q 23:58:35Z, gone 23:58:39Z |
| 77320, 79786 | Short-lived editor children at the two successful Play starts (not inspected) | gone |
| 71839 | `nettop` on the editor PID | stopped at the editor's exit |
| — | `netwatch.sh` | exited 23:58:36Z |
| 69780, 69894 | The pre-launch loopback check for `nettop` | exited |

- **Short-lived helpers:** `launch.sh`, eight `burst.sh` runs, `winctl`, `flow.py`, `state.py`,
  `neutral.py` and `imgtool` all exited. `pgrep` finds no editor or helper process.
- **Bursts:** each burst waited for 15 s without input, checked the front app before every
  event, and handed the front and pointer back. Three bursts waited 46, 172 and 106 s.
- **Nothing outside the window:** I captured no other app and no desktop.

## Slips, recorded

- **Undo looks filled in three captures.** `play-01`, `play-02` and `composer-01` were taken
  with the pointer parked inside the Undo button, so it shows its hover fill. Later captures
  parked the pointer on empty titlebar.
- **First redacted copies were converted to sRGB.** That changed every pixel value slightly.
  They were regenerated in the original colour space, and no capture was retaken.
- **Start-of-burst focus reads were null.** They ran while the editor was in the background.
  Attribution uses the first reads after the window came forward.
- **Two read-only calls failed and were redone.** One was a zsh `=Model` expansion and the
  other a null focus in `state.py`. Neither posted input.

## Changed paths

All are new, in `handoffs/0050-native-editor-focus-fit/`:

- `result.md`;
- `evidence/`: `native-review.json`, `focus-sequences.jsonl`, `states.jsonl`, `network.txt`,
  `hashes.txt`, `native-screenshots.sha256`, `bursts.txt` and `images.txt`;
- `screenshots/native/`: 6 PNGs and a README;
- `tools/native/`, with these files:
  - `winctl.swift`: 0048's tool plus `ax attr`, `ax bounds` and `ax focuspath`;
  - `launch.sh`: the 0050 hash check, an anchored process guard, and `nettop` plus the sampler
    from spawn;
  - `burst.sh` and `cap.sh`: `cap.sh` enforces the six-image budget;
  - `netwatch.sh`, `state.py`, `flow.py`, `neutral.py` and `imgtool.swift`.

No UI, bridge, Rust, style or fixture file was changed, and nothing outside this worktree.
`artifacts/0050/` is ignored and local.

## Checks not run

The UI suite, strict TypeScript and the production build were not re-run, because no source
changed. Root's 904-test run on `4589c2b` is root's evidence, not mine.

## Limitations

- **Coverage:** macOS on this display only, with two window sizes, two column widths, the
  default model value and the default approval mode. A longer model value was not checked.
- **Focus reads:** focus was polled, so a focus change shorter than one poll would not be seen.
- **Network:** the network observations are samples of this run. `nettop` covered only the
  editor PID, and WebKit's processes were not watched.
- **Out of scope here:** the refusal text still has the `--camera` wording noted in 0048
  observation 5, and the picker label truncates at 1000 / 380. Both are already tracked.

## Open questions

1. **For root:** should the editor renew a saved session over the network at startup, or should
   the expectation be "no provider request"?
2. **For the director:** none.
