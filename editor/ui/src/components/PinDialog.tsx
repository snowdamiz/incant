import { useId, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import { Icon } from '../icons/Icon';
import type { PinState } from '../projectFile/ProjectFileContext';
import { useProjectFile } from '../projectFile/ProjectFileContext';
import type { FileFailure, FileOperation, PinReview } from '../projectFile/model';
import { CANDIDATE_PREVIEW, failureTitle, revisionText } from '../projectFile/model';
import { reveal, sizeText, utf8Bytes } from './agent/agentText';

const FOCUSABLE = 'button, [href], input, [tabindex="0"]';

/**
 * Review, then pin the open project to the current engine version. The dialog shows the
 * host's review as returned: the requirement and the revision it was prepared at, with
 * the command, review ID, file checksums and the resulting file text (plain text, never
 * markup) on request. Pin sends only the review's ID. Nothing is chosen or applied by
 * itself: the safe answer has focus, and a review that the project has moved past offers
 * Review again instead of Pin. The review is named for the project it was requested
 * for; if another project opens meanwhile, the dialog says so and shows nothing of the
 * earlier one as current. Traps focus; Escape closes, and focus returns to the opener.
 */
export function PinDialog() {
  const file = useProjectFile();
  const { pin } = file;
  const dialogRef = useRef<HTMLDivElement>(null);
  const safeRef = useRef<HTMLButtonElement>(null);
  const ids = useId();
  const titleId = `${ids}-title`;
  const leadId = `${ids}-lead`;
  const project = file.pinProject?.name ?? null;

  // Focus starts on the safe answer, and returns to it whenever the focused control goes
  // away (Pin after it finishes, Review again while a review is prepared). It never moves
  // onto Pin by itself.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog || (dialog.contains(document.activeElement) && document.activeElement !== dialog)) return;
    safeRef.current?.focus();
  }, [pin.status]);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      file.closePin();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE);
    if (!focusable || focusable.length === 0) return;
    const first = focusable[0]!;
    const last = focusable[focusable.length - 1]!;
    if (event.shiftKey && (document.activeElement === first || document.activeElement === dialogRef.current)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };

  const busy = pin.status === 'requesting' || pin.status === 'applying';
  // Before a decision this dismisses the review; afterwards it only closes the dialog.
  const settled = pin.status !== 'requesting' && pin.status !== 'ready' && pin.status !== 'stale';
  const canReview = file.unavailable === null && file.busy === null;

  return (
    <div className="scrim" onMouseDown={file.closePin}>
      <div
        ref={dialogRef}
        className="dialog dialog--pin"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={leadId}
        aria-busy={busy || undefined}
        tabIndex={-1}
        onKeyDown={onKeyDown}
        onMouseDown={(event) => event.stopPropagation()}
      >
        {/* No close glyph: Cancel or Close is always in the footer, beside the decision. */}
        <div className="dialog__header">
          <h2 id={titleId} className="dialog__title">
            Pin engine version
          </h2>
        </div>
        <div className="dialog__body pin">
          <PinBody pin={pin} project={project} now={file.openProject?.name ?? null} leadId={leadId} canReview={canReview} />
          <div className="pin__actions">
            <button ref={safeRef} type="button" className="button" onClick={file.closePin}>
              {settled ? 'Close' : 'Cancel'}
            </button>
            <PrimaryAction pin={pin} canReview={canReview} onPin={file.applyPin} onReview={file.reviewAgain} />
          </div>
        </div>
      </div>
    </div>
  );
}

function PinBody({
  pin,
  project,
  now,
  leadId,
  canReview,
}: {
  pin: PinState;
  /** The project the review was requested for. */
  project: string | null;
  /** The project open now. */
  now: string | null;
  leadId: string;
  canReview: boolean;
}) {
  switch (pin.status) {
    case 'closed':
      return null;
    case 'requesting':
      return (
        <div role="status">
          <p id={leadId} className="pin__lead">
            <span className="spinner spinner--small" aria-hidden="true" />
            Preparing the review…
          </p>
        </div>
      );
    case 'refused':
      return <Failure id={leadId} operation="review" failure={pin.failure} />;
    case 'retired':
      // Nothing of the earlier project is shown as current; a pin still in flight for it
      // is said to be unknown here rather than guessed.
      return (
        <p id={leadId} className="pin__problem pin__problem--warning" role="alert">
          <Icon name="warning" size={14} className="pin__problem-icon" />
          <span>
            {pin.reconnected ? (
              <strong>
                The editor reconnected to <ProjectName project={project} /> after this review was requested.
              </strong>
            ) : (
              <>
                <strong>
                  <ProjectName project={project} /> is no longer the open project.
                </strong>
                {now !== null ? (
                  <>
                    {' '}
                    <ProjectName project={now} /> is open now.
                  </>
                ) : null}
              </>
            )}
            {pin.applying ? (
              <>
                {' '}
                Whether <ProjectName project={project} /> was pinned is not shown here.
              </>
            ) : null}
            {canReview ? ' Review again before pinning.' : ''}
          </span>
        </p>
      );
    case 'applied':
      return (
        <div role="status">
          <p id={leadId} className="pin__lead">
            <Icon name="check" size={14} className="pin__done" />
            <span>
              Pinned. <ProjectName project={project} /> now requires engine <code className="pin__value">{pin.review.command.requirement}</code>{' '}
              (revision {revisionText(pin.receipt.revision)}).
            </span>
          </p>
        </div>
      );
    default:
      return (
        <>
          <div role="status">
            <Summary id={leadId} review={pin.review} project={project} />
          </div>
          <ReviewDetails review={pin.review} />
          {pin.status === 'stale' ? (
            <p className="pin__problem pin__problem--warning" role="alert">
              <Icon name="warning" size={14} className="pin__problem-icon" />
              <span>
                <strong>This review can no longer be used.</strong> {pin.reason}
                {canReview ? ' Review again before pinning.' : ''}
              </span>
            </p>
          ) : null}
          {pin.status === 'failed' ? <Failure operation="apply" failure={pin.failure} hint="A new review is needed to try again." /> : null}
        </>
      );
  }
}

