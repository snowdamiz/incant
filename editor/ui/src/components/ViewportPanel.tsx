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
  if (!viewport) {
    status = 'No engine';
    title = 'No viewport';
    detail = 'Connect the editor process to see the scene here.';
  } else if (viewport.status === 'attached') {
    status = 'Live';
    title = '';
    detail = '';
  } else if (project.kind === 'loading') {
    status = 'Waiting';
    title = 'Opening project…';
    detail = 'The viewport attaches when the project has loaded.';
  } else if (viewport.status === 'error' && project.kind === 'failed' && sameError(viewport.error, project.error)) {
    // The project failed, not the surface. The banner carries the full message.
    status = 'Error';
    title = 'No project loaded';
    detail = 'There is nothing to render until the project opens.';
  } else if (viewport.status === 'error') {
    // One state covers surface setup, scene preparation and drawing failures. The
    // surface may still be attached with an earlier scene underneath, so this names
    // the render error without claiming a cause or that nothing is drawn.
    status = 'Error';
    title = 'Viewport render error';
    detail = viewport.error.message;
  } else {
    status = 'Not attached';
    title = 'Viewport not attached';
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
        aria-describedby="viewport-detail"
      >
        {attached ? null : (
          <div className={`viewport-empty viewport-empty--${tone}`}>
            <ViewportIllustration tone={tone} />
            <p className="viewport-empty__title">{title}</p>
            <p id="viewport-detail" className="viewport-empty__detail">
              {detail}
            </p>
          </div>
        )}
      </div>
    </section>
  );
}

/**
 * A quiet picture of what the viewport holds: a ground grid running to the horizon
 * and an outlined cube. Decorative; the title and detail say the state in words.
 */
function ViewportIllustration({ tone }: { tone: 'idle' | 'pending' | 'error' | 'ok' }) {
  const horizon = 44;
  const vanish = { x: 120, y: 30 };
  const t = (132 - horizon) / (132 - vanish.y);
  const rays = Array.from({ length: 13 }, (_, i) => {
    const x = 120 + (i - 6) * 40;
    return `M${x} 132L${(x + (vanish.x - x) * t).toFixed(1)} ${horizon}`;
  }).join('');
  const rows = [50, 58, 70, 88, 113].map((y) => `M0 ${y}H240`).join('');
  return (
    <svg className={`viewport-art viewport-art--${tone}`} viewBox="0 0 240 132" width="240" height="132" aria-hidden="true" focusable="false">
      <defs>
        <linearGradient id="viewport-art-fade" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0.28" stopColor="#fff" stopOpacity="0" />
          <stop offset="1" stopColor="#fff" stopOpacity="1" />
        </linearGradient>
        <radialGradient id="viewport-art-edge" cx="0.5" cy="0.7" r="0.62">
          <stop offset="0.55" stopColor="#fff" stopOpacity="1" />
          <stop offset="1" stopColor="#fff" stopOpacity="0" />
        </radialGradient>
        <mask id="viewport-art-mask">
          <rect width="240" height="132" fill="url(#viewport-art-fade)" />
        </mask>
        <mask id="viewport-art-vignette">
          <rect width="240" height="132" fill="url(#viewport-art-edge)" />
        </mask>
      </defs>
      <g mask="url(#viewport-art-vignette)">
        <path className="viewport-art__grid" d={rays + rows} mask="url(#viewport-art-mask)" />
        <path className="viewport-art__horizon" d={`M0 ${horizon}H240`} />
      </g>
      <ellipse className="viewport-art__shadow" cx="122" cy="103" rx="30" ry="6" />
      <g className="viewport-art__cube">
        <path d="M104 74L118 66L142 66L128 74Z" className="viewport-art__face viewport-art__face--top" />
        <path d="M128 74L142 66L142 92L128 100Z" className="viewport-art__face viewport-art__face--side" />
        <path d="M104 74H128V100H104Z" className="viewport-art__face" />
      </g>
    </svg>
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
