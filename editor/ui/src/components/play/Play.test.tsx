import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { StrictMode } from 'react';
import axe from 'axe-core';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import type { Invoke } from '../../../../bridge/native';
import { NativeBridge } from '../../../../bridge/native';
import { App } from '../../App';
import type { EditorBridge, PlayInput } from '../../bridge/contract';
import { createFixtureBridge } from '../../bridge/fixture';
import type { DoubleVariant } from '../../test/nativePlayDouble';
import { IDS, createNativePlayDouble, doubleRead } from '../../test/nativePlayDouble';

/**
 * The editor's Play UI against the REAL NativeBridge class (editor/bridge/native.ts),
 * whose Tauri commands are answered by the native-bridge double. Request wiring is
 * real; every Play result is simulated, so these are transport and UI tests only.
 */
const SCHEMAS = Object.fromEntries(
  ['Transform', 'Camera', 'RigidBody', 'Collider', 'Velocity', 'PointLight'].map((name) => [
    name,
    JSON.parse(readFileSync(resolve(__dirname, `../../../../../schemas/${name}.schema.json`), 'utf8')),
  ]),
);

/** The same bridge with a different capability list, as an older host would advertise. */
function withCapabilities(native: NativeBridge, capabilities: readonly string[]): EditorBridge {
  return {
    protocolVersion: 1,
    capabilities,
    label: native.label,
    isFixture: false,
    getSnapshot: () => native.getSnapshot(),
    subscribe: (listener) => native.subscribe(listener),
    dispatch: (command) => native.dispatch(command),
    request: (request) => native.request(request),
  };
}

async function open({
  variant = 'harbor',
  saved = true,
  read = true,
  capabilities,
  wrap,
  strict = false,
  mac = false,
}: {
  variant?: DoubleVariant;
  saved?: boolean;
  /** False leaves the Play status unread, as before the bridge's first read returns. */
  read?: boolean;
  capabilities?: (all: readonly string[]) => readonly string[];
  wrap?: (invoke: Invoke) => Invoke;
  /** Render inside StrictMode, as the dev build does. */
  strict?: boolean;
  /** Report a macOS native window. */
  mac?: boolean;
} = {}) {
  const double = createNativePlayDouble({ read: doubleRead(variant, SCHEMAS), saved, holdStart: true });
  const native = new NativeBridge(wrap ? wrap(double.invoke) : double.invoke);
  await native.startEngineUpdates(double.listen);
  if (read) await native.refreshPlay();
  if (mac) native.updateWindow({ platform: 'macos', controls: 'native-overlay', maximized: false, fullscreen: false, focused: true, leadingInset: 78, trailingInset: 0 });
  const dispatch = vi.spyOn(native, 'dispatch');
  const bridge = capabilities ? withCapabilities(native, capabilities(native.capabilities)) : native;
  const app = <App resolution={{ kind: 'bridge', bridge }} />;
  const view = render(strict ? <StrictMode>{app}</StrictMode> : app);
  /** One status read, as the bridge's 4 Hz poll makes. */
  const poll = () => act(async () => { await native.refreshPlay(); });
  return { double, native, dispatch, poll, view };
}
type Session = Awaited<ReturnType<typeof open>>;

const header = () => document.querySelector<HTMLElement>('.panel--viewport > .panel__header')!;
const stateText = () => header().querySelector('.status-pill')?.textContent ?? '';
const control = (name: string) => within(header()).getByRole('button', { name });
const picker = () => within(header()).getByRole('button', { name: /^Play (scene and camera|camera):/ });
const message = () => document.querySelector('.statusbar__message')?.textContent ?? '';
const runtime = () => document.querySelector<HTMLElement>('.runtime');
const runtimeRows = () =>
  [...(runtime()?.querySelectorAll('.field') ?? [])].map((row) => [
    row.querySelector('.field__label')?.textContent,
    [...row.querySelectorAll('input')].map((input) => input.value),
  ]);
const host = () => document.querySelector<HTMLElement>('[data-viewport-host]')!;
const ops = (session: Session) => session.double.commands.map((command) => `${String(command.op)}:${String(command.generation)}`);
const sent = (session: Session) => session.double.inputs.flatMap((packet) => packet.events);

