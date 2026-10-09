import type { MouseEvent } from 'react';
import type { Capability } from '../bridge/contract';
import type { IconName } from '../icons/Icon';
import { Icon } from '../icons/Icon';
import { useShell } from '../shell/ShellContext';
import { ProviderChip } from './ProviderChip';

export interface PanelVisibility {
  readonly hierarchy: boolean;
  readonly dock: boolean;
  readonly inspector: boolean;
}

/**
 * Custom titlebar. The whole bar is the window's drag handle; interactive items
 * opt out. Window behavior is requested through the bridge (window.* requests)
 * and only offered when the host advertises it, so nothing here pretends to work.
 * See TITLEBAR.md for the host contract.
 */
export function Titlebar({
  panels,
  onTogglePanel,
  onShortcuts,
}: {
  panels: PanelVisibility;
  onTogglePanel: (panel: keyof PanelVisibility) => void;
  onShortcuts: (from: HTMLElement) => void;
}) {
  const { snapshot, bridge, capabilities, ask, run } = useShell();
  const chrome = snapshot?.window;
  const connection = snapshot?.connection;
  const project =
    connection?.status === 'ready'
      ? connection.project.name
      : connection?.status === 'connecting'
        ? 'Connecting…'
        : connection?.status === 'error'
          ? 'Disconnected'
          : 'No project open';
  const history = snapshot?.history;
  const canUndo = (history?.applied ?? 0) > 0;
  const canRedo = history ? history.applied < history.entries.length : false;

  const isChrome = (event: MouseEvent) =>
    !(event.target as HTMLElement).closest('button, a, input, [role="button"], [data-no-drag]');

  const onMouseDown = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0 || event.detail > 1 || !isChrome(event)) return;
    if (capabilities.has('window.drag')) void ask({ type: 'window.drag' });
  };
  const onDoubleClick = (event: MouseEvent<HTMLElement>) => {
    if (!isChrome(event)) return;
    if (capabilities.has('window.maximize')) void ask({ type: 'window.maximize' });
  };

  return (
    <header
      className="titlebar"
      data-platform={chrome?.platform ?? 'browser'}
      data-window-focused={chrome ? String(chrome.focused) : 'true'}
      onMouseDown={onMouseDown}
      onDoubleClick={onDoubleClick}
    >
      {chrome && chrome.leadingInset > 0 ? (
        <div className="titlebar__inset" style={{ width: chrome.leadingInset }} aria-hidden="true" />
      ) : null}
      <div className="titlebar__start">
        <div className="titlebar__identity">
          <img className="titlebar__logo" src="./icon.svg" alt="Incant" width={16} height={16} />
          <h1 className="titlebar__project" title={project}>
            <span className="visually-hidden">Project: </span>
            {project}
          </h1>
          {bridge?.isFixture ? (
            <span
              className="fixture-tag"
              data-no-drag=""
              tabIndex={0}
              title={`${bridge.label}. Static sample data for design review: not engine state, and edits are disabled.`}
            >
              Sample data
              <span className="visually-hidden">: {bridge.label}, not engine state; edits are disabled</span>
            </span>
          ) : null}
        </div>
        <div className="titlebar__spacer" />
      </div>

      <div className="titlebar__tools">
        <div className="titlebar__group" role="group" aria-label="History">
          <ToolButton
            icon="undo"
            label="Undo"
            hint="⌘Z / Ctrl+Z"
            keys="Meta+Z Control+Z"
            available={capabilities.has('history.undo')}
            idle={!canUndo}
            onClick={() => void run({ type: 'history.undo' })}
          />
          <ToolButton
            icon="redo"
            label="Redo"
            hint="⇧⌘Z / Ctrl+Y"
            keys="Meta+Shift+Z Control+Y"
            available={capabilities.has('history.redo')}
            idle={!canRedo}
            onClick={() => void run({ type: 'history.redo' })}
          />
        </div>
        <span className="titlebar__divider" aria-hidden="true" />
        <div className="titlebar__group" role="group" aria-label="Layout">
          <ToggleButton icon="sidebarLeft" label="Hierarchy panel" pressed={panels.hierarchy} onClick={() => onTogglePanel('hierarchy')} />
          <ToggleButton icon="panelBottom" label="Output panel" pressed={panels.dock} onClick={() => onTogglePanel('dock')} />
          <ToggleButton icon="sidebarRight" label="Inspector and agent panel" pressed={panels.inspector} onClick={() => onTogglePanel('inspector')} />
        </div>
      </div>

      <div className="titlebar__end">
        <div className="titlebar__spacer" />
        <ProviderChip />
        <button
          type="button"
          className="tool-button"
          aria-label="Keyboard shortcuts"
          aria-keyshortcuts="Shift+Slash"
          title="Keyboard shortcuts (?)"
          onClick={(event) => onShortcuts(event.currentTarget)}
        >
          <Icon name="keyboard" />
        </button>
      </div>

      {chrome?.controls === 'custom' ? <WindowControls maximized={chrome.maximized} /> : null}
      {chrome && chrome.trailingInset > 0 ? (
        <div className="titlebar__inset" style={{ width: chrome.trailingInset }} aria-hidden="true" />
      ) : null}
    </header>
  );
}

function ToolButton({
  icon,
  label,
  hint,
  keys,
  available,
  idle,
  onClick,
}: {
  icon: IconName;
  label: string;
  hint: string;
  keys: string;
  /** False when the bridge lacks the capability; the click then explains why. */
  available: boolean;
  /** True when the capability exists but there is nothing to do. */
  idle: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className="tool-button"
      aria-label={label}
      aria-disabled={!available || idle || undefined}
      aria-keyshortcuts={keys}
      title={`${label} (${hint})`}
      onClick={() => {
        if (available && idle) return;
        onClick();
      }}
    >
      <Icon name={icon} />
    </button>
  );
}

function ToggleButton({ icon, label, pressed, onClick }: { icon: IconName; label: string; pressed: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      className="tool-button tool-button--toggle"
      aria-label={label}
      aria-pressed={pressed}
      title={`${pressed ? 'Hide' : 'Show'} ${label.toLowerCase()}`}
      onClick={onClick}
    >
      <Icon name={icon} />
    </button>
  );
}

const WINDOW_BUTTONS: { capability: Capability & `window.${string}`; label: string; icon: (maximized: boolean) => IconName }[] = [
  { capability: 'window.minimize', label: 'Minimize', icon: () => 'minimize' },
  { capability: 'window.maximize', label: 'Maximize', icon: (maximized) => (maximized ? 'restore' : 'maximize') },
  { capability: 'window.close', label: 'Close', icon: () => 'close' },
];

/** Windows/Linux caption buttons. Each is rendered only if the host advertises it. */
function WindowControls({ maximized }: { maximized: boolean }) {
  const { capabilities, ask } = useShell();
  const buttons = WINDOW_BUTTONS.filter((button) => capabilities.has(button.capability));
  if (buttons.length === 0) return null;
  return (
    <div className="window-controls" role="group" aria-label="Window">
      {buttons.map((button) => {
        const label = button.capability === 'window.maximize' && maximized ? 'Restore' : button.label;
        return (
          <button
            key={button.capability}
            type="button"
            className={`window-control${button.capability === 'window.close' ? ' window-control--close' : ''}`}
            aria-label={label}
            title={label}
            onClick={() => void ask({ type: button.capability } as { type: typeof button.capability })}
          >
            <Icon name={button.icon(maximized)} size={14} />
          </button>
        );
      })}
    </div>
  );
}
