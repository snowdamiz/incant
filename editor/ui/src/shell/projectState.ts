import type { BridgeError, BridgeSnapshot } from '../bridge/contract';

/**
 * The project-level state every panel presents. The native adapter maps an engine
 * `loading` response to connection=connecting and an `error` response to
 * connection=error; in both cases it publishes no entities and no history.
 */
export type ProjectState =
  | { readonly kind: 'none' }
  | { readonly kind: 'loading' }
  | { readonly kind: 'failed'; readonly error: BridgeError; readonly projectError: boolean }
  | { readonly kind: 'ready' };

export function projectState(snapshot: BridgeSnapshot | null): ProjectState {
  if (!snapshot) return { kind: 'none' };
  const connection = snapshot.connection;
  if (connection.status === 'connecting') return { kind: 'loading' };
  if (connection.status === 'error') {
    // `project.*` codes are project read/validation/recovery failures, not a lost process.
    return { kind: 'failed', error: connection.error, projectError: connection.error.code.startsWith('project.') };
  }
  return { kind: 'ready' };
}

/** True when `error` is the project failure itself, so a panel should not repeat its text. */
export function sameError(a: BridgeError, b: BridgeError): boolean {
  return a.code === b.code && a.message === b.message;
}
