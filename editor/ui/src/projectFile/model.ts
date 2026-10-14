/**
 * Project Save and engine pin: what the editor may say about them, read only from the
 * bridge snapshot and the host's typed receipts (editor/bridge/contract.ts,
 * ProjectFileResponse). Pure functions with no bridge calls. Host messages are shown as
 * written and never parsed into a cause or a promise.
 */
import type { BridgeSnapshot, EditorBridge, HistoryState, ProjectFileResponse } from '../bridge/contract';
import type { CapabilitySet } from '../bridge/resolve';
import { unavailableMessage } from '../bridge/resolve';
import { projectState } from '../shell/projectState';
import type { RequestResult } from '../shell/requests';
import { wasRejected } from '../shell/requests';

export type PinReview = Extract<ProjectFileResponse, { readonly operation: 'review_pin' }>['review'];
export type SaveReceipt = Extract<ProjectFileResponse, { readonly operation: 'save' }>['receipt'];
export type PinReceipt = Extract<ProjectFileResponse, { readonly operation: 'apply_pin' }>;

/** One host request at a time: the host itself refuses a second while one runs. */
export type FileOperation = 'save' | 'review' | 'apply';

/** Why a request cannot start while another runs, in the words of the one running. */
export const BUSY_REASON: Readonly<Record<FileOperation, string>> = {
  save: 'The project is being saved.',
  review: 'An engine pin review is in progress.',
  apply: 'The engine pin is being applied.',
};

/** Why Save and Pin cannot be requested now, or null when the host offers them. */
export function fileUnavailable(bridge: EditorBridge | null, snapshot: BridgeSnapshot | null, capabilities: CapabilitySet): string | null {
  if (!bridge) return unavailableMessage('project.file', null);
  const project = projectState(snapshot);
  if (project.kind === 'loading') return 'Wait for the project to open.';
  if (project.kind === 'failed') return project.projectError ? 'The project could not be opened.' : 'Lost connection to the editor process.';
  return capabilities.has('project.file') ? null : unavailableMessage('project.file', bridge);
}

/** The open document a review was prepared from, as far as the snapshot shows it. */
export interface ReviewBasis {
  readonly project: string;
  readonly history: string;
}

/**
 * The snapshot carries no project revision. Each revision step in the command bus
 * (crates/incant_cmd/src/lib.rs: commit, undo, redo, remote merge) pushes or pops a
 * History transaction, which moves the applied position or the transaction IDs at either
 * end, so this mark changes with it. An undo and redo that both land between two reads
 * look unchanged here; the host's own revision check still refuses that review.
 */
function historyMark(history: HistoryState): string {
  const { entries, applied } = history;
  return `${applied}/${entries.length}/${entries[applied - 1]?.transaction ?? ''}/${entries.at(-1)?.transaction ?? ''}`;
}

export function reviewBasis(snapshot: BridgeSnapshot | null): ReviewBasis | null {
  if (snapshot?.connection.status !== 'ready') return null;
  return { project: snapshot.connection.project.id, history: historyMark(snapshot.history) };
}

/**
 * Why a review can no longer be applied, or null while nothing the editor can see has
 * changed since it was requested. Null is not a promise: the host checks the review's
 * identity, revision and file again when it applies.
 */
export function staleReason(basis: ReviewBasis, now: ReviewBasis | null, unavailable: string | null): string | null {
  if (unavailable !== null) return unavailable;
  if (now === null) return 'The project is not open.';
  if (now.project !== basis.project) return 'A different project is open now.';
  if (now.history !== basis.history) return 'The project changed after this review.';
  return null;
}

/** Revisions are counts: grouped for reading, never abbreviated. */
export const revisionText = (revision: number) => revision.toLocaleString('en-US');

/** What a save receipt says, and no more: the file was written, or already matched. */
export function savedWords(receipt: SaveReceipt): string {
  return receipt.changed
    ? `Saved revision ${revisionText(receipt.revision)} to the project file.`
    : `Nothing to save: the project file already matches revision ${revisionText(receipt.revision)}.`;
}

/** A failure as shown: the host's message and code exactly, or the editor's own words with no code. */
export interface FileFailure {
  readonly message: string;
  readonly code: string | null;
  /**
   * Nothing says whether the host acted: the request's promise rejected, so no result
   * arrived, or the host reported success without its receipt. Shown as an unknown
   * result, not a failure.
   */
  readonly unknown: boolean;
}

/**
 * A failed request as shown, its code and message exactly as they came. Unknown exactly
 * when `Shell.request` caught a rejected promise, whatever was thrown; a host's own
 * failure answer is definite, whatever its code.
 */
export const requestFailure = (result: Extract<RequestResult, { readonly ok: false }>): FileFailure => ({
  message: result.error.message,
  code: result.error.code,
  unknown: wasRejected(result),
});

/** A success that arrived without its typed receipt is not reported as a success. */
export const MISSING_RECEIPT: Readonly<Record<FileOperation, FileFailure>> = {
  save: { code: null, unknown: true, message: 'The host reported success but returned no save receipt.' },
  review: { code: null, unknown: true, message: 'The host reported success but returned no review.' },
  apply: { code: null, unknown: true, message: 'The host reported success but returned no pin receipt.' },
};

/** How each outcome is titled: a refusal the host reported, or a result nobody reported. */
const FAILURE_TITLE: Readonly<Record<FileOperation, { readonly refused: string; readonly unknown: string }>> = {
  save: { refused: 'Save failed.', unknown: 'Save result unknown.' },
  review: { refused: 'The review could not be prepared.', unknown: 'No review was returned.' },
  apply: { refused: 'The engine pin was not applied.', unknown: 'Engine pin result unknown.' },
};

export const failureTitle = (operation: FileOperation, failure: FileFailure) =>
  failure.unknown ? FAILURE_TITLE[operation].unknown : FAILURE_TITLE[operation].refused;

/** Candidate text beyond this many characters waits for an explicit "Show all", as approval arguments do. */
export const CANDIDATE_PREVIEW = 64 * 1024;
