import { useLayoutEffect } from 'react';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AssetsProvider } from '../../assets/AssetsContext';
import type { ComponentSchema, Diagnostic, EditorBridge, Ulid } from '../../bridge/contract';
import { fixtureId, fixtureSnapshot } from '../../bridge/fixture';
import { ShellProvider, useShell } from '../../shell/ShellContext';
import { fieldDomId } from './FieldView';
import { InspectorPanel } from './InspectorPanel';

afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const ENTITY = fixtureId(9999);

const strings = { type: 'array', items: { type: 'string' } } as const;
const names = (prefix: string) => Array.from({ length: 64 }, (_, i) => `${prefix} ${i}`);

interface Entry {
  entity: Ulid;
  schema: ComponentSchema;
  value: Record<string, unknown>;
  paths: readonly string[];
}

function SelectedInspector({ entities }: { entities: readonly Ulid[] }) {
  const { select } = useShell();
  const initial = entities[0]!;
  useLayoutEffect(() => select(initial), [initial, select]);
  return <>
    {entities.map((entity) => <button key={entity} onClick={() => select(entity)}>Select {entity}</button>)}
    <InspectorPanel />
  </>;
}

function showEntries(entries: readonly Entry[]) {
  const entities = [...new Set(entries.map((entry) => entry.entity))];
  const snapshot = {
    ...fixtureSnapshot('sample'),
    hierarchy: { status: 'ready' as const, value: { roots: entities, nodes: Object.fromEntries(entities.map((id) => [id,
      { id, name: 'Diagnostic target', kind: 'entity' as const, parent: null, children: [] },
    ])) } },
    schemas: Object.fromEntries(entries.map((entry) => [entry.schema.type, entry.schema])),
    entities: Object.fromEntries(entities.map((id) => [id, { id, kind: 'entity' as const, name: 'Diagnostic target', components:
      entries.filter((entry) => entry.entity === id).map(({ schema, value }) => ({ type: schema.type, schemaVersion: 1, value })),
    }])),
    diagnostics: entries.flatMap(({ entity, schema, paths }) => paths.map((path): Diagnostic => ({
      id: `${entity}:${schema.type}:${path}`, severity: 'error', message: `Invalid ${path}`, entity, component: schema.type, path,
    }))),
  };
  const dispatch = vi.fn<EditorBridge['dispatch']>(() => Promise.resolve({ ok: true }));
  const bridge: EditorBridge = {
    protocolVersion: 1, label: 'Diagnostic test', capabilities: [], isFixture: false,
    getSnapshot: () => snapshot, subscribe: () => () => undefined,
    dispatch, request: () => Promise.resolve({ ok: true }),
  };
  render(<ShellProvider bridge={bridge}><AssetsProvider><SelectedInspector entities={entities} /></AssetsProvider></ShellProvider>);
  return { dispatch };
}

function show(schema: ComponentSchema, value: Record<string, unknown>, paths: readonly string[]) {
  const { dispatch } = showEntries([{ entity: ENTITY, schema, value, paths }]);
  const reveal = (path: string) => fireEvent.click(screen.getByRole('link', { name: `${schema.type}${path}` }));
  const field = (path: readonly string[]) => document.getElementById(fieldDomId(ENTITY, schema.type, path));
  return { reveal, field, dispatch };
}

