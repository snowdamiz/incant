import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import type { ReactNode } from 'react';
import type { EditorBridge, PlayEntity, PlayRequest, Ulid } from '../bridge/contract';
import { unavailableMessage } from '../bridge/resolve';
import { projectState } from '../shell/projectState';
import { useShell } from '../shell/ShellContext';
import type { PlayCamera, PlayChoice, PlayFeed, PlayScene, PlayState, PlayTarget, RuntimeTarget } from './model';
import { NO_CHOICE, isActive, playFeed, playScenes, playState, resolveTarget, runtimeTarget, sameFeed } from './model';

/** One read of the selected entity in the running world. Never written to the project. */
export interface Inspection {
  readonly entity: Ulid;
  readonly generation: number;
  readonly loading: boolean;
  readonly value: PlayEntity | null;
  readonly error: string | null;
}

export type PlayStep = 'pause' | 'resume' | 'step';

export interface Play {
  /** The host advertises play.control. */
  readonly supported: boolean;
  readonly feed: PlayFeed;
  readonly state: PlayState;
  readonly scenes: readonly PlayScene[];
  readonly target: PlayTarget;
  /** The name of the scene that is playing, when it is in the hierarchy. */
  readonly playingScene: string | null;
  readonly choose: (scene: Ulid, camera: Ulid | null) => void;
  /** Increments when Play needs a choice first; the picker opens. */
  readonly pickerRequests: number;
  /** Why Play cannot start now, or null. */
  readonly startBlocked: string | null;
  /** A pause, resume or step request in flight. */
  readonly busy: PlayStep | null;
  /**
   * Stop can act: a session exists, or one was last seen running while the status cannot
   * be read (the bridge then retries the read and stops it, or says why it cannot).
   */
  readonly stoppable: boolean;
  /** Starts the resolved target, or `chosen` when a choice was just made for this Play. */
  readonly start: (chosen?: { readonly scene: PlayScene; readonly camera: PlayCamera | null }) => void;
  readonly togglePause: () => void;
  readonly step: () => void;
  readonly stop: () => void;
  /** Play or Stop, for the one shortcut that does both. */
  readonly playOrStop: () => void;
  /** Clears a failed session's error (a Stop of that session). */
  readonly dismiss: () => void;
  readonly runtime: RuntimeTarget;
  readonly inspection: Inspection | null;
  /** Reads the selected entity again. */
  readonly inspect: () => void;
  readonly inputError: string | null;
  readonly reportInputError: (message: string | null) => void;
}

const PlayContext = createContext<Play | null>(null);

const EMPTY_FEED: PlayFeed = { session: null, transport: null };

/** Play status, selected so that running ticks (read at 4 Hz) do not re-render. */
function usePlayFeed(bridge: EditorBridge | null): PlayFeed {
  const cache = useRef<PlayFeed>(EMPTY_FEED);
  const subscribe = useCallback((listener: () => void) => (bridge ? bridge.subscribe(listener) : () => undefined), [bridge]);
  const read = useCallback(() => {
    const snapshot = bridge?.getSnapshot();
    const next = playFeed(snapshot?.play, snapshot?.playError);
    if (!sameFeed(next, cache.current)) cache.current = next;
    return cache.current;
  }, [bridge]);
  return useSyncExternalStore(subscribe, read);
}

interface StartRecord {
  /** The generation the start was sent from; its session is the next one. */
  readonly from: number;
  /** Stop was pressed, or a later session appeared: this start no longer speaks. */
  superseded: boolean;
  /** Stop was pressed before the new session's generation was known. */
  stopQueued: boolean;
}