async function choose(label: string) {
  fireEvent.click(picker());
  fireEvent.click(await screen.findByRole('menuitemradio', { name: label }));
}

/** A running Harbor session with the Player Camera, unsuspended, at tick 1. */
async function play(session: Session, target = 'Harbor · Player Camera') {
  await choose(target);
  fireEvent.click(control('Play'));
  await waitFor(() => expect(session.double.commands.at(-1)?.op).toBe('start'));
  await act(async () => session.double.finishStart());
  await waitFor(() => expect(session.double.status().phase).toBe('running'));
  // The first focused frame releases the worker's hold.
  session.double.frame();
  await session.poll();
  await waitFor(() => expect(stateText()).toContain('Playing'));
}

describe('native Play controls', () => {
  it('starts through the bridge with the chosen scene and camera, the session generation and the authoring revision', async () => {
    const session = await open();
    expect(stateText()).toBe('Live');
    expect(picker().getAttribute('aria-label')).toBe('Play scene and camera: Choose scene');
    await choose('Harbor · Player Camera');
    expect(picker().getAttribute('aria-label')).toBe('Play scene and camera: Harbor · Player Camera');
    fireEvent.click(control('Play'));
    await waitFor(() => expect(session.double.commands).toHaveLength(1));
    expect(session.double.commands[0]).toEqual({
      op: 'start',
      generation: 0,
      expected_revision: 41,
      options: { scene: IDS.harbor, camera: IDS.playerCamera },
    });
    // Preparing at once; Stop stays usable; nothing else acts.
    expect(stateText()).toBe('Preparing…');
    expect(control('Stop').getAttribute('aria-disabled')).toBeNull();
    expect(control('Play').getAttribute('aria-disabled')).toBe('true');
    expect(control('Step').getAttribute('aria-disabled')).toBe('true');
    await act(async () => session.double.finishStart());
    session.double.frame();
    await session.poll();
    await waitFor(() => expect(stateText()).toBe('Playing·, Harbor'));
    await waitFor(() => expect(message()).toBe('Playing Harbor.'));
    expect(control('Pause').getAttribute('aria-disabled')).toBeNull();
    // A running session has no picker: its options cannot change.
    expect(within(header()).queryByRole('button', { name: /^Play scene and camera/ })).toBeNull();
    expect(session.dispatch).not.toHaveBeenCalled();
  });

  it('uses the only scene and camera without a picker', async () => {
    const session = await open({ variant: 'single' });
    expect(within(header()).queryByRole('button', { name: /^Play (scene|camera)/ })).toBeNull();
    fireEvent.click(control('Play'));
    await waitFor(() => expect(session.double.commands).toHaveLength(1));
    expect(session.double.commands[0]).toMatchObject({ options: { scene: IDS.harbor, camera: IDS.playerCamera } });
  });

  it('asks for the scene and camera when Play needs them, and starts with the choice', async () => {
    const session = await open();
    fireEvent.click(control('Play'));
    const menu = await screen.findByRole('menu', { name: 'Scene and camera to play' });
    await waitFor(() => expect(message()).toBe('Choose a scene to play.'));
    expect(session.double.commands).toEqual([]);
    expect(within(menu).getAllByRole('menuitemradio').map((item) => item.textContent)).toEqual([
      'Harbor · Player Camera',
      'Harbor · Overview Camera',
      'Harbor · Dock Camera',
      'Title · Title Camera',
    ]);
    fireEvent.click(within(menu).getByRole('menuitemradio', { name: 'Harbor · Dock Camera' }));
    await waitFor(() => expect(session.double.commands).toHaveLength(1));
    expect(session.double.commands[0]).toMatchObject({ op: 'start', options: { scene: IDS.harbor, camera: IDS.dockCamera } });
    // Focus moves to the control that now acts on the session.
    expect(document.activeElement?.getAttribute('data-play-primary')).toBe('');
  });

  it('stops a session that is still preparing; its cancelled startup never replaces the stopped state', async () => {
    const session = await open();
    await choose('Harbor · Player Camera');
    fireEvent.click(control('Play'));
    await session.poll();
    expect(session.double.status()).toMatchObject({ generation: 1, phase: 'preparing' });
    fireEvent.click(control('Stop'));
    await waitFor(() => expect(ops(session)).toEqual(['start:0', 'stop:1']));
    // Stopped, but the cancelled startup still cooks: Play waits for it.
    await waitFor(() => expect(stateText()).toBe('Stopping…'));
    expect(control('Play').getAttribute('aria-disabled')).toBe('true');
    await waitFor(() => expect(message()).toBe('Play start cancelled.'));
    // Stop or Mod+P again while it finishes sends nothing more.
    fireEvent.keyDown(document.body, { key: 'p', code: 'KeyP', metaKey: true });
    await waitFor(() => expect(message()).toBe('Play is stopping.'));
    expect(ops(session)).toEqual(['start:0', 'stop:1']);
    // The startup's late "Play startup was cancelled" failure speaks for nothing.
    await act(async () => session.double.finishStart());
    await waitFor(() => expect(stateText()).toBe('Live'));
    expect(document.querySelector('.play-notice')).toBeNull();
    expect(message()).toBe('Play is stopping.');
    expect(control('Play').getAttribute('aria-disabled')).toBeNull();
    expect(ops(session)).toEqual(['start:0', 'stop:1']);
  });

  it('holds a Stop pressed before the new session is visible, then sends it for that session', async () => {
    const session = await open();
    await choose('Harbor · Player Camera');
    fireEvent.click(control('Play'));
    await waitFor(() => expect(ops(session)).toEqual(['start:0']));
    // No status has shown generation 1 yet; generation 0 would be refused by the worker.
    fireEvent.click(control('Stop'));
    expect(ops(session)).toEqual(['start:0']);
    expect(stateText()).toBe('Stopping…');
    expect(control('Stop').getAttribute('aria-disabled')).toBe('true');
    await session.poll();
    await waitFor(() => expect(ops(session)).toEqual(['start:0', 'stop:1']));
    await act(async () => session.double.finishStart());
    await waitFor(() => expect(stateText()).toBe('Live'));
    expect(document.querySelector('.play-notice')).toBeNull();
  });

  it('shows a refused start (unsaved project) once, in the notice, and Dismiss clears it', async () => {
    const session = await open({ saved: false });
    await choose('Harbor · Player Camera');
    fireEvent.click(control('Play'));
    const notice = await screen.findByRole('alert');
    expect(notice.textContent).toBe('Play could not start. Open a saved project to run PlayDismiss');
    expect(stateText()).toBe('Play failed');
    // The alert says it; the status line does not repeat it or keep "Preparing…".
    await waitFor(() => expect(message()).toBe(''));
    expect(control('Play').getAttribute('aria-disabled')).toBeNull();
    fireEvent.click(within(notice).getByRole('button', { name: 'Dismiss' }));
    await waitFor(() => expect(ops(session)).toEqual(['start:0', 'stop:1']));
    await waitFor(() => expect(stateText()).toBe('Live'));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('ends on a terminal runtime error with its tick and message, and Play can start again', async () => {
    const session = await open();
    await play(session);
    session.double.frames(41);
    await act(async () => session.double.fail('Native surface was lost or rejected; reopen the editor to recreate it'));
    await session.poll();
    const notice = await screen.findByRole('alert');
    expect(notice.textContent).toContain('Play stopped at tick 42. Native surface was lost or rejected');
    expect(control('Stop').getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(control('Play'));
    await waitFor(() => expect(ops(session).at(-1)).toBe('start:1'));
  });

  it('marks controls that cannot act and says why when pressed', async () => {
    renderFixture();
    for (const name of ['Play', 'Step', 'Stop']) expect(control(name).getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(control('Play'));
    await waitFor(() => expect(message()).toBe('Native Play is unavailable: the sample fixture is read-only.'));
  });

  it('explains Step and Stop while stopped, and Step while running', async () => {
    const session = await open({ variant: 'single' });
    fireEvent.click(control('Step'));
    await waitFor(() => expect(message()).toBe('Step works while Play is paused.'));
    fireEvent.click(control('Stop'));
    await waitFor(() => expect(message()).toBe('Play is not running.'));
    fireEvent.click(control('Play'));
    await act(async () => session.double.finishStart());
    session.double.frame();
    await session.poll();
    fireEvent.click(control('Step'));
    await waitFor(() => expect(message()).toBe('Pause Play to step one tick.'));
    expect(ops(session)).toEqual(['start:0']);
  });

  it('treats a missing status as waiting, not as running', async () => {
    await open({ read: false });
    expect(stateText()).toBe('Live');
    expect(control('Play').getAttribute('aria-disabled')).toBe('true');
    expect(control('Play').getAttribute('title')).toContain('Waiting for the Play status.');
  });

  it('keeps an unreadable status apart from a failed session; Stop tries, and says why it could not', async () => {
    const session = await open();
    await play(session);
    session.double.setReadable(false);
    await session.poll();
    await waitFor(() => expect(stateText()).toBe('Play unavailable'));
    expect(screen.getByRole('alert').textContent).toBe('The Play status cannot be read. Native Play worker has stopped');
    expect(control('Play').getAttribute('aria-disabled')).toBe('true');
    expect(control('Stop').getAttribute('aria-disabled')).toBeNull();
    // The bridge reads again before stopping; still unreadable, it refuses and says why.
    fireEvent.click(control('Stop'));
    await waitFor(() => expect(message()).toBe('Stop failed: Native Play worker has stopped'));
    expect(ops(session)).toEqual(['start:0']);
    // Readable again: Mod+P stops the session it last saw.
    session.double.setReadable(true);
    fireEvent.keyDown(document.body, { key: 'p', code: 'KeyP', metaKey: true });
    await waitFor(() => expect(ops(session)).toEqual(['start:0', 'stop:1']));
    await waitFor(() => expect(stateText()).toBe('Live'));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('opens and closes the scene and camera list with its button', async () => {
    await open();
    fireEvent.click(picker());
    expect(screen.getByRole('menu')).toBeTruthy();
    expect(picker().getAttribute('aria-expanded')).toBe('true');
    fireEvent.mouseDown(picker());
    fireEvent.click(picker());
    expect(screen.queryByRole('menu')).toBeNull();
    expect(picker().getAttribute('aria-expanded')).toBe('false');
    // A press elsewhere closes it too.
    fireEvent.click(picker());
    fireEvent.mouseDown(document.body);
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  });

  it('lets Play start again only after Dismiss has cleared the failed session', async () => {
    const session = await open({ saved: false });
    await choose('Harbor · Player Camera');
    fireEvent.click(control('Play'));
    await screen.findByRole('alert');
    session.double.setSaved(true);
    fireEvent.click(within(screen.getByRole('alert')).getByRole('button', { name: 'Dismiss' }));
    fireEvent.click(control('Play'));
    await waitFor(() => expect(message()).toBe('The previous session is still stopping.'));
    await waitFor(() => expect(stateText()).toBe('Live'));
    fireEvent.click(control('Play'));
    await waitFor(() => expect(ops(session)).toEqual(['start:0', 'stop:1', 'start:2']));
    expect(message()).not.toMatch(/could not start/);
  });

  it('drops a queued Stop when the start fails first, keeping its notice', async () => {
    const session = await open({ saved: false });
    await choose('Harbor · Player Camera');
    fireEvent.click(control('Play'));
    fireEvent.click(control('Stop'));
    const notice = await screen.findByRole('alert');
    expect(notice.textContent).toContain('Open a saved project to run Play');
    await session.poll();
    expect(ops(session)).toEqual(['start:0']);
    expect(stateText()).toBe('Play failed');
    expect(control('Play').getAttribute('aria-disabled')).toBeNull();
  });

  it('has no axe violations while playing with the runtime block shown', async () => {
    const session = await open();
    await play(session);
    fireEvent.click(screen.getByRole('treeitem', { name: 'Crate 01' }));
    await waitFor(() => expect(runtime()?.textContent).toContain('tick 1'));
    const results = await axe.run(session.view.container, { rules: { 'color-contrast': { enabled: false } } });
    expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
  });
});

function renderFixture() {
  return render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
}

describe('runtime inspection', () => {
  it('reads the selected entity on selection and after Pause and Step, labelled with its tick, and never edits the project', async () => {
    const session = await open();
    await play(session);
    fireEvent.click(screen.getByRole('treeitem', { name: 'Player' }));
    await waitFor(() => expect(runtime()?.querySelector('.runtime__tick')?.textContent).toBe('tick 1'));
    expect(runtimeRows()).toEqual([
      ['Translation', ['0.025', '1', '0']],
      ['Rotation', ['0', '0.7071', '0', '0.7071']],
      ['Scale', ['1', '1', '1']],
      ['Velocity', ['1.5', '0', '0']],
    ]);
    expect(session.double.commands.at(-1)).toEqual({ op: 'inspect', generation: 1, entity: IDS.player });
    // Running ticks are not inspected: the values stay those of tick 1.
    session.double.frames(59);
    await session.poll();
    expect(runtime()?.querySelector('.runtime__tick')?.textContent).toBe('tick 1');
    fireEvent.click(control('Pause'));
    await waitFor(() => expect(runtime()?.querySelector('.runtime__tick')?.textContent).toBe('tick 60'));
    expect(stateText()).toBe('Paused·, tick 60');
    expect(runtimeRows()[0]).toEqual(['Translation', ['1.5', '1', '0']]);
    fireEvent.click(control('Step'));
    await waitFor(() => expect(session.double.commands.at(-1)).toMatchObject({ op: 'step' }));
    // While the step is pending the worker refuses reads: none is sent, the values stay.
    fireEvent.click(screen.getByRole('button', { name: 'Read runtime state again' }));
    expect(session.double.commands.at(-1)).toMatchObject({ op: 'step' });
    expect(runtimeRows()[0]).toEqual(['Translation', ['1.5', '1', '0']]);
    await act(async () => session.double.frame());
    await waitFor(() => expect(runtime()?.querySelector('.runtime__tick')?.textContent).toBe('tick 61'));
    expect(runtimeRows()[0]).toEqual(['Translation', ['1.525', '1', '0']]);
    await waitFor(() => expect(message()).toBe('Stepped to tick 61.'));
    // The authored Transform below is unchanged and nothing was dispatched.
    const authored = screen.getByRole('region', { name: 'Transform' });
    expect([...authored.querySelectorAll('input')].slice(0, 3).map((input) => input.value)).toEqual(['0', '1', '0']);
    expect(session.dispatch).not.toHaveBeenCalled();
    expect([...runtime()!.querySelectorAll('input')].every((input) => input.readOnly)).toBe(true);
    expect(ops(session).filter((op) => op.startsWith('inspect'))).toHaveLength(3);
  });

  it('never lets an older read replace a newer selection or session', async () => {
    const held: { command: Record<string, unknown>; release: () => void }[] = [];
    let gate = false;
    const session = await open({
      wrap: (invoke) => async <T,>(name: string, args?: Record<string, unknown>) => {
        const command = args?.command as Record<string, unknown> | undefined;
        if (gate && name === 'play_command' && command?.op === 'inspect') await new Promise<void>((release) => held.push({ command, release }));
        return invoke<T>(name, args);
      },
    });
    await play(session);
    gate = true;
    fireEvent.click(screen.getByRole('treeitem', { name: 'Player' }));
    fireEvent.click(screen.getByRole('treeitem', { name: 'Crate 01' }));
    await waitFor(() => expect(held).toHaveLength(2));
    // Crate 01 has fallen a little by tick 1.
    await act(async () => held[1]!.release());
    await waitFor(() => expect(runtimeRows()[0]).toEqual(['Translation', ['2', '3.9986', '1']]));
    // The Player's read arrives last; the Crate stays.
    await act(async () => held[0]!.release());
    await new Promise((done) => setTimeout(done, 20));
    expect(runtimeRows()[0]).toEqual(['Translation', ['2', '3.9986', '1']]);
    // A read in flight when the session stops shows nothing afterwards.
    fireEvent.click(screen.getByRole('button', { name: 'Read runtime state again' }));
    await waitFor(() => expect(held).toHaveLength(3));
    fireEvent.click(control('Stop'));
    await waitFor(() => expect(stateText()).toBe('Live'));
    await act(async () => held[2]!.release());
    await new Promise((done) => setTimeout(done, 20));
    expect(runtime()).toBeNull();
  });

  it('says plainly when the entity is not in the running world or its scene is not playing', async () => {
    const session = await open();
    await play(session);
    session.double.frames(5);
    fireEvent.click(screen.getByRole('treeitem', { name: 'Tutorial Sign' }));
    await waitFor(() => expect(runtime()?.textContent).toContain('No runtime state. Entity is absent from this runtime world'));
    expect(runtimeRows()).toEqual([]);
    const reads = ops(session).filter((op) => op.startsWith('inspect')).length;
    fireEvent.click(screen.getByRole('treeitem', { name: 'Title Camera' }));
    await waitFor(() => expect(runtime()?.textContent).toContain('Not in the running scene. Play runs Harbor.'));
    // No read is sent for an entity of another scene, and a scene node has no runtime block.
    expect(ops(session).filter((op) => op.startsWith('inspect'))).toHaveLength(reads);
    fireEvent.click(screen.getByRole('treeitem', { name: 'Harbor' }));
    await waitFor(() => expect(runtime()).toBeNull());
  });

  it('labels runtime inspection and game input as unavailable on a host without them', async () => {
    const session = await open({ capabilities: (all) => all.filter((capability) => capability !== 'play.inspect' && capability !== 'play.input') });
    await play(session);
    fireEvent.click(screen.getByRole('treeitem', { name: 'Player' }));
    await waitFor(() => expect(runtime()?.textContent).toContain('Runtime inspection is unavailable in this editor build (play.inspect).'));
    expect(ops(session).some((op) => op.startsWith('inspect'))).toBe(false);
    expect(host().getAttribute('tabindex')).toBeNull();
    fireEvent.pointerDown(host(), { button: 0, pointerType: 'mouse' });
    await waitFor(() => expect(message()).toBe('Game input is unavailable in this editor build (play.input).'));
  });
});

describe('game input', () => {
  /** A running session whose game view was focused with a click (that press is not sent). */
  async function focused() {
    const session = await open();
    await play(session);
    expect(host().getAttribute('role')).toBe('application');
    expect(host().getAttribute('aria-label')).toBe('Game view');
    fireEvent.pointerDown(host(), { button: 0, pointerType: 'mouse', clientX: 30, clientY: 40 });
    fireEvent.pointerUp(host(), { button: 0, pointerType: 'mouse', clientX: 30, clientY: 40 });
    expect(document.activeElement).toBe(host());
    await waitFor(() => expect(sent(session)).toContainEqual({ type: 'focus', focused: true }));
    return session;
  }

  it('sends nothing until the game view is deliberately focused', async () => {
    const session = await open();
    await play(session);
    fireEvent.keyDown(document.body, { code: 'KeyW', key: 'w' });
    fireEvent.keyDown(host(), { code: 'KeyW', key: 'w' });
    await new Promise((done) => setTimeout(done, 60));
    expect(session.double.inputs).toEqual([]);
  });

  it('sends keys, buttons, position and wheel from the focused view, under the session generation', async () => {
    const session = await focused();
    expect(sent(session).some((event) => event.type === 'mouse_button')).toBe(false);
    expect(fireEvent.keyDown(host(), { code: 'KeyW', key: 'w' })).toBe(false);
    fireEvent.keyDown(host(), { code: 'KeyW', key: 'w', repeat: true });
    expect(fireEvent.keyDown(host(), { code: 'IntlBackslash', key: '\\' })).toBe(false);
    fireEvent.pointerMove(host(), { pointerType: 'mouse', clientX: 50, clientY: 60 });
    fireEvent.pointerDown(host(), { button: 2, pointerType: 'mouse', clientX: 50, clientY: 60 });
    expect(fireEvent.wheel(host(), { deltaY: 100, deltaMode: 0 })).toBe(false);
    fireEvent.pointerUp(host(), { button: 2, pointerType: 'mouse', clientX: 50, clientY: 60 });
    fireEvent.keyUp(host(), { code: 'KeyW', key: 'w' });
    const expected: PlayInput[] = [
      { type: 'key', code: 'KeyW', down: true },
      { type: 'pointer_position', position: [50, 60] },
      { type: 'mouse_button', button: 'right', down: true },
      { type: 'wheel', delta: [0, -100], unit: 'pixels' },
      { type: 'mouse_button', button: 'right', down: false },
      { type: 'key', code: 'KeyW', down: false },
    ];
    await waitFor(() => expect(sent(session).filter((event) => event.type !== 'focus' && !(event.type === 'pointer_position' && event.position[0] === 30))).toEqual(expected));
    expect(session.double.inputs.every((packet) => packet.generation === 1 && packet.events.length <= 128)).toBe(true);
    expect(session.double.refusedInputs).toEqual([]);
  });

  it('splits a burst into packets of at most 128 events that the bridge accepts, in order', async () => {
    const session = await focused();
    for (let index = 0; index < 150; index += 1) {
      fireEvent.keyDown(host(), { code: index % 2 ? 'KeyA' : 'KeyB' });
      fireEvent.keyUp(host(), { code: index % 2 ? 'KeyA' : 'KeyB' });
    }
    await waitFor(() => expect(sent(session).filter((event) => event.type === 'key')).toHaveLength(300));
    const packets = session.double.inputs.map((packet) => packet.events.length);
    expect(Math.max(...packets)).toBe(128);
    expect(packets.every((size) => size <= 128)).toBe(true);
    expect(session.double.refusedInputs).toEqual([]);
    expect(sent(session).filter((event) => event.type === 'key').slice(0, 2)).toEqual([
      { type: 'key', code: 'KeyB', down: true },
      { type: 'key', code: 'KeyB', down: false },
    ]);
  });

  it('works in StrictMode, as in the development build', async () => {
    const session = await open({ strict: true });
    await play(session);
    fireEvent.pointerDown(host(), { button: 0, pointerType: 'mouse' });
    fireEvent.keyDown(host(), { code: 'KeyW', key: 'w' });
    await waitFor(() => expect(sent(session)).toContainEqual({ type: 'key', code: 'KeyW', down: true }));
  });

  it('leaves Command combinations to the editor, and releases a game key pressed before them', async () => {
    const session = await focused();
    fireEvent.keyDown(host(), { code: 'KeyP', key: 'p' });
    // Cmd+S: macOS sends no release for S while Command is held, so S is not the game's.
    expect(fireEvent.keyDown(host(), { code: 'KeyS', key: 's', metaKey: true })).toBe(true);
    // P, held in the game, is released with Ctrl down: the release still reaches the game.
    fireEvent.keyUp(host(), { code: 'KeyP', key: 'p', ctrlKey: true });
    await waitFor(() => expect(sent(session)).toContainEqual({ type: 'key', code: 'KeyP', down: false }));
    expect(sent(session)).not.toContainEqual({ type: 'key', code: 'KeyS', down: true });
  });

  it('gives keys such as ? to the game, not to editor shortcuts, while keeping the Play shortcuts', async () => {
    const session = await focused();
    fireEvent.keyDown(host(), { code: 'Slash', key: '?', shiftKey: true });
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(sent(session)).toContainEqual({ type: 'key', code: 'Slash', down: true }));
    // Shift+Mod+P is the editor's: the session pauses and input stops.
    fireEvent.keyDown(host(), { code: 'KeyP', key: 'p', metaKey: true, shiftKey: true });
    await waitFor(() => expect(stateText()).toBe('Paused·, tick 1'));
    expect(sent(session)).not.toContainEqual({ type: 'key', code: 'KeyP', down: true });
    const count = sent(session).length;
    fireEvent.keyDown(host(), { code: 'KeyA', key: 'a' });
    await new Promise((done) => setTimeout(done, 60));
    expect(sent(session)).toHaveLength(count);
    // Paused, keys are the editor's again.
    fireEvent.keyDown(host(), { code: 'Slash', key: '?', shiftKey: true });
    expect(screen.getByRole('dialog', { name: 'Keyboard shortcuts' })).toBeTruthy();
  });

  it('releases held input when focus leaves with F6, and sends no more', async () => {
    const session = await focused();
    fireEvent.keyDown(host(), { code: 'KeyD', key: 'd' });
    await waitFor(() => expect(sent(session)).toContainEqual({ type: 'key', code: 'KeyD', down: true }));
    fireEvent.keyDown(host(), { code: 'F6', key: 'F6' });
    expect(document.activeElement).not.toBe(host());
    await waitFor(() => expect(sent(session).at(-1)).toEqual({ type: 'focus', focused: false }));
    expect(sent(session)).not.toContainEqual({ type: 'key', code: 'F6', down: true });
    const count = sent(session).length;
    fireEvent.keyUp(host(), { code: 'KeyD', key: 'd' });
    await new Promise((done) => setTimeout(done, 60));
    expect(sent(session)).toHaveLength(count);
  });

  it('moves focus to Play when the session ends in the game view, and the view stops taking focus', async () => {
    const session = await focused();
    fireEvent.keyDown(host(), { code: 'KeyP', key: 'p', metaKey: true });
    await waitFor(() => expect(stateText()).toBe('Live'));
    expect(host().getAttribute('tabindex')).toBeNull();
    expect(document.activeElement?.getAttribute('aria-label')).toBe('Play');
    await waitFor(() => expect(message()).toBe('Play stopped. Runtime changes were discarded.'));
    expect(ops(session).at(-1)).toBe('stop:1');
  });
});

describe('Play shortcuts', () => {
  it('play, pause, step, resume and stop by keyboard, by physical key', async () => {
    const session = await open({ variant: 'single' });
    const press = (init: KeyboardEventInit) => fireEvent.keyDown(document.body, { key: 'p', code: 'KeyP', ...init });
    press({ metaKey: true });
    await waitFor(() => expect(ops(session)).toEqual(['start:0']));
    await act(async () => session.double.finishStart());
    session.double.frame();
    await session.poll();
    press({ ctrlKey: true, shiftKey: true });
    await waitFor(() => expect(message()).toBe('Paused at tick 1.'));
    // Option changes the character on macOS; the physical key still counts.
    press({ metaKey: true, altKey: true, key: 'π' });
    await waitFor(() => expect(ops(session).at(-1)).toBe('step:1'));
    // A second press while the step is pending is not lost silently.
    press({ metaKey: true, altKey: true, key: 'π' });
    await waitFor(() => expect(message()).toBe('Wait for the current Play request to finish.'));
    await act(async () => session.double.frame());
    await waitFor(() => expect(stateText()).toBe('Paused·, tick 2'));
    await waitFor(() => expect(message()).toBe('Stepped to tick 2.'));
    press({ metaKey: true, shiftKey: true });
    await waitFor(() => expect(ops(session).at(-1)).toBe('resume:1'));
    press({ metaKey: true });
    await waitFor(() => expect(ops(session).at(-1)).toBe('stop:1'));
    expect(press({ metaKey: true, repeat: true })).toBe(false);
    expect(ops(session).filter((op) => op.startsWith('start'))).toHaveLength(1);
  });

  it('leave Ctrl+P in macOS text fields to the text field', async () => {
    const session = await open({ variant: 'single', mac: true });
    const filter = screen.getByRole('searchbox', { name: /Filter/ });
    filter.focus();
    expect(fireEvent.keyDown(filter, { key: 'p', code: 'KeyP', ctrlKey: true })).toBe(true);
    await new Promise((done) => setTimeout(done, 30));
    expect(session.double.commands).toEqual([]);
    expect(fireEvent.keyDown(filter, { key: 'p', code: 'KeyP', metaKey: true })).toBe(false);
    await waitFor(() => expect(ops(session)).toEqual(['start:0']));
  });

  it('leave AltGr typing in text fields and open dialogs alone', async () => {
    const session = await open({ variant: 'single' });
    const filter = screen.getByRole('searchbox', { name: /Filter/ });
    filter.focus();
    expect(fireEvent.keyDown(filter, { key: 'π', code: 'KeyP', ctrlKey: true, altKey: true })).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'p', code: 'KeyP', metaKey: true });
    await new Promise((done) => setTimeout(done, 30));
    expect(session.double.commands).toEqual([]);
  });
});