describe('diagnostic links into virtualized Inspector values', () => {
  it('reveals part 63 from part 1 and focuses its exact field with bounded rows', () => {
    const schema = fixtureSnapshot('sample').schemas.Collider!;
    const parts = Array.from({ length: 64 }, (_, index) => ({
      id: fixtureId(10000 + index), translation: [0, 0, 0], rotation: [0, 0, 0, 1],
      shape: { type: 'sphere', radius: index + 1 },
    }));
    const { reveal, field, dispatch } = show(schema, { shape: { type: 'compound', parts } }, [
      '/shape/parts/0/shape/radius', '/shape/parts/63/shape/radius',
    ]);
    expect(screen.getByRole('group', { name: /^Part 1 of 64/ })).toBeTruthy();
    expect(field(['shape', 'parts', '63'])).toBeNull();
    reveal('/shape/parts/63/shape/radius');
    expect(screen.getByRole('group', { name: /^Part 64 of 64/ })).toBeTruthy();
    expect(document.activeElement).toBe(field(['shape', 'parts', '63', 'shape', 'radius']));
    expect((document.activeElement as HTMLInputElement).value).toBe('64');
    expect(within(screen.getByRole('listbox', { name: 'Parts' })).getAllByRole('option').length).toBeLessThan(40);
    // A later link must win immediately; no earlier deferred focus may reclaim it.
    reveal('/shape/parts/0/shape/radius');
    expect(document.activeElement).toBe(field(['shape', 'parts', '0', 'shape', 'radius']));
    expect(dispatch).not.toHaveBeenCalled();
  });

  it('opens a collapsed component before resolving a virtualized descendant', () => {
    const schema: ComponentSchema = { type: 'Labels', version: 1, title: 'Labels', properties: {
      names: { type: 'array', items: { type: 'string' } },
    } };
    const { reveal, field } = show(schema, { names: Array.from({ length: 512 }, (_, i) => `Name ${i}`) }, ['/names/511']);
    const toggle = within(screen.getByRole('region', { name: 'Labels' })).getByRole('button');
    fireEvent.click(toggle);
    expect(toggle.getAttribute('aria-expanded')).toBe('false');
    reveal('/names/511');
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(document.activeElement).toBe(field(['names', '511']));
  });

  it('opens a collapsed numeric grid and reveals a cell outside both viewport axes', () => {
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
    const schema: ComponentSchema = { type: 'NavigationGrid', version: 1, title: 'Navigation Grid', properties: {
      dimensions: { type: 'array', items: { type: 'integer' } },
      costs: { type: 'array', items: { type: 'number' } },
    } };
    const { reveal, field, dispatch } = show(schema, { dimensions: [64, 64], costs: Array.from({ length: 4096 }, (_, i) => i + 1) }, ['/costs/4095']);
    expect(screen.getByRole('button', { name: 'Show values' }).getAttribute('aria-expanded')).toBe('false');
    expect(field(['costs', '4095'])).toBeNull();
    reveal('/costs/4095');
    expect(screen.getByRole('button', { name: 'Hide values' }).getAttribute('aria-expanded')).toBe('true');
    const target = field(['costs', '4095']);
    expect(target?.textContent).toBe('4096');
    expect(document.activeElement).toBe(target);
    const table = target!.closest<HTMLElement>('.number-table')!;
    expect(table.scrollTop).toBeGreaterThan(0);
    expect(table.scrollLeft).toBeGreaterThan(0);
    expect(table.querySelectorAll('.number-table__cell').length).toBeLessThan(500);
    // Closing removes the table's reveal handler; reopening registers its replacement.
    fireEvent.click(screen.getByRole('button', { name: 'Hide values' }));
    reveal('/costs/4095');
    expect(document.activeElement).toBe(field(['costs', '4095']));
    expect(dispatch).not.toHaveBeenCalled();
  });

  it('reveals an off-window string entry without mounting the entire list', () => {
    const schema: ComponentSchema = { type: 'Labels', version: 1, title: 'Labels', properties: {
      names: { type: 'array', items: { type: 'string' } },
    } };
    const { reveal, field, dispatch } = show(schema, { names: Array.from({ length: 512 }, (_, i) => `Name ${i}`) }, ['/names/511']);
    expect(field(['names', '511'])).toBeNull();
    reveal('/names/511');
    expect(document.activeElement).toBe(field(['names', '511']));
    expect((document.activeElement as HTMLInputElement).value).toBe('Name 511');
    expect(document.querySelectorAll('.string-list__row').length).toBeLessThan(50);
    expect(dispatch).not.toHaveBeenCalled();
  });
});

