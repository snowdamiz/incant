import { fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { App } from '../App';
import { createFixtureBridge } from '../bridge/fixture';
import { Splitter } from './Splitter';

/**
 * Pointer resizing. The native review (handoff 0044) found that dragging a splitter
 * selected the Inspector's text, because the press kept its default action. jsdom runs
 * no default actions, so these tests check that the press is canceled and that the
 * separator is focused explicitly in its place. handoffs/0047-splitter-input/tools/
 * checks the selection itself in a real browser.
 */

type Orientation = 'vertical' | 'horizontal';

/** Holds the panel size as App does, beside a text field that can hold focus. */
function Resizable({ orientation, invert }: { orientation: Orientation; invert: boolean }) {
  const [value, setValue] = useState(300);
  return (
    <>
      <input aria-label="Name" defaultValue="Mover" />
      <Splitter label="Resize panel" orientation={orientation} invert={invert} value={value} min={200} max={480} onChange={setValue} />
    </>
  );
}

/** TEST DOUBLE: jsdom has no pointer capture. It records which pointers were captured. */
function captureDouble(element: HTMLElement) {
  const captured: number[] = [];
  element.setPointerCapture = (pointerId: number) => {
    captured.push(pointerId);
  };
  return captured;
}

function setup(orientation: Orientation, invert = false) {
  render(<Resizable orientation={orientation} invert={invert} />);
  const splitter = screen.getByRole('separator', { name: 'Resize panel' });
  return { splitter, captured: captureDouble(splitter), size: () => Number(splitter.getAttribute('aria-valuenow')) };
}

type PointerInit = {
  pointerId: number;
  pointerType: string;
  isPrimary: boolean;
  button?: number;
  buttons?: number;
  ctrlKey?: boolean;
  clientX?: number;
  clientY?: number;
};

const mouse = { pointerId: 1, pointerType: 'mouse', isPrimary: true } as const;
const primary = { ...mouse, button: 0, buttons: 1 } as const;

/**
 * A press as a browser dispatches it: pointerdown, then the compatibility mousedown for a
 * primary mouse or pen pointer, unless pointerdown was canceled. Returns whether the
 * press's default action still runs; for a mouse, that action focuses the target and
 * starts a text selection.
 */
function press(target: Element, init: PointerInit) {
  if (!fireEvent.pointerDown(target, init)) return false;
  if (!init.isPrimary || init.pointerType === 'touch') return true;
  return fireEvent.mouseDown(target, init);
}

describe('Splitter pointer resizing', () => {
  it('cancels the default action of a primary press and focuses the separator itself', () => {
    const { splitter, captured, size } = setup('vertical');
    const name = screen.getByRole<HTMLInputElement>('textbox', { name: 'Name' });
    name.focus();
    expect(press(splitter, { ...primary, clientX: 100 })).toBe(false);
    expect(captured).toEqual([1]);
    expect(document.activeElement).toBe(splitter);
    fireEvent.pointerUp(splitter, { ...mouse, button: 0, buttons: 0, clientX: 100 });
    // The arrow keys continue the resize, as after the default focus.
    fireEvent.keyDown(document.activeElement ?? document.body, { key: 'ArrowRight' });
    expect(size()).toBe(316);
    expect(name.value).toBe('Mover');
  });

  it.each([
    ['a secondary button', { button: 2, buttons: 2 }],
    ['a middle button', { button: 1, buttons: 4 }],
    ['a second touch', { pointerId: 7, pointerType: 'touch', isPrimary: false, button: 0, buttons: 1 }],
  ])('does not drag on %s and keeps its default action', (_, init) => {
    const { splitter, captured, size } = setup('vertical');
    expect(press(splitter, { ...mouse, ...init, clientX: 100 })).toBe(true);
    fireEvent.pointerMove(splitter, { ...mouse, ...init, clientX: 160 });
    expect(size()).toBe(300);
    expect(captured).toEqual([]);
    expect(document.activeElement).not.toBe(splitter);
  });

  it('does not drag on a Control-click, the macOS secondary click, nor select text', () => {
    const { splitter, captured, size } = setup('vertical');
    expect(press(splitter, { ...primary, ctrlKey: true, clientX: 100 })).toBe(false);
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, ctrlKey: true, clientX: 160 });
    expect(size()).toBe(300);
    expect(captured).toEqual([]);
    expect(document.activeElement).toBe(splitter);
  });

  it.each([
    ['vertical', false],
    ['vertical', true],
    ['horizontal', false],
    ['horizontal', true],
  ] as const)('drags a %s separator (invert: %s) along its own axis, within its bounds', (orientation, invert) => {
    const { splitter, size } = setup(orientation, invert);
    const at = (along: number, across = 50) =>
      orientation === 'vertical' ? { clientX: along, clientY: across } : { clientX: across, clientY: along };
    const sign = invert ? -1 : 1;
    press(splitter, { ...primary, ...at(100) });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, ...at(140) });
    expect(size()).toBe(300 + sign * 40);
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, ...at(140, 400) });
    expect(size()).toBe(300 + sign * 40);
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, ...at(100 + sign * 1000) });
    expect(size()).toBe(480);
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, ...at(100 - sign * 1000) });
    expect(size()).toBe(200);
  });

  it.each(['pointerUp', 'pointerCancel', 'lostPointerCapture'] as const)('ends the drag on %s from its own pointer', (end) => {
    const { splitter, size } = setup('vertical');
    press(splitter, { ...primary, clientX: 100 });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 120 });
    // Another pointer neither moves nor ends this drag.
    const other = { ...mouse, pointerId: 2, pointerType: 'pen' };
    fireEvent.pointerMove(splitter, { ...other, buttons: 1, clientX: 400 });
    expect(size()).toBe(320);
    fireEvent[end](splitter, { ...other, clientX: 400 });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 130 });
    expect(size()).toBe(330);
    fireEvent[end](splitter, { ...mouse, clientX: 130 });
    // Still pressed, so only the end event can have stopped the drag.
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 200 });
    expect(size()).toBe(330);
    // The next press starts from the size the last drag left.
    press(splitter, { ...primary, clientX: 200 });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 190 });
    expect(size()).toBe(320);
  });
});

