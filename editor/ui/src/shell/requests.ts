import type { BridgeResult } from '../bridge/contract';

/**
 * The editor's own code for a host request whose promise rejected with no code of its own.
 * Bridges report failures as results, so a rejection says only that no result arrived:
 * not whether the host acted.
 */
export const REQUEST_REJECTED = 'request.rejected';

/** The bridge's local result, including uncertainty after a dispatched operation. */
export type RequestResult = BridgeResult;

export type RejectedRequest = Extract<BridgeResult, { readonly ok: false }> & { readonly outcomeUnknown: true };

/** The failure for a rejected request: the thrown error's own code and message when it has them, else its text. */
export function rejectedRequest(error: unknown): RejectedRequest {
  if (typeof error === 'object' && error !== null) {
    const { code, message } = error as { readonly code?: unknown; readonly message?: unknown };
    if (typeof code === 'string' && typeof message === 'string') return { ok: false, error: { code, message }, outcomeUnknown: true };
  }
  const detail = typeof error === 'string' ? error : error instanceof Error ? error.message : '';
  return { ok: false, error: { code: REQUEST_REJECTED, message: detail.trim() || 'The request ended without a result.' }, outcomeUnknown: true };
}