describe('diagnostic identities and JSON pointers', () => {
  it('distinguishes an underscore property from a nested property path', () => {
    const schema: ComponentSchema = { type: 'Probe', version: 1, title: 'Probe', order: ['a_b', 'a'], properties: {
      a_b: strings, a: { type: 'object', properties: { b: strings } },
    } };
    const { reveal } = show(schema, { a_b: names('Flat'), a: { b: names('Nested') } }, ['/a_b/63', '/a/b/63']);
    reveal('/a/b/63');
    expect((document.activeElement as HTMLInputElement).value).toBe('Nested 63');
    reveal('/a_b/63');
    expect((document.activeElement as HTMLInputElement).value).toBe('Flat 63');
    const ids = [...document.querySelectorAll('[id]')].map((element) => element.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('routes only the named component despite punctuation collisions', () => {
    const schema = (type: string): ComponentSchema => ({ type, version: 1, title: type, properties: { names: strings } });
    showEntries([
      { entity: ENTITY, schema: schema('Probe.One'), value: { names: names('Dot') }, paths: ['/names/63'] },
      { entity: ENTITY, schema: schema('Probe_One'), value: { names: names('Underscore') }, paths: ['/names/63'] },
    ]);
    const dotToggle = within(screen.getByRole('region', { name: 'Probe.One' })).getByRole('button');
    fireEvent.click(dotToggle);
    fireEvent.click(screen.getByRole('link', { name: 'Probe_One/names/63' }));
    expect((document.activeElement as HTMLInputElement).value).toBe('Underscore 63');
    expect(dotToggle.getAttribute('aria-expanded')).toBe('false');
    const controls = screen.getAllByRole('button').map((button) => button.getAttribute('aria-controls')).filter(Boolean);
    expect(new Set(controls).size).toBe(controls.length);
  });

  it.each([
    { key: '', path: '//63', other: 'empty' },
    { key: 'a/b', path: '/a~1b/63', other: 'a_1b' },
    { key: 'a~b', path: '/a~0b/63', other: 'a_0b' },
    { key: 'a~1b', path: '/a~01b/63', other: 'a/b' },
    { key: '%2F', path: '/%2F/63', other: '/' },
  ])('preserves and decodes the authored key $key exactly once', ({ key, path, other }) => {
    const schema: ComponentSchema = { type: 'Probe', version: 1, title: 'Probe', properties: { [other]: strings, [key]: strings } };
    const { reveal, field } = show(schema, { [other]: names('Other'), [key]: names('Exact') }, [path]);
    reveal(path);
    const target = field([key, '63']);
    expect(document.activeElement).toBe(target);
    expect((target as HTMLInputElement).value).toBe('Exact 63');
    expect(screen.getByRole('link', { name: `Probe${path}` }).getAttribute('href')).toBe(`#${target!.id}`);
  });

  it('preserves trailing and repeated empty tokens instead of addressing the root', () => {
    const schema: ComponentSchema = { type: 'Probe', version: 1, title: 'Probe', properties: {
      '': { type: 'object', properties: { '': { type: 'string', title: 'Empty key' } } },
    } };
    const { reveal, field } = show(schema, { '': { '': 'Nested empty key' } }, ['//']);
    reveal('//');
    expect(document.activeElement).toBe(field(['', '']));
    expect((document.activeElement as HTMLInputElement).value).toBe('Nested empty key');
  });

  it('replaces registrations when selection changes while the Inspector remains mounted', () => {
    const other = fixtureId(9998);
    const schema: ComponentSchema = { type: 'Probe', version: 1, title: 'Probe', properties: { names: strings } };
    showEntries([
      { entity: ENTITY, schema, value: { names: names('First') }, paths: ['/names/63'] },
      { entity: other, schema, value: { names: names('Second') }, paths: ['/names/63'] },
    ]);
    fireEvent.click(screen.getByRole('link', { name: 'Probe/names/63' }));
    expect((document.activeElement as HTMLInputElement).value).toBe('First 63');
    const inspector = screen.getByRole('region', { name: 'Inspector' });
    const component = screen.getByRole('region', { name: 'Probe' });
    fireEvent.click(screen.getByRole('button', { name: `Select ${other}` }));
    expect(screen.getByRole('region', { name: 'Inspector' })).toBe(inspector);
    // This descendant DOM survives, proving the provider above it was not remounted.
    expect(screen.getByRole('region', { name: 'Probe' })).toBe(component);
    expect(document.getElementById(fieldDomId(ENTITY, 'Probe', ['names']))).toBeNull();
    fireEvent.click(screen.getByRole('link', { name: 'Probe/names/63' }));
    expect((document.activeElement as HTMLInputElement).value).toBe('Second 63');
    expect(document.activeElement?.id).toBe(fieldDomId(other, 'Probe', ['names', '63']));
    fireEvent.click(screen.getByRole('button', { name: `Select ${ENTITY}` }));
    fireEvent.click(screen.getByRole('link', { name: 'Probe/names/63' }));
    expect((document.activeElement as HTMLInputElement).value).toBe('First 63');
  });

  it('falls back to the component for an invalid pointer without guessing a matching key', () => {
    const schema: ComponentSchema = { type: 'Probe', version: 1, title: 'Probe', properties: { 'names~2': strings } };
    const { reveal, field } = show(schema, { 'names~2': names('Must not reveal') }, ['/names~2/63']);
    reveal('/names~2/63');
    expect(document.activeElement).toBe(within(screen.getByRole('region', { name: 'Probe' })).getByRole('button'));
    expect(field(['names~2', '63'])).toBeNull();
  });

  it('keeps arbitrary identity tuples and derived label IDs distinct and selector-safe', () => {
    const paths = [[], [''], ['', ''], ['a_b'], ['a', 'b'], ['a-b'], ['~'], ['/'], ['_002f'],
      ['é'], ['e\u0301'], ['🚀'], ['\ud800'], ['label'], ['a', 'label'], ['value']];
    const ids = ['Probe', 'Probe.One', 'Probe_One', 'Probe-One', '', '/', '~'].flatMap((component) =>
      paths.map((path) => fieldDomId(ENTITY, component, path)),
    );
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) {
      expect(id).toMatch(/^field-[A-Za-z0-9_-]+$/);
      expect(ids).not.toContain(`${id}-label`);
      expect(ids).not.toContain(`${id}-body`);
    }
  });
});
