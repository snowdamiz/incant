import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { IconName } from '../../icons/Icon';
import { Icon } from '../../icons/Icon';
import { targetLabel, targetOptions } from '../../play/model';
import type { Play } from '../../play/PlayContext';
import { usePlay } from '../../play/PlayContext';
import { useShell } from '../../shell/ShellContext';
import type { MenuItem } from '../Menu';
import { ContextMenu } from '../Menu';
import { comboText, usePlatform } from '../ShortcutsDialog';

/** Ticks are counts: grouped for reading, never abbreviated. */
export function tickText(tick: number): string {
  return tick.toLocaleString('en-US');
}

export interface PlayStatusView {
  readonly tone: 'ok' | 'pending' | 'paused' | 'error';
  readonly text: string;
  /** Quiet words after the state: the tick while paused, the scene when there are several. */
  readonly detail?: string;
  readonly busy?: boolean;
  readonly title?: string;
}

/** A short-lived suspension (the first frame after Start) is not worth a word. */
const SUSPENDED_DELAY_MS = 400;

function useSettled(flag: boolean, delay: number): boolean {
  const [settled, setSettled] = useState(false);
  useEffect(() => {
    if (!flag) {
      setSettled(false);
      return;
    }
    const timer = window.setTimeout(() => setSettled(true), delay);
    return () => window.clearTimeout(timer);
  }, [flag, delay]);
  return flag && settled;
}

/**
 * The Play state for the viewport header, or null while no session exists (the
 * viewport's own state, such as Live, is shown then). Generation, sequence and
 * discarded-tick counters are deliberately not shown.
 */
export function usePlayStatusView(): PlayStatusView | null {
  const { state, feed, playingScene, scenes } = usePlay();
  const suspended = useSettled(state === 'suspended', SUSPENDED_DELAY_MS);
  const scene = scenes.length > 1 && playingScene ? { detail: playingScene } : {};
  switch (state) {
    case 'preparing':
      return { tone: 'pending', text: 'Preparing…', busy: true };
    case 'stopping':
      return { tone: 'pending', text: 'Stopping…', busy: true };
    case 'running':
      return { tone: 'ok', text: 'Playing', ...scene };
    case 'suspended':
      return suspended
        ? { tone: 'pending', text: 'Suspended', ...scene, title: 'Play is held while the editor window is in the background or the viewport is hidden. It continues when they are back.' }
        : { tone: 'ok', text: 'Playing', ...scene };
    case 'paused':
      return { tone: 'paused', text: 'Paused', detail: `tick ${tickText(feed.session?.tick ?? 0)}` };
    case 'error':
      return { tone: 'error', text: 'Play failed' };
    case 'unavailable':
      return { tone: 'error', text: 'Play unavailable' };
    default:
      return null;
  }
}

export function PlayStatusPill({ view }: { view: PlayStatusView }) {
  return (
    <span className={`status-pill status-pill--${view.tone}${view.busy ? ' is-busy' : ''}`} title={view.title}>
      {view.busy ? <span className="spinner spinner--small" aria-hidden="true" /> : null}
      {view.text}
      {view.detail ? (
        <span className="status-pill__detail">
          <span aria-hidden="true">·</span>
          <span className="visually-hidden">, </span>
          {view.detail}
        </span>
      ) : null}
    </span>
  );
}

/** Platform key text and the matching aria-keyshortcuts for the three Play shortcuts. */
function useShortcutText() {
  const platform = usePlatform();
  const mac = platform === 'mac';
  return {
    play: { text: comboText(['Mod', 'P'], platform), aria: 'Meta+P Control+P' },
    pause: { text: comboText(mac ? ['Shift', 'Mod', 'P'] : ['Ctrl', 'Shift', 'P'], platform), aria: 'Meta+Shift+P Control+Shift+P' },
    step: { text: comboText(mac ? ['Alt', 'Mod', 'P'] : ['Ctrl', 'Alt', 'P'], platform), aria: 'Meta+Alt+P Control+Alt+P' },
  };
}

/**
 * Play, Pause or Resume, Step and Stop, in the viewport header, with the scene and
 * camera picker before them when there is a choice to make. Buttons that cannot act
 * stay focusable, are marked unavailable and say why when pressed.
 */
export function PlayControls() {
  const play = usePlay();
  const keys = useShortcutText();
  const { state, busy } = play;
  const running = state === 'running' || state === 'suspended';
  const paused = state === 'paused';
  const waiting = busy !== null ? 'Wait for the current Play request to finish.' : null;
  const primary = running
    ? { icon: 'pause' as const, label: 'Pause', keys: keys.pause, disabled: waiting, run: play.togglePause }
    : paused
      ? { icon: 'play' as const, label: 'Resume', keys: keys.pause, disabled: waiting, run: play.togglePause }
      : { icon: 'play' as const, label: 'Play', keys: keys.play, disabled: play.startBlocked, run: () => play.start() };
  const step = paused ? waiting : running ? 'Pause Play to step one tick.' : 'Step works while Play is paused.';
  // While the status cannot be read, Stop is still offered for a session last seen running.
  const stop = play.stoppable ? null : state === 'stopping' ? 'Play is stopping.' : 'Play is not running.';
  const picker = play.supported && play.target.choosable && (state === 'stopped' || state === 'error' || state === 'loading');
  return (
    <div className="play-controls">
      {picker ? <TargetPicker play={play} /> : null}
      <div className="play-controls__buttons" role="group" aria-label="Play controls">
        <TransportButton icon={primary.icon} label={primary.label} keys={primary.keys} disabled={primary.disabled} onRun={primary.run} primary />
        <TransportButton icon="step" label="Step" keys={keys.step} disabled={step} onRun={play.step} />
        <TransportButton icon="stop" label="Stop" keys={keys.play} disabled={stop} onRun={play.stop} />
      </div>
    </div>
  );
}

