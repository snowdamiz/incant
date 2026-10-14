import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import axe from 'axe-core';
import { describe, expect, it } from 'vitest';
import type { EngineRead, Invoke } from '../../../bridge/native';
import { NativeBridge } from '../../../bridge/native';
import { App } from '../App';
import type { EditorBridge, WindowChrome } from '../bridge/contract';
import { createFixtureBridge } from '../bridge/fixture';
import { createNativePlayDouble, doubleRead } from '../test/nativePlayDouble';

/**
 * Save and the engine pin against the REAL NativeBridge class (editor/bridge/native.ts).
 * Its Tauri commands are answered by a scripted TEST DOUBLE that holds each file request
 * until the test answers it. Request wiring, revision checks and receipt validation are
 * the production bridge's; every host answer is invented, so these are transport and UI
 * tests. Nothing here writes or reads a project file.
 */
const PROJECT = '01ARZ3NDEKTSV4RRFFQ69G5FAX';
const SCENE = '01ARZ3NDEKTSV4RRFFQ69G5FAV';
const OTHER_PROJECT = '01ARZ3NDEKTSV4RRFFQ69G5FAW';
const REVIEW = '01ARZ3NDEKTSV4RRFFQ69G5FAY';
const SECOND_REVIEW = '01ARZ3NDEKTSV4RRFFQ69G5FB1';
const PIN_TX = '01ARZ3NDEKTSV4RRFFQ69G5FAZ';
const BEFORE = 'a1'.repeat(32);
const AFTER = 'b2'.repeat(32);
/** Untrusted text: markup and a hidden direction override must stay visible as text. */
const CANDIDATE = '{\n  "engine_version": "=0.4.2",\n  "name": "Harbor <img src=x onerror=alert(1)>‮"\n}\n';

/** An invented project with one user edit per revision. */
function engine(revision: number, overrides: Partial<EngineRead> = {}): EngineRead {
  const history = Array.from({ length: revision }, (_, index) => ({
    id: `01ARZ3NDEKTSV4RRFFQ69H${String(index).padStart(4, '0')}`,
    description: `Edit ${index + 1}`,
    actor: { origin: 'user' as const, actor: 'editor', model: null, conversation_id: null },
  }));
  return {
    project: { id: PROJECT, name: 'Harbor', scenes: { [SCENE]: { id: SCENE, name: 'Docks', entities: {} } } },
    revision,
    can_redo: false,
    history,
    applied: revision,
    schemas: {},
    console: [],
    viewport_error: null,
    project_file: { available: true },
    ...overrides,
  };
}

const reviewAt = (revision: number, id = REVIEW) => ({
  operation: 'review_pin',
  review: {
    review_id: id,
    expected_revision: revision,
    source_sha256: BEFORE,
    candidate_sha256: AFTER,
    candidate_text: CANDIDATE,
    command: { op: 'set_engine_version', requirement: '=0.4.2' },
  },
});
const savedAt = (revision: number, changed: boolean) => ({
  operation: 'save',
  receipt: { revision, changed, source_sha256: BEFORE, candidate_sha256: changed ? AFTER : BEFORE },
});

const chrome = (platform: 'macos' | 'windows'): WindowChrome => ({
  platform,
  controls: platform === 'macos' ? 'native-overlay' : 'custom',
  maximized: false,
  fullscreen: false,
  focused: true,
  leadingInset: platform === 'macos' ? 78 : 0,
  trailingInset: 0,
});

/** Lets every promise the host's answer settles run, then React commit. */
const settle = () => act(async () => { await new Promise((done) => setTimeout(done, 0)); });

interface FileCall {
  readonly request: Record<string, unknown>;
  /** Answers as the host does: the response and the read that follows it. */
  answer(response: unknown, next?: EngineRead): Promise<void>;
  /** Fails as the host does: Tauri rejects with the command's error text. */
  refuse(error: unknown): Promise<void>;
}

/** One host connection: the real bridge over a double whose file requests wait for the test. */
async function createHost(read: EngineRead, platform: 'macos' | 'windows' = 'macos') {
  let current = read;
  const files: FileCall[] = [];
  const invoke: Invoke = <T,>(name: string, args?: Record<string, unknown>): Promise<T> => {
    if (name === 'engine_read') return Promise.resolve(current as T);
    if (name !== 'engine_project_file') return Promise.resolve(null as T);
    return new Promise<T>((resolve, reject) => {
      files.push({
        request: args?.request as Record<string, unknown>,
        answer: async (response, next = current) => {
          current = next;
          resolve({ response, snapshot: next } as T);
          await settle();
        },
        refuse: async (error) => {
          reject(error);
          await settle();
        },
      });
    });
  };
  const bridge = new NativeBridge(invoke);
  await bridge.start();
  bridge.updateWindow(chrome(platform));
  /** Another edit, project or host change, published as the host's change event is. */
  const publish = async (next: EngineRead) => {
    current = next;
    await act(async () => {
      await bridge.start();
    });
  };
  return { bridge, files, publish };
}

