import { describe, expect, it } from 'vitest';
import type { EditorBridge } from './contract';
import { fixtureSnapshot, FIXTURE_VARIANTS, createFixtureBridge } from './fixture';
import { readCapabilities, resolveBridge, unavailableMessage } from './resolve';

const fakeBridge = (protocolVersion: number): EditorBridge => ({
  ...createFixtureBridge('empty'),
  protocolVersion,
  isFixture: false,
  label: 'Test double',
});

describe('resolveBridge', () => {
  it('prefers an injected bridge over a fixture request', () => {
    const injected = fakeBridge(1);
    expect(resolveBridge(injected, '?fixture=sample')).toEqual({ kind: 'bridge', bridge: injected });
  });
  it('rejects an injected bridge with a different protocol version', () => {
    expect(resolveBridge(fakeBridge(2), '')).toEqual({ kind: 'incompatible', protocolVersion: 2 });
  });
  it('uses no data at all unless a fixture is explicitly requested', () => {
    expect(resolveBridge(undefined, '')).toEqual({ kind: 'none' });
  });
  it('builds a labeled, read-only fixture on request', async () => {
    const result = resolveBridge(undefined, '?fixture=sample');
    expect(result.kind).toBe('bridge');
    if (result.kind !== 'bridge') return;
    expect(result.bridge.isFixture).toBe(true);
    expect(result.bridge.label).toMatch(/sample fixture/i);
    expect(result.bridge.capabilities).toEqual([]);
    const dispatched = await result.bridge.dispatch({ type: 'history.undo' });
    expect(dispatched.ok).toBe(false);
  });
  it('reports an unknown fixture name instead of guessing', () => {
    expect(resolveBridge(undefined, '?fixture=nope')).toEqual({ kind: 'bad-fixture', requested: 'nope' });
  });
});

describe('readCapabilities', () => {
  it('keeps known capabilities and reports unknown ones', () => {
    const caps = readCapabilities(['entity.rename', 'teleport.everything']);
    expect(caps.has('entity.rename')).toBe(true);
    expect(caps.has('entity.delete')).toBe(false);
    expect(caps.unknown).toEqual(['teleport.everything']);
  });
  it('explains why an action is unavailable', () => {
    expect(unavailableMessage('entity.delete', null)).toMatch(/no engine bridge/);
    expect(unavailableMessage('entity.delete', createFixtureBridge('sample'))).toMatch(/read-only/);
    expect(unavailableMessage('entity.delete', fakeBridge(1))).toMatch(/does not support entity\.delete/);
  });
});

describe('fixtures', () => {
  const CREDENTIAL_PATTERNS = [/sk-[A-Za-z0-9]{8,}/, /api[_-]?key\s*[:=]/i, /bearer\s+[A-Za-z0-9._-]{8,}/i, /password/i, /secret/i, /token["']?\s*[:=]/i];
  it.each(FIXTURE_VARIANTS)('%s contains no credential-shaped values', (variant) => {
    const text = JSON.stringify(fixtureSnapshot(variant));
    for (const pattern of CREDENTIAL_PATTERNS) expect(text).not.toMatch(pattern);
  });
  it('uses well-formed 26-character Crockford ULIDs', () => {
    const snapshot = fixtureSnapshot('sample');
    if (snapshot.hierarchy.status !== 'ready') throw new Error('expected ready');
    for (const key of Object.keys(snapshot.hierarchy.value.nodes)) {
      expect(key).toMatch(/^[0-9A-HJKMNP-TV-Z]{26}$/);
    }
  });
});
