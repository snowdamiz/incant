/** Panel sizes in CSS pixels. The viewport takes whatever remains. */
export interface Layout {
  readonly left: number;
  readonly right: number;
  readonly dock: number;
  readonly agent: number;
}

/** Fixed chrome outside the workspace grid: titlebar (40) + status line (28). */
export const CHROME_HEIGHT = 40 + 28;
/** Separators are 1 px lines (their drag target overlaps the neighbours). */
export const SPLITTER = 1;
/** The viewport never shrinks below this, so the native surface stays usable. */
export const MIN_VIEWPORT = { width: 360, height: 200 } as const;

const clamp = (value: number, min: number, max: number) => Math.round(Math.min(max, Math.max(min, value)));

/**
 * Proportional defaults after the landing-page illustration (a narrow outline, a wide
 * viewport, a ~20 rem inspector), tuned so 1280×800 and 1920×1080 both keep a readable
 * hierarchy (≥ 232 px), an inspector wide enough for a vec3 row (≥ 320 px), and a
 * viewport at least half the window wide.
 */
export function defaultLayout(width: number, height: number): Layout {
  return clampLayout(
    {
      left: clamp(width * 0.17, 232, 296),
      right: clamp(width * 0.23, 320, 400),
      dock: clamp(height * 0.27, 160, 340),
      agent: clamp(height * 0.33, 260, 340),
    },
    width,
    height,
  );
}

/** Keeps every panel within its bounds and preserves the minimum viewport size. */
export function clampLayout(layout: Layout, width: number, height: number): Layout {
  const workspace = Math.max(0, height - CHROME_HEIGHT);
  let left = clamp(layout.left, 200, 480);
  let right = clamp(layout.right, 280, 560);
  const overflow = left + right + 2 * SPLITTER + MIN_VIEWPORT.width - width;
  if (overflow > 0) {
    // Take space back from the wider side panel first.
    const fromRight = Math.min(overflow, right - 280);
    right -= fromRight;
    left = Math.max(200, left - (overflow - fromRight));
  }
  const dock = clamp(layout.dock, 120, Math.max(120, Math.min(640, workspace - SPLITTER - MIN_VIEWPORT.height)));
  const agent = clamp(layout.agent, 180, Math.max(180, Math.min(560, workspace - SPLITTER - 160)));
  return { left, right, dock, agent };
}

/** Room for the compact unavailable message and the one-line composer (measured). */
export const IDLE_AGENT_MIN = 200;

/**
 * Agent height while no agent can run (signed out, not ready, unsupported). The pane
 * then only explains why and offers sign-in, so it keeps just the compact empty
 * state plus the disabled composer and gives the rest of a short window to the
 * Inspector. Never taller than the regular agent height for the same window.
 */
export function idleAgentHeight(height: number): number {
  const workspace = Math.max(0, height - CHROME_HEIGHT);
  return clamp(height * 0.24, IDLE_AGENT_MIN, Math.max(180, Math.min(240, workspace - SPLITTER - 160)));
}