async function openEditor({ read = engine(3), platform = 'macos' }: { read?: EngineRead; platform?: 'macos' | 'windows' } = {}) {
  const host = await createHost(read, platform);
  const view = render(<App resolution={{ kind: 'bridge', bridge: host.bridge }} />);
  return { ...host, view };
}

/** Another invented project, opened later in the same window. */
const lighthouse = (revision = 7, overrides: Partial<EngineRead> = {}) =>
  engine(revision, { project: { id: OTHER_PROJECT, name: 'Lighthouse', scenes: {} }, ...overrides });

/**
 * TEST DOUBLE: the same bridge, except that a request rejects with `next.error` once it is
 * set, as a faulty bridge would, instead of resolving to a result.
 */
function rejecting(native: NativeBridge) {
  const next: { error: unknown } = { error: undefined };
  const bridge: EditorBridge = {
    protocolVersion: native.protocolVersion,
    get capabilities() {
      return native.capabilities;
    },
    label: native.label,
    isFixture: false,
    getSnapshot: () => native.getSnapshot(),
    subscribe: (listener) => native.subscribe(listener),
    dispatch: (command) => native.dispatch(command),
    request: (hostRequest) => {
      const { error } = next;
      if (error === undefined) return native.request(hostRequest);
      next.error = undefined;
      return Promise.reject(error);
    },
  };
  return { bridge, next };
}

const projectButton = () => screen.getByRole('button', { name: /^Project: Harbor/ });
const message = () => document.querySelector('.statusbar__message')?.textContent ?? '';
const requests = (files: readonly FileCall[]) => files.map((call) => call.request);
const openMenu = () => {
  fireEvent.click(projectButton());
  return screen.getByRole('menu', { name: 'Project file' });
};
const choose = (name: RegExp) => fireEvent.click(within(openMenu()).getByRole('menuitem', { name }));
const pinDialog = () => screen.getByRole('dialog', { name: 'Pin engine version' });
const footer = (name: string) => within(pinDialog()).getByRole('button', { name });
const describedBy = (element: HTMLElement) =>
  (element.getAttribute('aria-describedby') ?? '')
    .split(' ')
    .map((id) => document.getElementById(id)?.textContent ?? '')
    .join(' ');
const press = (init: KeyboardEventInit, target: Element = document.body) => fireEvent.keyDown(target, { key: 's', code: 'KeyS', ...init });

