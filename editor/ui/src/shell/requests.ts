import type { BridgeError, BridgeResult } from '../bridge/contract';

/**
 * The editor's own code for a host request whose promise rejected with no code of its own.
 * Bridges report failures as results, so a rejection says only that no result arrived:
 * not whether the host acted.
 */
export const REQUEST_REJECTED = 'request.rejected';

/**
 * Marks a failure the editor made itself, in `Shell.request`'s catch, for a request whose
 * promise rejected. A symbol, so no host answer, IPC payload or code can carry it.
 */
const REJECTED: unique symbol = Symbol('request rejected');

/** A request whose promise rejected: the error as thrown, and no result from the host. */
export interface RejectedRequest {
  readonly ok: false;
  readonly error: BridgeError;
  readonly [REJECTED]: true;
}

/** What `Shell.request` resolves to: the bridge's own result, or a rejected request. */
export type RequestResult = BridgeResult | RejectedRequest;

/** The failure for a rejected request: the thrown error's own code and message when it has them, else its text. */
export function rejectedRequest(error: unknown): RejectedRequest {
  if (typeof error === 'object' && error !== null) {
    const { code, message } = error as { readonly code?: unknown; readonly message?: unknown };
    if (typeof code === 'string' && typeof message === 'string') return { ok: false, error: { code, message }, [REJECTED]: true };
  }
  const detail = typeof error === 'string' ? error : error instanceof Error ? error.message : '';
  return { ok: false, error: { code: REQUEST_REJECTED, message: detail.trim() || 'The request ended without a result.' }, [REJECTED]: true };
}

/** True only for a failure `Shell.request` made from a rejected promise, never from a host's answer. */
export const wasRejected = (result: RequestResult): result is RejectedRequest => REJECTED in result;
