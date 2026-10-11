import { useEffect, useRef } from 'react';
import { projectState, sameError } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';

/**
 * The area the engine's native surface occupies. The UI never draws engine pixels
 * here. When the bridge supports `viewport.bounds`, the host is
 * told where the hole is so it can position the native child surface; see
 * NATIVE_VIEWPORT.md for the constraints this layout is designed around.
 */
export function ViewportPanel() {
  const { snapshot, capabilities, ask } = useShell();
  const hostRef = useRef<HTMLDivElement>(null);
  const reportBounds = capabilities.has('viewport.bounds');

  useEffect(() => {
    const host = hostRef.current;
    if (!host || !reportBounds) return;
    let frame = 0;
    const send = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const rect = host.getBoundingClientRect();
        void ask({
          type: 'viewport.bounds',
          rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
          devicePixelRatio: window.devicePixelRatio,
          cornerRadii: readRadii(host),
        });
      });
    };
    const observer = new ResizeObserver(send);
    observer.observe(host);
    window.addEventListener('resize', send);
    send();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      window.removeEventListener('resize', send);
    };
  }, [ask, reportBounds]);

  const viewport = snapshot?.viewport;
  const project = projectState(snapshot);
  const attached = viewport?.status === 'attached';
  let status: string;
  let title: string;
  let detail: string;
  // The header's status says the state; the empty area adds only what it does not:
  // a title where the state needs one, and the host's reason or error verbatim.
  if (!viewport) {
    status = 'No engine';
    title = 'No viewport';
    detail = '';
  } else if (viewport.status === 'attached') {
    status = 'Live';
    title = '';
    detail = '';
  } else if (project.kind === 'loading') {
    // The banner says the project is opening; the title stays for assistive tech only.
    status = 'Waiting';
    title = 'Opening project…';
    detail = '';
  } else if (viewport.status === 'error' && project.kind === 'failed' && sameError(viewport.error, project.error)) {
    // The project failed, not the surface. The banner carries the full message.
    status = 'Error';
    title = 'No project loaded';
    detail = '';
  } else if (viewport.status === 'error') {
    // One state covers surface setup, scene preparation and drawing failures. The
    // surface may still be attached with an earlier scene underneath, so this names
    // the render error without claiming a cause or that nothing is drawn.
    status = 'Error';
    title = 'Viewport render error';
    detail = viewport.error.message;
  } else {
    // "Not attached" is already the status; only the host's reason follows.
    status = 'Not attached';
    title = '';
    detail = viewport.reason;
  }

  const tone = viewport?.status === 'error' ? 'error' : attached ? 'ok' : project.kind === 'loading' ? 'pending' : 'idle';
  return (
    <section className="panel panel--viewport" data-region="viewport" aria-labelledby="viewport-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="viewport-title" className="panel__title">
          Viewport
        </h2>
        <span
          className={`status-pill status-pill--${tone}`}
          title={attached ? 'The engine’s view is attached and drawing here.' : undefined}
        >
          {status}
        </span>
      </header>
      <div
        ref={hostRef}
        className={`viewport-host${attached ? ' is-attached' : ''}`}
        data-viewport-host=""
        aria-describedby={!attached && detail ? 'viewport-detail' : undefined}
      >
        {attached ? null : (
          <div className={`viewport-empty viewport-empty--${tone}`}>
            {title ? <p className={project.kind === 'loading' ? 'visually-hidden' : 'viewport-empty__title'}>{title}</p> : null}
            {detail ? (
              <p id="viewport-detail" className="viewport-empty__detail">
                {detail}
              </p>
            ) : null}
          </div>
        )}
      </div>
    </section>
  );
}

/** The host clips the native surface to the island's rounded corners. */
function readRadii(element: HTMLElement): [number, number, number, number] {
  const style = getComputedStyle(element);
  const px = (value: string) => Number.parseFloat(value) || 0;
  return [
    px(style.borderTopLeftRadius),
    px(style.borderTopRightRadius),
    px(style.borderBottomRightRadius),
    px(style.borderBottomLeftRadius),
  ];
}