describe('Splitter after a lost release', () => {
  it('ends the drag at the first move without the primary button', () => {
    const { splitter, size } = setup('vertical');
    press(splitter, { ...primary, clientX: 100 });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 120 });
    // A context menu took the release, so no pointerup arrives.
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 0, clientX: 160 });
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 200 });
    expect(size()).toBe(320);
  });
});

describe('Splitter in the editor shell', () => {
  const columns = () => document.querySelector<HTMLElement>('.workspace')?.style.gridTemplateColumns;

  it('resizes the hierarchy by pointer, then by keyboard, ringed only for the keyboard', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    const splitter = screen.getByRole('separator', { name: 'Resize hierarchy' });
    captureDouble(splitter);
    const start = Number(splitter.getAttribute('aria-valuenow'));
    expect(press(splitter, { ...primary, clientX: 300 })).toBe(false);
    fireEvent.pointerMove(splitter, { ...mouse, buttons: 1, clientX: 340 });
    fireEvent.pointerUp(splitter, { ...mouse, button: 0, buttons: 0, clientX: 340 });
    expect(splitter.getAttribute('aria-valuenow')).toBe(String(start + 40));
    expect(columns()).toMatch(new RegExp(`^${start + 40}px `));
    expect(document.activeElement).toBe(splitter);
    expect(splitter.hasAttribute('data-focus-visible')).toBe(false);
    fireEvent.keyDown(splitter, { key: 'ArrowRight' });
    expect(splitter.getAttribute('aria-valuenow')).toBe(String(start + 56));
    expect(splitter.hasAttribute('data-focus-visible')).toBe(true);
  });

  it('closes an open menu when a splitter is pressed, like any press outside it', () => {
    render(<App resolution={{ kind: 'bridge', bridge: createFixtureBridge('sample') }} />);
    fireEvent.contextMenu(screen.getByRole('treeitem', { name: /^Crate 01/ }), { clientX: 120, clientY: 300 });
    expect(screen.getByRole('menu')).toBeTruthy();
    const splitter = screen.getByRole('separator', { name: 'Resize inspector and agent' });
    captureDouble(splitter);
    expect(press(splitter, { ...primary, clientX: 700 })).toBe(false);
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(splitter);
  });
});
