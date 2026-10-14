import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { BridgeSnapshot, EditorBridge } from '../bridge/contract';
import { useShell } from '../shell/ShellContext';
import type { FileFailure, FileOperation, PinReceipt, PinReview, ReviewBasis } from './model';
import {
  BUSY_REASON,
  MISSING_RECEIPT,
  failureTitle,
  fileUnavailable,
  requestFailure,
  revisionText,
  reviewBasis,
  savedWords,
  staleReason,
} from './model';

/** An open project as the titlebar names it. */
export interface OpenProject {
  readonly id: string;
  readonly name: string;
}

/** The engine pin dialog, from its review request to the outcome of applying it. */
export type PinState =
  | { readonly status: 'closed' }
  | { readonly status: 'requesting' }
  | { readonly status: 'refused'; readonly failure: FileFailure }
  | { readonly status: 'ready'; readonly review: PinReview; readonly basis: ReviewBasis }
  | { readonly status: 'stale'; readonly review: PinReview; readonly reason: string }
  | { readonly status: 'applying'; readonly review: PinReview }
  | { readonly status: 'applied'; readonly review: PinReview; readonly receipt: PinReceipt }
  | { readonly status: 'failed'; readonly review: PinReview; readonly failure: FileFailure }
  /**
   * The host connection or the open project changed after the dialog was opened. Nothing
   * requested before is applied, shown or reported here; `applying` says a pin was still
   * in flight, so its result is unknown here.
   */
  | { readonly status: 'retired'; readonly applying: boolean; readonly reconnected: boolean };

/** A failure no dialog is showing. It stays until dismissed or superseded, in its own project. */
export interface FileNotice {
  readonly operation: 'save' | 'pin';
  readonly title: string;
  readonly failure: FileFailure;
}

export interface ProjectFile {
  /** Why Save and Pin cannot be requested now, or null. */
  readonly unavailable: string | null;
  /** The open project's request in flight. There is never more than one. */
  readonly busy: FileOperation | null;
  readonly save: () => void;
  readonly pin: PinState;
  /** The project the dialog's review was requested for; it labels the review. */
  readonly pinProject: OpenProject | null;
  /** The project open now. */
  readonly openProject: OpenProject | null;
  readonly openPin: (from: HTMLElement | null) => void;
  readonly closePin: () => void;
  readonly reviewAgain: () => void;
  readonly applyPin: () => void;
  readonly notice: FileNotice | null;
  readonly dismissNotice: () => void;
}

const ProjectFileContext = createContext<ProjectFile | null>(null);

const openProjectOf = (snapshot: BridgeSnapshot | null | undefined): OpenProject | null =>
  snapshot?.connection.status === 'ready' ? { id: snapshot.connection.project.id, name: snapshot.connection.project.name } : null;

/**
 * The host connection and project that requests belong to. A new object whenever the
 * bridge is replaced or a different project is read; a project that is only reloading or
 * failed does not end it. Requests compare it by identity when their answers arrive.
 */
interface Scope {
  readonly bridge: EditorBridge | null;
  readonly project: string | null;
}

interface Run {
  readonly operation: FileOperation;
  readonly scope: Scope;
}

/**
 * Save and the engine pin, through the host's existing `project.save`,
 * `project.pin.review` and `project.pin.apply` requests only. The editor never writes a
 * file or a document itself, sends one request at a time, never retries on its own, and
 * reports only what the host's receipt says, to the project that asked: an answer that
 * arrives after another project (or another host connection) opened is dropped. A review
 * is applied only by its own ID and only while nothing the editor can see has changed
 * since it was requested; the host checks the same review, revision and file again
 * before it writes.
 */