async function expectNoAxeViolations(container: HTMLElement) {
  // jsdom has no layout or computed colours; contrast is checked by tokens.test.ts and the browser review.
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } });
  expect(results.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`)).toEqual([]);
}

describe('project file menu', () => {
  it('opens from the project name and saves through project.save, reporting the receipt', async () => {
    const { files, view } = await openEditor();
    const menu = openMenu();
    expect(projectButton().getAttribute('aria-expanded')).toBe('true');
    const items = within(menu).getAllByRole('menuitem');
    expect(items.map((item) => item.textContent)).toEqual(['Save⌘S', 'Pin engine version…']);
    expect(items.every((item) => item.getAttribute('aria-disabled') === null)).toBe(true);
    await expectNoAxeViolations(view.container);

    fireEvent.click(items[0]!);
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(projectButton());
    expect(requests(files)).toEqual([{ operation: 'save', expected_revision: 3 }]);
    // While the host saves, the title says so and a second Save is the same request.
    expect(projectButton().querySelector('.spinner')).not.toBeNull();
    expect(projectButton().getAttribute('aria-label')).toBe('Project: Harbor, saving');
    choose(/^Save/);
    expect(press({ metaKey: true })).toBe(false);
    expect(files).toHaveLength(1);

    await files[0]!.answer(savedAt(3, true));
    await waitFor(() => expect(message()).toBe('Saved revision 3 to the project file.'));
    expect(projectButton().querySelector('.spinner')).toBeNull();

    press({ metaKey: true });
    expect(requests(files)).toEqual([{ operation: 'save', expected_revision: 3 }, { operation: 'save', expected_revision: 3 }]);
    await files[1]!.answer(savedAt(3, false));
    await waitFor(() => expect(message()).toBe('Nothing to save: the project file already matches revision 3.'));
  });

  it('says why it cannot save, without a request, on the sample fixture, an older host and with no engine', async () => {
    const fixture = render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.click(screen.getByRole('button', { name: /^Project: Dock Prototype/ }));
    const reason = 'Project file access is unavailable: the sample fixture is read-only.';
    for (const item of screen.getAllByRole('menuitem')) {
      expect(item.getAttribute('aria-disabled')).toBe('true');
      expect(item.textContent).toContain(`unavailable: ${reason}`);
    }
    fireEvent.click(screen.getByRole('menuitem', { name: /^Pin engine version/ }));
    await waitFor(() => expect(message()).toBe(reason));
    expect(screen.queryByRole('dialog')).toBeNull();
    fixture.unmount();

    const { files } = await openEditor({ read: engine(3, { project_file: { available: false } }), platform: 'windows' });
    expect(press({ ctrlKey: true })).toBe(false);
    await waitFor(() => expect(message()).toBe('Project file access is unavailable in this editor build (project.file).'));
    choose(/^Save/);
    expect(files).toEqual([]);
  });

  it('with no engine, the title is not a menu and the shortcut says why', async () => {
    render(<App resolution={{ kind: 'none' }} />);
    expect(screen.queryByRole('button', { name: /^Project:/ })).toBeNull();
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('No project open');
    press({ ctrlKey: true });
    await waitFor(() => expect(message()).toBe('Project file access is unavailable: no engine is connected.'));
  });
});

describe('Save shortcut', () => {
  it('is ⌘S on macOS, from anywhere outside a dialog, text fields included', async () => {
    const { files } = await openEditor({ platform: 'macos' });
    // Not Save on macOS: Control+S, Save As, Option (a different character) and a held key.
    expect(press({ ctrlKey: true })).toBe(true);
    expect(press({ metaKey: true, shiftKey: true })).toBe(true);
    expect(press({ metaKey: true, altKey: true, key: 'ß' })).toBe(true);
    expect(press({ metaKey: true, repeat: true })).toBe(false);
    expect(files).toEqual([]);

    const filter = screen.getByRole('searchbox', { name: 'Filter hierarchy by name' });
    filter.focus();
    expect(press({ metaKey: true }, filter)).toBe(false);
    expect(requests(files)).toEqual([{ operation: 'save', expected_revision: 3 }]);
    expect(document.activeElement).toBe(filter);
    await files[0]!.answer(savedAt(3, false));
  });

  it('is Ctrl+S on Windows and Linux, by the S key on a layout without Latin letters', async () => {
    const { files } = await openEditor({ platform: 'windows' });
    expect(press({ metaKey: true })).toBe(true);
    // AltGr arrives as Ctrl+Alt and types a character.
    expect(press({ ctrlKey: true, altKey: true, key: 'ś' })).toBe(true);
    expect(files).toEqual([]);
    expect(press({ ctrlKey: true, key: 'ы' })).toBe(false);
    expect(requests(files)).toEqual([{ operation: 'save', expected_revision: 3 }]);
    await files[0]!.answer(savedAt(3, true));
    expect(press({ ctrlKey: true, key: 'S' })).toBe(false);
    expect(files).toHaveLength(2);
    await files[1]!.answer(savedAt(3, false));
  });

  it('is taken but does nothing while a dialog is open, and is listed with the shortcuts', async () => {
    const { files } = await openEditor({ platform: 'windows' });
    fireEvent.click(screen.getByRole('button', { name: 'Keyboard shortcuts' }));
    const shortcuts = screen.getByRole('dialog', { name: 'Keyboard shortcuts' });
    expect(within(shortcuts).getByRole('cell', { name: 'General Save the project Ctrl S' })).toBeTruthy();
    expect(press({ ctrlKey: true }, within(shortcuts).getByRole('searchbox'))).toBe(false);
    expect(files).toEqual([]);
    expect(screen.getByRole('dialog', { name: 'Keyboard shortcuts' })).toBeTruthy();
  });

  it('leaves Ctrl+S to the focused game on Windows and Linux, and keeps ⌘S the editor’s on macOS', async () => {
    for (const platform of ['windows', 'macos'] as const) {
      const files: unknown[] = [];
      const double = createNativePlayDouble({ read: { ...doubleRead('single', {}), project_file: { available: true } }, window: chrome(platform) });
      const invoke: Invoke = <T,>(name: string, args?: Record<string, unknown>) => {
        if (name !== 'engine_project_file') return double.invoke<T>(name, args);
        files.push(args?.request);
        return new Promise<T>(() => undefined);
      };
      const bridge = new NativeBridge(invoke);
      await bridge.startEngineUpdates(double.listen);
      await bridge.refreshPlay();
      bridge.updateWindow(chrome(platform));
      const view = render(<App resolution={{ kind: 'bridge', bridge }} />);
      const header = document.querySelector<HTMLElement>('.panel--viewport > .panel__header')!;
      fireEvent.click(within(header).getByRole('button', { name: 'Play' }));
      await waitFor(() => expect(double.status().phase).toBe('running'));
      double.frame();
      await act(async () => {
        await bridge.refreshPlay();
      });
      const host = document.querySelector<HTMLElement>('[data-viewport-host]')!;
      await waitFor(() => expect(host.getAttribute('aria-label')).toBe('Game view'));
      fireEvent.pointerDown(host, { button: 0, pointerType: 'mouse', clientX: 30, clientY: 40 });
      expect(document.activeElement).toBe(host);
      await waitFor(() => expect(double.inputs.flatMap((packet) => packet.events)).toContainEqual({ type: 'focus', focused: true }));

      expect(press(platform === 'macos' ? { metaKey: true } : { ctrlKey: true }, host)).toBe(false);
      const keys = () => double.inputs.flatMap((packet) => packet.events).filter((event) => event.type === 'key');
      if (platform === 'windows') {
        await waitFor(() => expect(keys()).toContainEqual({ type: 'key', code: 'KeyS', down: true }));
        expect(files).toEqual([]);
      } else {
        expect(files).toEqual([{ operation: 'save', expected_revision: 41 }]);
        await new Promise((done) => setTimeout(done, 60));
        expect(keys()).not.toContainEqual({ type: 'key', code: 'KeyS', down: true });
      }
      view.unmount();
    }
  });
});

describe('failed saves', () => {
  it('stay in view in the host’s words until dismissed or a later save succeeds', async () => {
    const { files } = await openEditor({ platform: 'windows' });
    press({ ctrlKey: true });
    // Tauri rejects with the native error's text; the bridge keeps it as written.
    await files[0]!.refuse('The project file changed outside the editor.');
    const region = screen.getByRole('region', { name: 'Project status' });
    const alert = within(region).getByRole('alert');
    expect(alert.textContent).toContain('Save result unknown. The project file changed outside the editor.');
    expect(alert.textContent).toContain('engine.request');
    expect(message()).not.toMatch(/Saved/);

    const dismiss = within(alert).getByRole('button', { name: 'Dismiss' });
    dismiss.focus();
    fireEvent.click(dismiss);
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    expect(document.activeElement).toBe(projectButton());

    press({ ctrlKey: true });
    await files[1]!.refuse({ code: 'engine.request', message: 'Wait for the current engine operation.' });
    expect(within(screen.getByRole('region', { name: 'Project status' })).getByRole('alert').textContent).toContain('Wait for the current engine operation.');
    press({ ctrlKey: true });
    expect(requests(files)).toHaveLength(3);
    await files[2]!.answer(savedAt(3, true));
    await waitFor(() => expect(message()).toBe('Saved revision 3 to the project file.'));
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
  });

  it('a receipt the bridge cannot verify is a failure, never a save', async () => {
    const { files } = await openEditor();
    press({ metaKey: true });
    // The production bridge refuses a receipt for a revision it did not send.
    await files[0]!.answer(savedAt(9, true));
    const alert = within(screen.getByRole('region', { name: 'Project status' })).getByRole('alert');
    expect(alert.textContent).toContain('Save result unknown. Invalid project file response.');
    expect(message()).not.toMatch(/Saved/);
  });

  it('a success without its receipt is reported as unknown, never as saved', async () => {
    // TEST DOUBLE: a host that answers ok but returns no typed receipt.
    const bridge: EditorBridge = {
      ...createFixtureBridge('sample'),
      isFixture: false,
      label: 'Test double',
      capabilities: ['project.file'],
      request: () => Promise.resolve({ ok: true }),
    };
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    press({ ctrlKey: true });
    const region = await screen.findByRole('region', { name: 'Project status' });
    expect(within(region).getByRole('alert').textContent).toContain(
      'Save result unknown. The host reported success but returned no save receipt.',
    );
    expect(message()).not.toMatch(/Saved/);
  });
});

describe('engine pin review', () => {
  it('shows the host’s review, pins with only its review ID, and returns focus', async () => {
    const { files, view } = await openEditor();
    choose(/^Pin engine version/);
    const dialog = pinDialog();
    expect(requests(files)).toEqual([{ operation: 'review_pin', expected_revision: 3 }]);
    expect(describedBy(dialog)).toBe('Preparing the review…');
    expect(footer('Pin').getAttribute('aria-disabled')).toBe('true');
    // The safe answer has focus, never Pin.
    expect(document.activeElement).toBe(footer('Cancel'));
    await expectNoAxeViolations(view.container);

    await files[0]!.answer(reviewAt(3));
    expect(describedBy(dialog)).toBe('Harbor will require engine =0.4.2.Reviewed at revision 3. The project file changes only when you pin.');
    expect(footer('Pin').getAttribute('aria-disabled')).toBeNull();
    expect(document.activeElement).toBe(footer('Cancel'));

    fireEvent.click(within(dialog).getByRole('button', { name: 'Review details' }));
    const facts = [...dialog.querySelectorAll('.pin__facts dt, .pin__facts dd')].map((cell) => cell.textContent);
    expect(facts).toEqual(['Command', 'set_engine_version', 'Review ID', REVIEW, 'Current SHA-256', BEFORE, 'Pinned SHA-256', AFTER]);
    const text = within(dialog).getByLabelText(/^Pinned file/);
    expect(text.tagName).toBe('PRE');
    expect(text.textContent).toBe(CANDIDATE.replace('‮', '\\u202E'));
    expect(dialog.querySelector('img')).toBeNull();
    await expectNoAxeViolations(view.container);

    fireEvent.click(footer('Pin'));
    fireEvent.click(footer('Pinning…'));
    expect(requests(files)).toEqual([
      { operation: 'review_pin', expected_revision: 3 },
      { operation: 'apply_pin', expected_revision: 3, review_id: REVIEW },
    ]);
    expect(footer('Pinning…').getAttribute('aria-disabled')).toBe('true');
    expect(footer('Close')).toBeTruthy();

    await files[1]!.answer({ operation: 'apply_pin', transaction_id: PIN_TX, revision: 4 }, engine(4));
    expect(describedBy(dialog)).toBe('Pinned. Harbor now requires engine =0.4.2 (revision 4).');
    expect(within(dialog).queryByRole('button', { name: /^Pin/ })).toBeNull();
    expect(document.activeElement).toBe(footer('Close'));

    fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(projectButton());
    expect(files).toHaveLength(2);
  });

  it('keeps keyboard focus inside the dialog and closes on Escape', async () => {
    const { files } = await openEditor();
    const button = projectButton();
    button.focus();
    fireEvent.click(button);
    const item = within(screen.getByRole('menu')).getByRole('menuitem', { name: 'Save' });
    expect(document.activeElement).toBe(item);
    fireEvent.keyDown(item, { key: 'ArrowDown' });
    fireEvent.keyDown(document.activeElement!, { key: 'Enter' });
    const dialog = pinDialog();
    await files[0]!.answer(reviewAt(3));
    const details = within(dialog).getByRole('button', { name: 'Review details' });
    const pin = footer('Pin');
    pin.focus();
    fireEvent.keyDown(pin, { key: 'Tab' });
    expect(document.activeElement).toBe(details);
    fireEvent.keyDown(details, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(pin);
    fireEvent.keyDown(pin, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(button);
    expect(requests(files)).toEqual([{ operation: 'review_pin', expected_revision: 3 }]);
  });

  it('retires a review the project has moved past and asks for a fresh one, never reusing the old ID', async () => {
    const { files, publish } = await openEditor();
    choose(/^Pin engine version/);
    await files[0]!.answer(reviewAt(3));
    footer('Pin');

    // An Undo elsewhere (revision 4)...
    await publish({ ...engine(3), revision: 4, applied: 2, can_redo: true });
    const dialog = pinDialog();
    expect(within(dialog).getByRole('alert').textContent).toBe(
      'This review can no longer be used. The project changed after this review. Review again before pinning.',
    );
    expect(within(dialog).queryByRole('button', { name: 'Pin' })).toBeNull();
    // ...and a Redo (revision 5) return History to exactly what was reviewed. That does
    // not revive the review: the revision has still moved on.
    await publish({ ...engine(3), revision: 5 });
    expect(within(dialog).queryByRole('button', { name: 'Pin' })).toBeNull();
    expect(within(dialog).getByRole('alert').textContent).toContain('The project changed after this review.');

    fireEvent.click(footer('Review again'));
    expect(document.activeElement).toBe(footer('Cancel'));
    await files[1]!.answer(reviewAt(5, SECOND_REVIEW));
    fireEvent.click(footer('Pin'));
    expect(requests(files)).toEqual([
      { operation: 'review_pin', expected_revision: 3 },
      { operation: 'review_pin', expected_revision: 5 },
      { operation: 'apply_pin', expected_revision: 5, review_id: SECOND_REVIEW },
    ]);
  });

  it('retires the review when another project opens or file access is withdrawn', async () => {
    const other = await openEditor();
    choose(/^Pin engine version/);
    await other.files[0]!.answer(reviewAt(3));
    await other.publish(engine(3, { project: { id: OTHER_PROJECT, name: 'Lighthouse', scenes: {} } }));
    expect(within(pinDialog()).getByRole('alert').textContent).toBe(
      'Harbor is no longer the open project. Lighthouse is open now. Review again before pinning.',
    );
    expect(within(pinDialog()).queryByRole('button', { name: 'Pin' })).toBeNull();
    other.view.unmount();

    const withdrawn = await openEditor();
    choose(/^Pin engine version/);
    await withdrawn.files[0]!.answer(reviewAt(3));
    await withdrawn.publish(engine(3, { project_file: { available: false } }));
    const dialog = pinDialog();
    expect(within(dialog).getByRole('alert').textContent).toBe(
      'This review can no longer be used. Project file access is unavailable in this editor build (project.file).',
    );
    expect(within(dialog).queryByRole('button', { name: /^(Pin|Review again)$/ })).toBeNull();
    expect(withdrawn.files).toHaveLength(1);
  });

  it('shows a refused review and a refused pin in the host’s words, and never retries by itself', async () => {
    const { files, view } = await openEditor();
    choose(/^Pin engine version/);
    await files[0]!.refuse('engine pin review applies only to a legacy project without an engine declaration');
    const dialog = pinDialog();
    const refusal = within(dialog).getByRole('alert');
    expect(refusal.textContent).toBe(
      'No review was returned. engine pin review applies only to a legacy project without an engine declarationengine.request',
    );
    expect(describedBy(dialog)).toBe(refusal.textContent);
    expect(within(dialog).queryByRole('button', { name: 'Pin' })).toBeNull();
    await expectNoAxeViolations(view.container);
    await new Promise((done) => setTimeout(done, 60));
    expect(files).toHaveLength(1);

    fireEvent.click(footer('Review again'));
    await files[1]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    await files[2]!.refuse('document revision conflict: expected 3, current 4');
    expect(within(dialog).getByRole('alert').textContent).toBe(
      'Engine pin result unknown. document revision conflict: expected 3, current 4engine.requestA new review is needed to try again.',
    );
    expect(within(dialog).queryByRole('button', { name: 'Pin' })).toBeNull();
    expect(document.activeElement).toBe(footer('Close'));
    await new Promise((done) => setTimeout(done, 60));
    expect(requests(files).map((request) => request.operation)).toEqual(['review_pin', 'review_pin', 'apply_pin']);
  });

  it('reports a pin that finishes after its dialog closed', async () => {
    const { files } = await openEditor();
    choose(/^Pin engine version/);
    await files[0]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    fireEvent.keyDown(pinDialog(), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    // The menu waits for the pin; it does not start a second request.
    choose(/^Save/);
    await waitFor(() => expect(message()).toBe('The engine pin is being applied.'));
    await files[1]!.refuse('project source conflict');
    const alert = within(screen.getByRole('region', { name: 'Project status' })).getByRole('alert');
    expect(alert.textContent).toContain('Engine pin result unknown. project source conflict');

    choose(/^Pin engine version/);
    await files[2]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    fireEvent.click(footer('Close'));
    await files[3]!.answer({ operation: 'apply_pin', transaction_id: PIN_TX, revision: 4 }, engine(4));
    await waitFor(() => expect(message()).toBe('Pinned to engine =0.4.2 at revision 4.'));
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
  });

  it('sends one file request at a time and drops a review whose dialog was closed', async () => {
    const { files } = await openEditor();
    press({ metaKey: true });
    openMenu();
    const pin = screen.getByRole('menuitem', { name: /^Pin engine version/ });
    expect(pin.textContent).toContain('unavailable: The project is being saved.');
    fireEvent.click(pin);
    expect(screen.queryByRole('dialog')).toBeNull();
    await files[0]!.answer(savedAt(3, false));

    choose(/^Pin engine version/);
    fireEvent.keyDown(pinDialog(), { key: 'Escape' });
    press({ metaKey: true });
    await waitFor(() => expect(message()).toBe('An engine pin review is in progress.'));
    expect(requests(files).map((request) => request.operation)).toEqual(['save', 'review_pin']);
    await files[1]!.answer(reviewAt(3));
    expect(screen.queryByRole('dialog')).toBeNull();
    press({ metaKey: true });
    expect(requests(files).map((request) => request.operation)).toEqual(['save', 'review_pin', 'save']);
    await files[2]!.answer(savedAt(3, false));
  });
});

describe('answers that arrive after another project opened', () => {
  it('drops a Save answer when another project opens while it is pending', async () => {
    const { files, publish } = await openEditor();
    press({ metaKey: true });
    expect(projectButton().querySelector('.spinner')).not.toBeNull();

    await publish(lighthouse());
    // The project open now has no request of its own: no spinner, and its Save is free.
    const title = screen.getByRole('button', { name: /^Project: Lighthouse/ });
    expect(title.getAttribute('aria-label')).toBe('Project: Lighthouse');
    expect(title.querySelector('.spinner')).toBeNull();

    // The host answers Harbor's save with its current read (Lighthouse's); the bridge
    // refuses that pairing. Neither that nor a host refusal is reported in Lighthouse.
    await files[0]!.answer(savedAt(3, true));
    await new Promise((done) => setTimeout(done, 60));
    expect(message()).toBe('');
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();

    press({ metaKey: true });
    expect(requests(files)).toEqual([
      { operation: 'save', expected_revision: 3 },
      { operation: 'save', expected_revision: 7 },
    ]);
    await files[1]!.answer(savedAt(7, false));
    await waitFor(() => expect(message()).toBe('Nothing to save: the project file already matches revision 7.'));
  });

  it('drops a Save receipt or refusal when the host connection is replaced while it is pending', async () => {
    const harbor = await createHost(engine(3));
    const other = await createHost(lighthouse());
    const view = render(<App resolution={{ kind: 'bridge', bridge: harbor.bridge }} />);
    press({ metaKey: true });
    view.rerender(<App resolution={{ kind: 'bridge', bridge: other.bridge }} />);
    expect(screen.getByRole('button', { name: /^Project: Lighthouse/ }).querySelector('.spinner')).toBeNull();

    // Harbor's host saves and sends a valid receipt; it is not Lighthouse's.
    await harbor.files[0]!.answer(savedAt(3, true));
    await new Promise((done) => setTimeout(done, 60));
    expect(message()).toBe('');
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();

    // The same for a refusal that arrives late.
    press({ metaKey: true });
    view.rerender(<App resolution={{ kind: 'bridge', bridge: harbor.bridge }} />);
    await other.files[0]!.refuse('The project file changed outside the editor.');
    await new Promise((done) => setTimeout(done, 60));
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    expect(message()).toBe('');
    expect(requests(harbor.files)).toEqual([{ operation: 'save', expected_revision: 3 }]);
    expect(requests(other.files)).toEqual([{ operation: 'save', expected_revision: 7 }]);
  });

  it('shows a pin still in flight for a project that is no longer open as unknown, never as the open one’s', async () => {
    const { files, publish } = await openEditor();
    choose(/^Pin engine version/);
    await files[0]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    expect(requests(files).at(-1)).toEqual({ operation: 'apply_pin', expected_revision: 3, review_id: REVIEW });

    await publish(lighthouse());
    const dialog = pinDialog();
    expect(within(dialog).getByRole('alert').textContent).toBe(
      'Harbor is no longer the open project. Lighthouse is open now. Whether Harbor was pinned is not shown here. Review again before pinning.',
    );
    expect(describedBy(dialog)).toBe(within(dialog).getByRole('alert').textContent);
    expect(within(dialog).queryByRole('button', { name: /^(Pin|Pinning…)$/ })).toBeNull();
    expect(dialog.textContent).not.toContain('will require engine');

    // The host answers for Harbor with Lighthouse's read; nothing of it reaches Lighthouse.
    await files[1]!.answer({ operation: 'apply_pin', transaction_id: PIN_TX, revision: 4 });
    await new Promise((done) => setTimeout(done, 60));
    expect(dialog.textContent).not.toMatch(/Pinned|not applied|unknown\./);
    expect(message()).toBe('');
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();

    fireEvent.click(footer('Review again'));
    expect(requests(files).at(-1)).toEqual({ operation: 'review_pin', expected_revision: 7 });
    await files[2]!.answer(reviewAt(7, SECOND_REVIEW));
    expect(describedBy(dialog)).toBe('Lighthouse will require engine =0.4.2.Reviewed at revision 7. The project file changes only when you pin.');
    fireEvent.click(footer('Pin'));
    expect(requests(files).at(-1)).toEqual({ operation: 'apply_pin', expected_revision: 7, review_id: SECOND_REVIEW });
  });

  it('does not report, in the project open now, a pin answered after its dialog closed', async () => {
    // Same connection, another project: Harbor's pin is refused late, as the bridge refuses
    // an answer paired with Lighthouse's read.
    const same = await openEditor();
    choose(/^Pin engine version/);
    await same.files[0]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    fireEvent.click(footer('Close'));
    expect(screen.queryByRole('dialog')).toBeNull();
    await same.publish(lighthouse());
    await same.files[1]!.answer({ operation: 'apply_pin', transaction_id: PIN_TX, revision: 4 });
    await new Promise((done) => setTimeout(done, 60));
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    expect(message()).toBe('');
    same.view.unmount();

    // A replaced connection: Harbor's host pins and answers with a valid receipt.
    const harbor = await createHost(engine(3));
    const other = await createHost(lighthouse());
    const view = render(<App resolution={{ kind: 'bridge', bridge: harbor.bridge }} />);
    choose(/^Pin engine version/);
    await harbor.files[0]!.answer(reviewAt(3));
    fireEvent.click(footer('Pin'));
    fireEvent.click(footer('Close'));
    view.rerender(<App resolution={{ kind: 'bridge', bridge: other.bridge }} />);
    await harbor.files[1]!.answer({ operation: 'apply_pin', transaction_id: PIN_TX, revision: 4 }, engine(4));
    await new Promise((done) => setTimeout(done, 60));
    expect(message()).toBe('');
    expect(screen.queryByRole('region', { name: 'Project status' })).toBeNull();
    expect(other.files).toEqual([]);
  });

  it('cannot apply an earlier connection’s review once the host connection is replaced', async () => {
    const harbor = await createHost(engine(3));
    const other = await createHost(lighthouse());
    const view = render(<App resolution={{ kind: 'bridge', bridge: harbor.bridge }} />);
    choose(/^Pin engine version/);
    await harbor.files[0]!.answer(reviewAt(3));
    expect(footer('Pin').getAttribute('aria-disabled')).toBeNull();

    view.rerender(<App resolution={{ kind: 'bridge', bridge: other.bridge }} />);
    const dialog = pinDialog();
    expect(within(dialog).getByRole('alert').textContent).toBe(
      'Harbor is no longer the open project. Lighthouse is open now. Review again before pinning.',
    );
    expect(within(dialog).queryByRole('button', { name: 'Pin' })).toBeNull();

    // A reconnection to the same project is said as such; its review still cannot be used.
    const again = await createHost(engine(3));
    fireEvent.keyDown(dialog, { key: 'Escape' });
    view.rerender(<App resolution={{ kind: 'bridge', bridge: harbor.bridge }} />);
    choose(/^Pin engine version/);
    await harbor.files[1]!.answer(reviewAt(3, SECOND_REVIEW));
    view.rerender(<App resolution={{ kind: 'bridge', bridge: again.bridge }} />);
    expect(within(pinDialog()).getByRole('alert').textContent).toBe(
      'The editor reconnected to Harbor after this review was requested. Review again before pinning.',
    );
    expect(within(pinDialog()).queryByRole('button', { name: 'Pin' })).toBeNull();

    fireEvent.click(footer('Review again'));
    expect(requests(again.files)).toEqual([{ operation: 'review_pin', expected_revision: 3 }]);
    // No review was ever applied, on any connection.
    for (const host of [harbor, other, again]) expect(requests(host.files).some((request) => request.operation === 'apply_pin')).toBe(false);
  });
});

describe('requests that reject instead of answering', () => {
  it('Save: an unknown result in the rejection’s own words and code, and the request slot freed', async () => {
    const native = (await createHost(engine(3))).bridge;
    const { bridge, next } = rejecting(native);
    render(<App resolution={{ kind: 'bridge', bridge }} />);

    next.error = new Error('IPC channel closed');
    press({ metaKey: true });
    const region = await screen.findByRole('region', { name: 'Project status' });
    expect(within(region).getByRole('alert').textContent).toBe('Save result unknown. IPC channel closedrequest.rejectedDismiss');
    expect(projectButton().querySelector('.spinner')).toBeNull();
    expect(projectButton().getAttribute('aria-label')).toBe('Project: Harbor');
    expect(message()).not.toMatch(/Saved/);

    next.error = { code: 'host.closed', message: 'The editor process exited.' };
    press({ metaKey: true });
    await waitFor(() =>
      expect(within(screen.getByRole('region', { name: 'Project status' })).getByRole('alert').textContent).toBe(
        // A structured rejection keeps its own code and words, and is still no result.
        'Save result unknown. The editor process exited.host.closedDismiss',
      ),
    );
  });

  it('review and pin: leave their pending state with a failure, never a success', async () => {
    const host = await createHost(engine(3));
    const { bridge, next } = rejecting(host.bridge);
    render(<App resolution={{ kind: 'bridge', bridge }} />);

    next.error = 'Bridge transport closed';
    choose(/^Pin engine version/);
    const dialog = pinDialog();
    await waitFor(() => expect(within(dialog).getByRole('alert').textContent).toBe('No review was returned. Bridge transport closedrequest.rejected'));
    expect(dialog.getAttribute('aria-busy')).toBeNull();
    expect(dialog.querySelector('.spinner')).toBeNull();
    expect(host.files).toEqual([]);

    fireEvent.click(footer('Review again'));
    await host.files[0]!.answer(reviewAt(3));
    next.error = new TypeError('Failed to fetch');
    fireEvent.click(footer('Pin'));
    await waitFor(() =>
      expect(within(dialog).getByRole('alert').textContent).toBe(
        'Engine pin result unknown. Failed to fetchrequest.rejectedA new review is needed to try again.',
      ),
    );
    expect(within(dialog).queryByRole('button', { name: /^(Pin|Pinning…)$/ })).toBeNull();
    expect(dialog.getAttribute('aria-busy')).toBeNull();
    expect(document.activeElement).toBe(footer('Close'));
    // The rejected pin never reached the host, and nothing claims it was applied.
    expect(requests(host.files)).toEqual([{ operation: 'review_pin', expected_revision: 3 }]);
    expect(dialog.textContent).not.toContain('Pinned');

    // The slot is free again: a save goes out.
    fireEvent.keyDown(dialog, { key: 'Escape' });
    press({ metaKey: true });
    expect(requests(host.files).at(-1)).toEqual({ operation: 'save', expected_revision: 3 });
    await host.files[1]!.answer(savedAt(3, false));
  });

  it('a structured transport rejection keeps its code and message, and never says the pin was not applied', async () => {
    const host = await createHost(engine(3));
    const { bridge, next } = rejecting(host.bridge);
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    const closed = { code: 'host.closed', message: 'The editor process exited.' };

    next.error = closed;
    choose(/^Pin engine version/);
    const dialog = pinDialog();
    await waitFor(() => expect(within(dialog).getByRole('alert').textContent).toBe('No review was returned. The editor process exited.host.closed'));

    fireEvent.click(footer('Review again'));
    await host.files[0]!.answer(reviewAt(3));
    next.error = closed;
    fireEvent.click(footer('Pin'));
    await waitFor(() =>
      expect(within(dialog).getByRole('alert').textContent).toBe(
        'Engine pin result unknown. The editor process exited.host.closedA new review is needed to try again.',
      ),
    );
    expect(dialog.textContent).not.toContain('was not applied');
    expect(within(dialog).queryByRole('button', { name: /^(Pin|Pinning…)$/ })).toBeNull();

    // With the dialog closed first, the notice says the same.
    fireEvent.click(footer('Review again'));
    await host.files[1]!.answer(reviewAt(3));
    next.error = closed;
    fireEvent.click(footer('Pin'));
    fireEvent.click(footer('Close'));
    const region = await screen.findByRole('region', { name: 'Project status' });
    expect(within(region).getByRole('alert').textContent).toBe('Engine pin result unknown. The editor process exited.host.closedDismiss');
    expect(requests(host.files).map((request) => request.operation)).toEqual(['review_pin', 'review_pin']);
  });

  it('a host’s resolved refusal stays definite, even with the editor’s own rejection code', async () => {
    // TEST DOUBLE: a host that answers with a failure result (no rejection) whose code
    // happens to be the one the editor gives rejections without a code.
    const bridge: EditorBridge = {
      ...createFixtureBridge('sample'),
      isFixture: false,
      label: 'Test double',
      capabilities: ['project.file'],
      request: () => Promise.resolve({ ok: false, error: { code: 'request.rejected', message: 'The host refused this save.' } }),
    };
    render(<App resolution={{ kind: 'bridge', bridge }} />);
    press({ ctrlKey: true });
    const region = await screen.findByRole('region', { name: 'Project status' });
    expect(within(region).getByRole('alert').textContent).toBe('Save failed. The host refused this save.request.rejectedDismiss');
  });
});