/** The project's name as untrusted text, isolated so its direction cannot reorder the sentence. */
function ProjectName({ project }: { project: string | null }) {
  return project === null ? <>The project</> : <bdi className="pin__name">{reveal(project)}</bdi>;
}

function Summary({ id, review, project }: { id: string; review: PinReview; project: string | null }) {
  return (
    <div id={id} className="pin__summary">
      <p className="pin__lead">
        <span>
          <ProjectName project={project} /> will require engine <code className="pin__value">{review.command.requirement}</code>.
        </span>
      </p>
      <p className="pin__note">Reviewed at revision {revisionText(review.expected_revision)}. The project file changes only when you pin.</p>
    </div>
  );
}

/**
 * The rest of the review, on request: the command and review ID, both checksums, and the
 * file the host would write, as text. Long text waits for "Show all", as approval
 * arguments do; hidden direction characters are shown as escapes.
 */
function ReviewDetails({ review }: { review: PinReview }) {
  const [open, setOpen] = useState(false);
  const [whole, setWhole] = useState(false);
  const regionId = useId();
  const captionId = useId();
  const textRef = useRef<HTMLPreElement>(null);
  const bytes = useMemo(() => utf8Bytes(review.candidate_text), [review.candidate_text]);
  const shown = useMemo(() => {
    if (!open) return null;
    const full = review.candidate_text;
    if (whole || full.length <= CANDIDATE_PREVIEW) return { text: reveal(full), truncated: false };
    // Never end the preview inside a surrogate pair.
    const cut = /[\uD800-\uDBFF]/.test(full.charAt(CANDIDATE_PREVIEW - 1)) ? CANDIDATE_PREVIEW - 1 : CANDIDATE_PREVIEW;
    return { text: reveal(full.slice(0, cut)), truncated: true };
  }, [open, whole, review.candidate_text]);
  return (
    <div className="pin__details">
      <button type="button" className="pin__disclosure" aria-expanded={open} aria-controls={regionId} onClick={() => setOpen((value) => !value)}>
        <Icon name="chevron" size={10} className={`pin__chevron${open ? ' is-open' : ''}`} />
        Review details
      </button>
      {shown ? (
        <div id={regionId} className="pin__detail-body">
          <dl className="pin__facts">
            <dt>Command</dt>
            <dd className="mono">{review.command.op}</dd>
            <dt>Review ID</dt>
            <dd className="mono">{review.review_id}</dd>
            <dt>Current SHA-256</dt>
            <dd className="mono pin__hash">{review.source_sha256}</dd>
            <dt>Pinned SHA-256</dt>
            <dd className="mono pin__hash">{review.candidate_sha256}</dd>
          </dl>
          <p id={captionId} className="pin__caption">
            Pinned file <span className="pin__size">{sizeText(bytes)}</span>
          </p>
          <pre ref={textRef} className="pin__text" tabIndex={0} aria-labelledby={captionId}>
            {shown.text}
          </pre>
          {shown.truncated ? (
            <button
              type="button"
              className="pin__more"
              onClick={() => {
                setWhole(true);
                // The button goes; reading continues in the text rather than outside the dialog.
                textRef.current?.focus();
              }}
            >
              Show all {sizeText(bytes)}
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

/**
 * A refusal in the host's own words, its code beside it, or a result nobody reported,
 * titled as unknown. Never retried by itself.
 */
function Failure({ id, operation, failure, hint }: { id?: string; operation: FileOperation; failure: FileFailure; hint?: string }) {
  return (
    <div id={id} className="pin__problem pin__problem--error" role="alert">
      <Icon name="error" size={14} className="pin__problem-icon" />
      <span className="pin__problem-text">
        <span>
          <strong>{failureTitle(operation, failure)}</strong> {failure.message}
        </span>
        {failure.code !== null ? <span className="pin__code mono">{failure.code}</span> : null}
        {/* Its own line: the host's words are shown as sent, with or without a full stop. */}
        {hint ? <span className="pin__hint">{hint}</span> : null}
      </span>
    </div>
  );
}

function PrimaryAction({
  pin,
  canReview,
  onPin,
  onReview,
}: {
  pin: PinState;
  canReview: boolean;
  onPin: () => void;
  onReview: () => void;
}): ReactNode {
  const status = pin.status;
  if (status === 'requesting' || status === 'ready' || status === 'applying') {
    const waiting = status !== 'ready';
    return (
      <button
        key="pin"
        type="button"
        className="button button--primary"
        aria-disabled={waiting || undefined}
        title={status === 'requesting' ? 'Waiting for the review' : undefined}
        onClick={() => !waiting && onPin()}
      >
        {status === 'applying' ? <span className="spinner spinner--small pin__spinner" aria-hidden="true" /> : null}
        {status === 'applying' ? 'Pinning…' : 'Pin'}
      </button>
    );
  }
  if ((status === 'stale' || status === 'refused' || status === 'failed' || status === 'retired') && canReview) {
    return (
      <button key="review" type="button" className="button" onClick={onReview}>
        Review again
      </button>
    );
  }
  return null;
}