export function PlayProvider({ children }: { children: ReactNode }) {
  const { bridge, capabilities, snapshot, request, announce, selection } = useShell();
  const supported = capabilities.has('play.control');
  const canInspect = capabilities.has('play.inspect');
  const feed = usePlayFeed(bridge);
  const session = feed.session;
  /** The freshest status, for decisions after an await. */
  const latest = useCallback(() => bridge?.getSnapshot().play, [bridge]);

  const project = projectState(snapshot);
  const tree = snapshot?.hierarchy.status === 'ready' ? snapshot.hierarchy.value : null;
  const scenes = useMemo(() => playScenes(tree), [tree]);
  const [choice, setChoice] = useState<PlayChoice>(NO_CHOICE);
  const target = useMemo(() => resolveTarget(scenes, choice), [scenes, choice]);
  const [pickerRequests, setPickerRequests] = useState(0);
  const choose = useCallback((scene: Ulid, camera: Ulid | null) => {
    setChoice((current) => ({ scene, cameras: camera ? { ...current.cameras, [scene]: camera } : current.cameras }));
  }, []);

  const startRecord = useRef<StartRecord | null>(null);
  const [startedFrom, setStartedFrom] = useState<number | null>(null);
  const [stopping, setStopping] = useState(false);
  const [busy, setBusy] = useState<PlayStep | null>(null);
  const state = playState({ supported, feed, startedFrom, stopping });
  const lastSeenActive = session !== null && (session.phase === 'preparing' || session.phase === 'running' || session.phase === 'paused');
  const stoppable = isActive(state) || (state === 'unavailable' && lastSeenActive && !stopping);

  let startBlocked: string | null = null;
  if (!supported) startBlocked = unavailableMessage('play.control', bridge);
  else if (state === 'unavailable') startBlocked = `The Play status cannot be read: ${feed.transport?.message ?? 'unknown error'}`;
  else if (state === 'loading') startBlocked = 'Waiting for the Play status.';
  else if (project.kind === 'none') startBlocked = 'Open a project to play.';
  else if (project.kind === 'loading') startBlocked = 'Wait for the project to open.';
  else if (project.kind === 'failed') startBlocked = 'The project could not be opened.';
  else if (scenes.length === 0) startBlocked = 'This project has no scene to play.';
  else if (state === 'preparing') startBlocked = 'Play is starting.';
  else if (state === 'stopping') startBlocked = 'The previous session is still stopping.';

  // ---- Runtime inspection: on selection, and after a pause or step. Never polled. ----
  const runtime = useMemo(() => runtimeTarget(tree, selection, session?.scene ?? null), [tree, selection, session?.scene]);
  const runtimeId = runtime.kind === 'entity' ? runtime.id : null;
  const inspectable = canInspect && !feed.transport && session !== null && (session.phase === 'running' || session.phase === 'paused');
  const [inspection, setInspection] = useState<Inspection | null>(null);
  const inspectToken = useRef(0);
  const inspectEntity = useCallback(
    async (entity: Ulid) => {
      const status = latest();
      if (!status) return;
      const generation = status.generation;
      const token = ++inspectToken.current;
      setInspection((previous) => ({
        entity,
        generation,
        loading: true,
        // A refresh keeps the values it replaces on screen, still labelled with their tick.
        value: previous?.entity === entity && previous.generation === generation ? previous.value : null,
        error: null,
      }));
      const result = await request({ type: 'play.inspect', entity }, { quiet: true });
      // A newer read, another selection or another session owns the panel now.
      if (token !== inspectToken.current || latest()?.generation !== generation) return;
      if (result.ok && result.playEntity?.entity === entity && result.playEntity.generation === generation) {
        setInspection({ entity, generation, loading: false, value: result.playEntity, error: null });
      } else {
        const error = result.ok ? 'The engine returned no runtime state for this entity.' : result.error.message;
        setInspection({ entity, generation, loading: false, value: null, error });
      }
    },
    [latest, request],
  );
  // While a step is pending the worker refuses every request but Stop; the step's own
  // completion reads the selection again.
  const stepping = busy === 'step';
  useEffect(() => {
    if (!inspectable || runtimeId === null) {
      inspectToken.current += 1;
      setInspection(null);
      return;
    }
    if (!stepping) void inspectEntity(runtimeId);
  }, [inspectable, runtimeId, session?.generation, inspectEntity, stepping]);
  const inspect = useCallback(() => {
    if (inspectable && runtimeId !== null && !stepping) void inspectEntity(runtimeId);
  }, [inspectable, runtimeId, inspectEntity, stepping]);
  const inspectLatest = useRef(inspect);
  useLayoutEffect(() => {
    inspectLatest.current = inspect;
  });

  // ---- Controls ----
  const sendStop = useCallback(async () => {
    // A session stopped while preparing never ran, so nothing of it was discarded.
    const ran = latest()?.phase !== 'preparing';
    setStopping(true);
    const result = await request({ type: 'play.stop' }, { quiet: true });
    setStopping(false);
    if (!result.ok) announce(`Stop failed: ${result.error.message}`);
    else announce(ran ? 'Play stopped. Runtime changes were discarded.' : 'Play start cancelled.');
  }, [announce, latest, request]);

  const start = useCallback(async (chosen?: { readonly scene: PlayScene; readonly camera: PlayCamera | null }) => {
    if (startBlocked !== null) {
      announce(startBlocked);
      return;
    }
    if (isActive(state)) return;
    if (!chosen && (target.missing !== null || !target.scene)) {
      setPickerRequests((count) => count + 1);
      announce(target.missing === 'camera' ? `Choose a camera for ${target.scene?.name ?? 'the scene'}.` : 'Choose a scene to play.');
      return;
    }
    const scene = chosen?.scene ?? target.scene;
    const camera = chosen ? chosen.camera : target.camera;
    const status = latest();
    if (startRecord.current) {
      // A stopped startup can still be finishing before its reply arrives.
      announce('The previous Play startup is still finishing.');
      return;
    }
    if (!scene || !status) return;
    const record: StartRecord = { from: status.generation, superseded: false, stopQueued: false };
    startRecord.current = record;
    setStartedFrom(record.from);
    announce(`Preparing ${scene.name}…`);
    const request_: PlayRequest = { type: 'play.start', scene: scene.id, ...(camera ? { camera: camera.id } : {}) };
    const result = await request(request_, { quiet: true });
    startRecord.current = null;
    setStartedFrom(null);
    const now = latest();
    if (record.stopQueued) {
      // The reply can publish the new session before any status read did.
      record.stopQueued = false;
      if (now && now.generation > record.from && (now.phase === 'preparing' || now.phase === 'running' || now.phase === 'paused')) void sendStop();
      else setStopping(false);
    }
    // A start that was stopped, or overtaken by a later session, does not speak for it.
    if (record.superseded || (now !== undefined && now.generation > record.from + 1)) return;
    if (result.ok) announce(`Playing ${scene.name}.`);
    // A failure the session itself reports is shown by the error notice, once.
    else if (!(now?.generation === record.from + 1 && now.phase === 'error')) announce(`Play could not start: ${result.error.message}`);
  }, [announce, latest, request, sendStop, startBlocked, state, target]);

  const stop = useCallback(() => {
    if (stopping) return;
    const record = startRecord.current;
    if (record && latest()?.generation === record.from) {
      // The session being prepared has no known generation yet: stop it once it does.
      record.superseded = true;
      record.stopQueued = true;
      setStopping(true);
      return;
    }
    if (!stoppable) {
      announce(state === 'stopping' ? 'Play is stopping.' : 'Play is not running.');
      return;
    }
    if (record) record.superseded = true;
    void sendStop();
  }, [announce, latest, sendStop, state, stoppable, stopping]);

  // A queued Stop goes out as soon as a status shows the new session, unless that
  // session already ended (a failed start keeps its notice).
  const generation = session?.generation;
  useEffect(() => {
    const record = startRecord.current;
    if (!record?.stopQueued || generation === undefined || generation <= record.from) return;
    record.stopQueued = false;
    const phase = latest()?.phase;
    if (phase === 'preparing' || phase === 'running' || phase === 'paused') void sendStop();
    else setStopping(false);
  }, [generation, latest, sendStop]);

  const runStep = useCallback(
    async (kind: PlayStep) => {
      if (busy || stopping) {
        announce(stopping ? 'Play is stopping.' : 'Wait for the current Play request to finish.');
        return;
      }
      const before = latest()?.generation;
      setBusy(kind);
      const result = await request({ type: kind === 'pause' ? 'play.pause' : kind === 'resume' ? 'play.resume' : 'play.step' }, { quiet: true });
      setBusy(null);
      const now = latest();
      // Stopped or replaced meanwhile: the outcome belongs to a session that is gone.
      if (!now || now.generation !== before) return;
      if (!result.ok) {
        announce(`${kind === 'pause' ? 'Pause' : kind === 'resume' ? 'Resume' : 'Step'} failed: ${result.error.message}`);
        return;
      }
      if (kind === 'resume') announce('Resumed.');
      else announce(`${kind === 'pause' ? 'Paused at' : 'Stepped to'} tick ${now.tick.toLocaleString('en-US')}.`);
      if (kind !== 'resume') inspectLatest.current();
    },
    [announce, busy, latest, request, stopping],
  );

  const togglePause = useCallback(() => {
    if (state === 'running' || state === 'suspended') void runStep('pause');
    else if (state === 'paused') void runStep('resume');
    else announce('Play is not running.');
  }, [announce, runStep, state]);

  const step = useCallback(() => {
    if (state === 'paused') void runStep('step');
    else if (state === 'running' || state === 'suspended') announce('Pause Play to step one tick.');
    else announce('Step works while Play is paused.');
  }, [announce, runStep, state]);

  const playOrStop = useCallback(() => {
    if (stoppable || startRecord.current) stop();
    else void start();
  }, [start, stoppable, stop]);

  // Clearing a failed session is a Stop of it: Play waits until it is done.
  const dismiss = useCallback(async () => {
    if (stopping) return;
    setStopping(true);
    const result = await request({ type: 'play.stop' }, { quiet: true });
    setStopping(false);
    if (!result.ok) announce(`Could not clear the Play error: ${result.error.message}`);
  }, [announce, request, stopping]);

  // A failed session or an unreadable status is stated by the notice, which is an alert;
  // an earlier "Preparing…" or "Playing" in the status line would now contradict it.
  const failed = state === 'error' || state === 'unavailable';
  useEffect(() => {
    if (failed) announce('');
  }, [announce, failed]);

  // ---- Game input errors: announced once per distinct message, cleared per session. ----
  const [inputError, setInputError] = useState<string | null>(null);
  useEffect(() => setInputError(null), [generation]);
  useEffect(() => {
    if (inputError) announce(`Game input failed: ${inputError}`);
  }, [announce, inputError]);

  const playingScene = session?.scene ? (scenes.find((scene) => scene.id === session.scene)?.name ?? null) : null;

  const value = useMemo<Play>(
    () => ({
      supported,
      feed,
      state,
      scenes,
      target,
      playingScene,
      choose,
      pickerRequests,
      startBlocked,
      busy,
      stoppable,
      start: (chosen) => void start(chosen),
      togglePause,
      step,
      stop,
      playOrStop,
      dismiss: () => void dismiss(),
      runtime,
      inspection,
      inspect,
      inputError,
      reportInputError: setInputError,
    }),
    [supported, feed, state, scenes, target, playingScene, choose, pickerRequests, startBlocked, busy, stoppable, start, togglePause, step, stop, playOrStop, dismiss, runtime, inspection, inspect, inputError],
  );
  return <PlayContext.Provider value={value}>{children}</PlayContext.Provider>;
}

export function usePlay(): Play {
  const play = useContext(PlayContext);
  if (!play) throw new Error('usePlay must be used inside PlayProvider');
  return play;
}

/** For views that also render without Play, such as the Inspector on its own. */
export function useOptionalPlay(): Play | null {
  return useContext(PlayContext);
}
