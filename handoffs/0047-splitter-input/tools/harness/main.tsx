/**
 * BROWSER HARNESS for handoff 0047: the editor's real Splitter, shell styles and input
 * modality tracking, between panes of selectable text. It is not the editor and not the
 * native window. browser-review.mjs builds it twice: with the Splitter from the base
 * commit ("before") and with this worktree's Splitter ("after").
 *
 * ?layout=vertical   a column separator; the left pane's width (invert off)
 * ?layout=horizontal a row separator; the bottom pane's height (invert on, like the dock)
 */
import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@fontsource-variable/inter';
import '@fontsource-variable/jetbrains-mono';
import '../../../../editor/ui/src/styles/tokens.css';
import '../../../../editor/ui/src/styles/base.css';
import '../../../../editor/ui/src/styles/shell.css';
import './harness.css';
import { Splitter } from '../../../../editor/ui/src/components/Splitter';
import { installInputModality } from '../../../../editor/ui/src/shell/inputModality';

const params = new URLSearchParams(window.location.search);
const layout = params.get('layout') === 'horizontal' ? 'horizontal' : 'vertical';
const variant = params.get('variant') ?? 'unlabelled';

const TRANSFORM = [
  'Translation X 310 Y 0 Z 0',
  'Rotation X 0 Y 0 Z 0 W 1',
  'Scale X 1 Y 1 Z 1',
];

function Prose({ id, title }: { id: string; title: string }) {
  return (
    <section className="harness__pane" aria-labelledby={`${id}-title`}>
      <h2 id={`${id}-title`}>{title}</h2>
      {TRANSFORM.map((line) => (
        <p key={line} className="harness__mono">{line}</p>
      ))}
      <p data-prose={id}>
        This paragraph is ordinary selectable text beside the splitter. Dragging the separator must
        resize the panes without selecting it, and a drag that starts on the text must still select it.
      </p>
      <p>
        A second paragraph gives a press room to start a normal selection and a drag past the clamp
        something to cross.
      </p>
    </section>
  );
}

function Harness() {
  const [size, setSize] = useState(layout === 'vertical' ? 360 : 260);
  useEffect(() => installInputModality(document), []);
  const grid = layout === 'vertical'
    ? { gridTemplateColumns: `${size}px var(--splitter-size) minmax(0, 1fr)` }
    : { gridTemplateRows: `minmax(0, 1fr) var(--splitter-size) ${size}px` };
  return (
    <div className="harness">
      <p className="harness__caption">
        Browser harness, headless Chromium (not the native window): Splitter {variant}, {layout} separator, value {size}
      </p>
      <main className={`harness__split harness__split--${layout}`} style={grid}>
        <div className="harness__first">
          <label className="harness__field">
            Name <input defaultValue="Mover" />
          </label>
          <Prose id="first" title={layout === 'vertical' ? 'Left pane' : 'Top pane'} />
        </div>
        <Splitter
          label={layout === 'vertical' ? 'Resize left pane' : 'Resize bottom pane'}
          orientation={layout}
          invert={layout === 'horizontal'}
          value={size}
          min={layout === 'vertical' ? 200 : 120}
          max={layout === 'vertical' ? 560 : 420}
          onChange={setSize}
        />
        <Prose id="second" title={layout === 'vertical' ? 'Right pane' : 'Bottom pane'} />
      </main>
    </div>
  );
}

const root = document.getElementById('root');
if (!root) throw new Error('Missing #root element');
createRoot(root).render(
  <StrictMode>
    <Harness />
  </StrictMode>,
);
