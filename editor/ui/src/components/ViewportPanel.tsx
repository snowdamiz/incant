import { useEffect, useRef } from 'react';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';

/**
 * The area the native wgpu surface will occupy (Spike 1). The UI never draws
 * engine pixels here. When the bridge supports `viewport.bounds`, the host is
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
  const attached = viewport?.status === 'attached';
  let status: string;
  let detail: string;
  if (!viewport) {
    status = 'No engine';
    detail = 'Connect the editor process to attach a viewport.';
  } else if (viewport.status === 'attached') {
    status = 'Attached';
    detail = '';
  } else if (viewport.status === 'error') {
    status = 'Error';
    detail = viewport.error.message;
  } else {
    status = 'Not attached';
    detail = viewport.reason;
  }

  return (
    <section className="panel panel--viewport" data-region="viewport" aria-labelledby="viewport-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="viewport-title" className="panel__title">
          Viewport
        </h2>
        <span className={`status-pill${viewport?.status === 'error' ? ' status-pill--error' : attached ? ' status-pill--ok' : ''}`}>
          {status}
        </span>
      </header>
      <div
        ref={hostRef}
        className={`viewport-host${attached ? ' is-attached' : ''}`}
        data-viewport-host=""
        aria-describedby="viewport-detail"
      >
        {attached ? null : (
          <div className="viewport-empty">
            <span className="viewport-empty__tile" aria-hidden="true">
              <Icon name="viewport" size={22} />
            </span>
            <p className="viewport-empty__title">
              {viewport?.status === 'error' ? 'The viewport failed to attach' : 'Native viewport not attached'}
            </p>
            <p id="viewport-detail" className="viewport-empty__detail">
              {detail}
            </p>
            <p className="viewport-empty__note">Nothing in this area is rendered by the engine.</p>
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