function TransportButton({
  icon,
  label,
  keys,
  disabled,
  onRun,
  primary = false,
}: {
  icon: IconName;
  label: string;
  keys: { text: string; aria: string };
  /** Why the button cannot act now, or null. */
  disabled: string | null;
  onRun: () => void;
  primary?: boolean;
}) {
  const { announce } = useShell();
  return (
    <button
      type="button"
      className="tool-button play-button"
      aria-label={label}
      aria-keyshortcuts={keys.aria}
      aria-disabled={disabled !== null ? true : undefined}
      title={disabled !== null ? `${label} (${keys.text}). ${disabled}` : `${label} (${keys.text})`}
      data-play-primary={primary ? '' : undefined}
      onClick={() => (disabled !== null ? announce(disabled) : onRun())}
    >
      <Icon name={icon} />
    </button>
  );
}

/**
 * The scene and camera Play starts with, shown only when there is a choice: several
 * scenes, or several cameras in the scene. One list of radio items; choosing while Play
 * waits for the choice starts it.
 */
function TargetPicker({ play }: { play: Play }) {
  const button = useRef<HTMLButtonElement>(null);
  const [menu, setMenu] = useState<{ at: { x: number; y: number }; forPlay: boolean } | null>(null);
  const open = useCallback((forPlay: boolean) => {
    const rect = button.current?.getBoundingClientRect();
    if (rect) setMenu({ at: { x: rect.left, y: rect.bottom + 4 }, forPlay });
  }, []);
  const requests = useRef(play.pickerRequests);
  useEffect(() => {
    if (play.pickerRequests === requests.current) return;
    requests.current = play.pickerRequests;
    open(true);
  }, [play.pickerRequests, open]);
  const options = useMemo(() => targetOptions(play.scenes), [play.scenes]);
  const { target } = play;
  const label = targetLabel(target, play.scenes.length);
  const items: MenuItem[] = options.map((option) => ({
    id: option.key,
    label: option.label,
    checked: option.scene.id === target.scene?.id && (option.camera?.id ?? null) === (target.camera?.id ?? null) && target.missing === null,
    ...(option.camera ? {} : { note: 'No camera' }),
    onSelect: () => {
      play.choose(option.scene.id, option.camera?.id ?? null);
      if (menu?.forPlay) {
        play.start({ scene: option.scene, camera: option.camera });
        document.querySelector<HTMLElement>('[data-play-primary]')?.focus();
      }
    },
  }));
  const name = `Play ${play.scenes.length > 1 ? 'scene and camera' : 'camera'}: ${label}`;
  return (
    <>
      <button
        ref={button}
        type="button"
        className={`play-target${target.missing ? ' is-missing' : ''}`}
        aria-haspopup="menu"
        aria-expanded={menu !== null}
        aria-label={name}
        title={name}
        onClick={() => (menu ? setMenu(null) : open(false))}
      >
        <Icon name={target.camera || target.missing === 'camera' ? 'camera' : 'scene'} size={14} className="play-target__icon" />
        <span className="play-target__label">{label}</span>
        <Icon name="chevron" size={12} className="play-target__chevron" />
      </button>
      {menu ? (
        <ContextMenu
          label={play.scenes.length > 1 ? 'Scene and camera to play' : 'Camera to play'}
          items={items}
          at={menu.at}
          onClose={(restore) => {
            setMenu(null);
            if (restore) button.current?.focus();
          }}
          onUnavailable={() => undefined}
          anchor={button}
        />
      ) : null}
    </>
  );
}

/**
 * The failed session's message under the header, until Play starts again or it is
 * dismissed; or why the Play status cannot be read. Announced once as an alert.
 */
export function PlayNotice() {
  const play = usePlay();
  const session = play.feed.session;
  if (play.state === 'error' && session) {
    const title = session.scene === null ? 'Play could not start.' : `Play stopped at tick ${tickText(session.tick ?? 0)}.`;
    return (
      <div className="play-notice" role="alert">
        <Icon name="error" size={14} className="play-notice__icon" />
        <p className="play-notice__text">
          <strong>{title}</strong> {session.error ?? 'The engine gave no reason.'}
        </p>
        <button type="button" className="button button--ghost button--small play-notice__dismiss" onClick={play.dismiss}>
          Dismiss
        </button>
      </div>
    );
  }
  if (play.state === 'unavailable' && play.feed.transport) {
    return (
      <div className="play-notice" role="alert">
        <Icon name="error" size={14} className="play-notice__icon" />
        <p className="play-notice__text">
          <strong>The Play status cannot be read.</strong> {play.feed.transport.message}
        </p>
      </div>
    );
  }
  return null;
}
