# Custom titlebar: host contract (for Astra)

Status: proposal from handoff 0001, revision 1. The UI side is implemented in
`src/components/Titlebar.tsx`; types are in `src/bridge/contract.ts`. **No native window
ran for this handoff.** Window behavior is untested against a real host; the browser
captures use labeled evidence test doubles.

## What the UI renders

```
[leading inset][logo] Project name [Sample data] ……drag…… [↶ ↷] | [◧ ⬓ ◨] ……drag…… (● ChatGPT Sign in) [⌨] [— ▢ ✕][trailing inset]
```

- 40 px tall, canvas color (`--color-bg-app`). Revision 2 (handoff 0006) draws a 1 px
  hairline at the bottom as an inset shadow, so content still centres on the full 40 px.
  The tool cluster is centred in the window between equal-width start and end sections.
  Empty space in both sections is a drag area.
- The whole bar is a drag handle except interactive elements (`button`, `a`, `input`,
  `[role=button]`, `[data-no-drag]`).
- Layout toggles (hierarchy, output, inspector) and the shortcuts button are local UI
  state, not bridge calls. Undo/redo go through `dispatch` (`history.undo|redo`).
- Unfocused windows (`window.focused === false`) dim the identity and tools to 55 %.

## Bridge additions

Snapshot (absent outside the native host):

```ts
interface WindowChrome {
  platform: 'macos' | 'windows' | 'linux';
  controls: 'native-overlay' | 'custom';
  maximized: boolean;
  fullscreen: boolean;
  focused: boolean;
  leadingInset: number;  // CSS px kept clear at the leading edge
  trailingInset: number; // CSS px kept clear at the trailing edge (OS-drawn controls)
}
BridgeSnapshot.window?: WindowChrome
```

Capabilities and host requests (not journaled; not document edits):

| Capability | Request | UI trigger | Tauri v2 mapping (suggested) |
| --- | --- | --- | --- |
| `window.drag` | `{ type: 'window.drag' }` | primary mousedown on a drag area (single click only) | `Window::start_dragging()` |
| `window.maximize` | `{ type: 'window.maximize' }` | maximize button; double-click on a drag area | `toggle_maximize()`; on macOS honor `AppleActionOnDoubleClick` (zoom / minimize / none) |
| `window.minimize` | `{ type: 'window.minimize' }` | minimize button | `minimize()` |
| `window.close` | `{ type: 'window.close' }` | close button | `close()` (host decides about unsaved-work prompts) |

Rules the UI follows:

- A caption button is drawn only when `window.controls === 'custom'` **and** its
  capability is advertised. Nothing is drawn as a placeholder.
- Without `window.drag` the bar is inert. The host may instead use Tauri's
  `data-tauri-drag-region`; if so, do not also advertise `window.drag` (double drag).
- After a request, the host updates `window.maximized/focused/fullscreen` in the
  snapshot; the UI never assumes the outcome (e.g. the maximize icon switches to
  "restore" only when the snapshot says `maximized: true`).

## Platform specifics

### macOS: native traffic lights (`controls: 'native-overlay'`)

- Window config: `titleBarStyle: "Overlay"`, `hiddenTitle: true`, `decorations: true`.
- Position the traffic lights vertically centered in the 40 px bar:
  `trafficLightPosition: { x: 14, y: 22 }`. Measured natively in handoff 0002 (tao
  0.37.1): `y` is extra titlebar-container height, not a top offset, and the light
  centre lands at `y − 2.25` pt. With y=22 the centres sit at 19.75 pt, level with the
  logo; y=14 left them 8 pt high.
- Logo placement (handoff 0006, director feedback "too close to traffic lights and not
  aligned"): the 16 px logo box starts at 86 pt, so the glyph starts at 88.5 pt, 15 pt after
  the green light (which ends at 73.5 pt). The glyph is 14.5 pt tall, close to the 14 pt
  lights, and its centre is 19.75 pt, level with the lights. Measured in a browser capture
  with the macOS inset double; native confirmation is requested in that handoff.
- Report `leadingInset: 78` (traffic lights + margin) and `trailingInset: 0`.
  In fullscreen the lights hide: report `fullscreen: true, leadingInset: 0`.
- Do not draw custom caption buttons on macOS.

### Windows: custom caption buttons (`controls: 'custom'`)

- Window config: `decorations: false` (keep the native resize border and shadow; on
  Windows 11 enable rounded corners via DWM if Tauri does not by default).
- Advertise `window.minimize`, `window.maximize`, `window.close`, `window.drag`.
- Buttons are 46 × 40 px, edge-aligned at the right with no padding, Close hover
  `#c42b1c` with white glyph, matching Windows 11 caption metrics.
- Open item: Windows 11 Snap Layouts appear on hover over a *native* maximize button
  only. Getting them with custom chrome needs WM_NCHITTEST returning HTMAXBUTTON over the
  UI's maximize button rect. Astra to decide whether to support this (the UI can report
  the button rect via a future `window.captionRects` request).

### Linux

- Default to `controls: 'custom'` with `decorations: false`, same as Windows. Under
  Wayland compositors that force server-side decorations, report `controls:
  'native-overlay'` with zero insets and the UI will draw no buttons.

## Window background and transparency

When the native viewport is attached the webview is transparent (see
NATIVE_VIEWPORT.md). The **window background color must be the canvas color
`#0b0c0f`**. Revision 2 has no gutters and a square viewport, so this colour is visible
only during a resize, before the webview repaints.

## Open questions

1. `data-tauri-drag-region` vs. `window.drag` request: which does the host prefer?
2. Should the titlebar host an application menu (File/Edit/View) on Windows/Linux? On
   macOS the system menu bar covers it. Not designed in this revision.
3. Snap Layouts support (above).
