import type { Capability, EditorBridge } from './contract';
import { KNOWN_CAPABILITIES, UI_PROTOCOL_VERSION } from './contract';
import { createFixtureBridge, isFixtureVariant } from './fixture';

export type BridgeResolution =
  | { readonly kind: 'bridge'; readonly bridge: EditorBridge }
  | { readonly kind: 'none' }
  | { readonly kind: 'incompatible'; readonly protocolVersion: number }
  | { readonly kind: 'bad-fixture'; readonly requested: string };

/**
 * Picks the data source. An injected host bridge always wins. The sample fixture
 * is only used when explicitly requested with `?fixture=<variant>`.
 */
export function resolveBridge(injected: EditorBridge | undefined, search: string): BridgeResolution {
  if (injected) {
    if (injected.protocolVersion !== UI_PROTOCOL_VERSION) {
      return { kind: 'incompatible', protocolVersion: injected.protocolVersion };
    }
    return { kind: 'bridge', bridge: injected };
  }
  const requested = new URLSearchParams(search).get('fixture');
  if (requested === null) return { kind: 'none' };
  if (!isFixtureVariant(requested)) return { kind: 'bad-fixture', requested };
  return { kind: 'bridge', bridge: createFixtureBridge(requested) };
}

export interface CapabilitySet {
  readonly has: (capability: Capability) => boolean;
  /** Advertised strings this UI does not understand. Shown, never guessed at. */
  readonly unknown: readonly string[];
}

export function readCapabilities(advertised: readonly string[]): CapabilitySet {
  const known = new Set<string>(KNOWN_CAPABILITIES);
  const supported = new Set<string>();
  const unknown: string[] = [];
  for (const value of advertised) {
    if (known.has(value)) supported.add(value);
    else unknown.push(value);
  }
  return { has: (capability) => supported.has(capability), unknown };
}

const CAPABILITY_LABELS: Record<Capability, string> = {
  'entity.rename': 'Rename',
  'entity.delete': 'Delete',
  'history.undo': 'Undo',
  'history.redo': 'Redo',
  'provider.connect': 'Connect provider',
  'agent.send': 'Send to agent',
  'viewport.bounds': 'Native viewport placement',
  'window.drag': 'Window dragging',
  'window.minimize': 'Minimize',
  'window.maximize': 'Maximize',
  'window.close': 'Close window',
};

export function unavailableMessage(capability: Capability, bridge: EditorBridge | null): string {
  const action = CAPABILITY_LABELS[capability];
  if (!bridge) return `${action} is unavailable: no engine bridge is connected.`;
  if (bridge.isFixture) return `${action} is unavailable: the sample fixture is read-only.`;
  return `${action} is unavailable: the connected bridge does not support ${capability}.`;
}