export function ProjectFileProvider({ children }: { children: ReactNode }) {
  const { bridge, snapshot, capabilities, request, announce } = useShell();
  const unavailable = fileUnavailable(bridge, snapshot, capabilities);
  const openProject = useMemo(() => openProjectOf(snapshot), [snapshot]);

  const [scope, setScope] = useState<Scope>(() => ({ bridge, project: openProjectOf(bridge?.getSnapshot())?.id ?? null }));
  const scopeRef = useRef(scope);
  // Read synchronously, so a second request in the same event cannot start.
  const running = useRef<Run | null>(null);
  const [busy, setBusy] = useState<FileOperation | null>(null);
  const [notice, setNotice] = useState<FileNotice | null>(null);
  const [pin, setPin] = useState<PinState>({ status: 'closed' });
  const [pinFor, setPinFor] = useState<{ readonly scope: Scope; readonly project: OpenProject | null } | null>(null);
  // The dialog's current request. Closing it or asking again retires an earlier answer.
  const ticket = useRef(0);
  const opener = useRef<HTMLElement | null>(null);

  // Observed on every publish, which comes before the bridge resolves the request it
  // answers, so a late answer is always compared with the project open by then.
  useLayoutEffect(() => {
    const observe = () => {
      const project = openProjectOf(bridge?.getSnapshot())?.id ?? null;
      const seen = scopeRef.current;
      if (seen.bridge === bridge && (project === null || project === seen.project)) return;
      const next: Scope = { bridge, project: project ?? (seen.bridge === bridge ? seen.project : null) };
      scopeRef.current = next;
      // The newly open project has no request of its own yet, and no earlier failure.
      running.current = null;
      setBusy(null);
      setNotice(null);
      setScope(next);
    };
    observe();
    return bridge?.subscribe(observe);
  }, [bridge]);

  const begin = (operation: FileOperation): Run => {
    const run = { operation, scope: scopeRef.current };
    running.current = run;
    setBusy(operation);
    return run;
  };
  const end = (run: Run) => {
    if (running.current !== run) return;
    running.current = null;
    setBusy(null);
  };
  /** An answer belongs to the scope it was requested in, or to nothing shown now. */
  const belongs = (run: Run) => run.scope === scopeRef.current;

  const save = useCallback(async () => {
    if (unavailable !== null) {
      announce(unavailable);
      return;
    }
    if (running.current !== null) {
      // A second Save while one runs is the same request; anything else says what runs.
      if (running.current.operation !== 'save') announce(BUSY_REASON[running.current.operation]);
      return;
    }
    const run = begin('save');
    try {
      const result = await request({ type: 'project.save' }, { quiet: true });
      if (!belongs(run)) return;
      if (result.ok && result.projectFile?.operation === 'save') {
        announce(savedWords(result.projectFile.receipt));
        setNotice((current) => (current?.operation === 'save' ? null : current));
        return;
      }
      const failure = result.ok ? MISSING_RECEIPT.save : requestFailure(result);
      setNotice({ operation: 'save', title: failureTitle('save', failure), failure });
    } finally {
      end(run);
    }
  }, [announce, request, unavailable]);

  const requestReview = useCallback(async () => {
    if (running.current !== null) return;
    const id = ++ticket.current;
    setPinFor({ scope: scopeRef.current, project: openProjectOf(bridge?.getSnapshot()) });
    if (unavailable !== null) {
      setPin({ status: 'refused', failure: { message: unavailable, code: null, unknown: false } });
      return;
    }
    const run = begin('review');
    setPin({ status: 'requesting' });
    // Taken from the same publish whose read's revision the bridge sends with the request.
    // The snapshot itself has no revision, so the basis is its History mark (model.ts).
    const basis = reviewBasis(bridge?.getSnapshot() ?? null);
    try {
      const result = await request({ type: 'project.pin.review' }, { quiet: true });
      // A closed dialog, a newer request or another project retires this answer; the host
      // keeps only its latest review in any case.
      if (id !== ticket.current || !belongs(run)) return;
      if (result.ok && result.projectFile?.operation === 'review_pin' && basis !== null) {
        setPin({ status: 'ready', review: result.projectFile.review, basis });
      } else if (result.ok && result.projectFile?.operation === 'review_pin') {
        setPin({ status: 'refused', failure: { message: 'The project was not open when the review was requested.', code: null, unknown: false } });
      } else {
        setPin({ status: 'refused', failure: result.ok ? MISSING_RECEIPT.review : requestFailure(result) });
      }
    } finally {
      end(run);
    }
  }, [bridge, request, unavailable]);

  const openPin = useCallback(
    (from: HTMLElement | null) => {
      if (unavailable !== null) {
        announce(unavailable);
        return;
      }
      if (running.current !== null) {
        announce(BUSY_REASON[running.current.operation]);
        return;
      }
      opener.current = from;
      void requestReview();
    },
    [announce, requestReview, unavailable],
  );

  const closePin = useCallback(() => {
    ticket.current += 1;
    setPin({ status: 'closed' });
    setPinFor(null);
    const target = opener.current;
    opener.current = null;
    // The opener can be gone (the project closed while the dialog was open).
    if (target?.isConnected) target.focus();
    else document.querySelector<HTMLElement>('.titlebar__project-button')?.focus();
  }, []);

  // A dialog opened for another connection or project shows nothing of it as current.
  const retired = pin.status !== 'closed' && pin.status !== 'retired' && pinFor !== null && pinFor.scope !== scope;
  const reconnected = pinFor !== null && scope.project !== null && pinFor.scope.project === scope.project;
  // Anything the editor can see that changes after a review retires it for good: History
  // that returns to an identical mark has still been through other revisions.
  const changed = !retired && pin.status === 'ready' ? staleReason(pin.basis, reviewBasis(snapshot), unavailable) : null;
  useEffect(() => {
    if (retired) setPin({ status: 'retired', applying: pin.status === 'applying', reconnected });
    else if (pin.status === 'ready' && changed !== null) setPin({ status: 'stale', review: pin.review, reason: changed });
  }, [retired, reconnected, pin, changed]);
  const shownPin = useMemo<PinState>(() => {
    if (retired) return { status: 'retired', applying: pin.status === 'applying', reconnected };
    if (pin.status === 'ready' && changed !== null) return { status: 'stale', review: pin.review, reason: changed };
    return pin;
  }, [retired, reconnected, pin, changed]);

  const applyPin = useCallback(async () => {
    if (pin.status !== 'ready' || retired || changed !== null || running.current !== null) return;
    const { review } = pin;
    const id = ticket.current;
    const run = begin('apply');
    setPin({ status: 'applying', review });
    try {
      // Only the review's own ID: the host applies its retained review, never a payload from here.
      const result = await request({ type: 'project.pin.apply', reviewId: review.review_id }, { quiet: true });
      // The pin belonged to a project that is no longer open: nothing is said about it here.
      if (!belongs(run)) return;
      const shown = id === ticket.current;
      if (result.ok && result.projectFile?.operation === 'apply_pin') {
        const receipt = result.projectFile;
        setNotice((current) => (current?.operation === 'pin' ? null : current));
        // The open dialog says it; a closed one leaves it to the status line.
        if (shown) setPin({ status: 'applied', review, receipt });
        else announce(`Pinned to engine ${review.command.requirement} at revision ${revisionText(receipt.revision)}.`);
        return;
      }
      const failure = result.ok ? MISSING_RECEIPT.apply : requestFailure(result);
      if (shown) setPin({ status: 'failed', review, failure });
      else setNotice({ operation: 'pin', title: failureTitle('apply', failure), failure });
    } finally {
      end(run);
    }
  }, [announce, changed, pin, request, retired]);

  const reviewAgain = useCallback(() => void requestReview(), [requestReview]);
  const dismissNotice = useCallback(() => setNotice(null), []);

  const value = useMemo<ProjectFile>(
    () => ({
      unavailable,
      busy,
      save: () => void save(),
      pin: shownPin,
      pinProject: pinFor?.project ?? null,
      openProject,
      openPin,
      closePin,
      reviewAgain,
      applyPin: () => void applyPin(),
      notice,
      dismissNotice,
    }),
    [unavailable, busy, save, shownPin, pinFor, openProject, openPin, closePin, reviewAgain, applyPin, notice, dismissNotice],
  );
  return <ProjectFileContext.Provider value={value}>{children}</ProjectFileContext.Provider>;
}

export function useProjectFile(): ProjectFile {
  const value = useContext(ProjectFileContext);
  if (!value) throw new Error('useProjectFile must be used inside ProjectFileProvider');
  return value;
}
