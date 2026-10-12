/**
 * The agent's token allowance as the pane presents it (editor/bridge/contract.ts). Pure
 * functions with no state and no bridge calls.
 *
 * Two sources, never mixed: the allowance is the project's authored cap
 * (`projectAgentBudget`) where the host has one, otherwise (older hosts) the session's
 * own budget. Usage is always the native session's recorded total. While the session
 * catches up with a changed cap its `max_tokens` can differ from the authored cap; the
 * authored cap is the allowance then, and the session's stale copy is never shown.
 */
import type { AgentSessionSnapshot, BridgeSnapshot } from '../../bridge/contract';

/** The snapshot fields the allowance is read from. */
export interface AllowanceSource {
  readonly connection: BridgeSnapshot['connection'];
  readonly projectAgentBudget?: BridgeSnapshot['projectAgentBudget'] | undefined;
  readonly agentSession?: AgentSessionSnapshot | undefined;
}

/** The largest allowance the host accepts: the largest JavaScript safe integer. */
export const MAX_ALLOWANCE = Number.MAX_SAFE_INTEGER;

export type UsageLevel = 'normal' | 'low' | 'out' | 'off';

export interface Allowance {
  /** `project`: the project's authored cap. `session`: an older host's per-session budget. */
  readonly scope: 'project' | 'session';
  readonly cap: number;
  /** Tokens recorded as used; null until the host has read the session. */
  readonly used: number | null;
  /** Some recorded usage is a conservative charge, not an exact count. */
  readonly uncertain: boolean;
  /** `cap − used`, never below zero (a lowered cap can be under what is already used). */
  readonly left: number | null;
  /** Used at least the allowance: the host refuses new requests. */
  readonly exhausted: boolean;
  readonly level: UsageLevel;
}

/** A count in the reader's own digit grouping. */
export const formatCount = (value: number) => value.toLocaleString();

export function usageLevel(used: number | null, cap: number): UsageLevel {
  if (cap === 0) return 'off';
  if (used === null) return 'normal';
  return used >= cap ? 'out' : used >= cap * 0.8 ? 'low' : 'normal';
}

/** "6,532 of 1,000,000 tokens used", marked when some of it is a conservative charge. */
export function usageText(used: number, cap: number, uncertain: boolean): string {
  return `${formatCount(used)} of ${formatCount(cap)} tokens used${uncertain ? ' (some estimated)' : ''}`;
}

/** The allowance of the open project, or null when there is no project or nothing to show. */
export function allowanceOf(snapshot: AllowanceSource | null | undefined): Allowance | null {
  if (!snapshot || snapshot.connection.status !== 'ready') return null;
  const session = snapshot.agentSession;
  const authored = snapshot.projectAgentBudget?.maxTokens;
  const cap = authored ?? session?.budget.max_tokens;
  if (cap === undefined) return null;
  const used = session ? session.budget.used_tokens : null;
  return {
    scope: authored === undefined ? 'session' : 'project',
    cap,
    used,
    uncertain: session?.accounting_uncertain ?? false,
    left: used === null ? null : Math.max(0, cap - used),
    exhausted: used !== null && used >= cap,
    level: usageLevel(used, cap),
  };
}

/** The allowance line's visible words. */
export function allowanceText(allowance: Allowance): string {
  return allowance.used === null ? `Allowance: ${formatCount(allowance.cap)} tokens` : usageText(allowance.used, allowance.cap, allowance.uncertain);
}

const UNCERTAIN = 'Some usage could not be confirmed or saved exactly, so a conservative amount was recorded.';

function leftText(allowance: Allowance): string {
  return allowance.left === null ? '' : `${formatCount(allowance.left)} ${allowance.left === 1 ? 'token' : 'tokens'} left.`;
}

/** What the line does not show, said after it: what is left, and why some is estimated. */
export function allowanceSpoken(allowance: Allowance): string {
  return [leftText(allowance), allowance.uncertain ? UNCERTAIN : ''].filter(Boolean).join(' ');
}

/** The line's tooltip: what is left, where the allowance and its usage belong, and why some is estimated. */
export function allowanceTitle(allowance: Allowance): string {
  const scope =
    allowance.scope === 'project'
      ? 'Usage is recorded on this computer for this project and kept across restarts and account changes.'
      : 'This allowance belongs to the current agent session.';
  return [leftText(allowance), scope, allowance.uncertain ? UNCERTAIN : ''].filter(Boolean).join(' ');
}

/* ---- Editing ---- */

/** `format`: anything but a whole number (a fraction, an exponent, a sign, letters, uneven groups). */
export type AllowanceProblem = 'empty' | 'negative' | 'format' | 'unsafe';
export type AllowanceInput = { readonly ok: true; readonly value: number } | { readonly ok: false; readonly problem: AllowanceProblem };

/** Digit-group separators people type: comma, period, space, apostrophes and underscore. */
const SEPARATORS = new Set([',', '.', ' ', '\'', '’', '_']);
/** Groups of three (1,000,000) or, as written in India, of two before the last three (10,00,000). Nothing in between. */
const GROUPED = /^(?:\d{1,3}(?:,\d{3})+|\d{1,2}(?:,\d{2})+,\d{3})$/;

const decimalMarks = new Map<string, string>();
function decimalMark(locale: string | undefined): string {
  const key = locale ?? '';
  let mark = decimalMarks.get(key);
  if (mark === undefined) {
    mark = new Intl.NumberFormat(locale).formatToParts(1.5).find((part) => part.type === 'decimal')?.value ?? '.';
    decimalMarks.set(key, mark);
  }
  return mark;
}

/**
 * Reads a typed allowance: plain digits, or digits grouped by one consistent separator
 * that is not the locale's decimal mark ("1,000,000"; "1.000.000" where the decimal mark
 * is a comma). Fractions, signs, exponents, uneven groups and values past the largest
 * safe integer are refused with a reason; nothing is rounded or clamped.
 */
export function parseAllowance(text: string, locale?: string): AllowanceInput {
  // NFKC folds full-width digits and no-break spaces into their plain forms.
  const raw = text.normalize('NFKC').trim();
  if (!raw) return { ok: false, problem: 'empty' };
  if (/^[-−]/.test(raw)) return { ok: false, problem: 'negative' };
  let digits = /^\d+$/.test(raw) ? raw : null;
  if (digits === null) {
    const separators = new Set([...raw].filter((char) => !/\d/.test(char)));
    const [separator] = separators;
    if (separators.size === 1 && separator !== undefined && SEPARATORS.has(separator) && separator !== decimalMark(locale)) {
      const groups = raw.split(separator);
      if (GROUPED.test(groups.join(','))) digits = groups.join('');
    }
  }
  if (digits === null) return { ok: false, problem: 'format' };
  const value = Number(digits);
  return Number.isSafeInteger(value) ? { ok: true, value } : { ok: false, problem: 'unsafe' };
}

export function problemText(problem: AllowanceProblem): string {
  switch (problem) {
    case 'empty':
      return 'Enter a number of tokens.';
    case 'negative':
      return 'The allowance can’t be negative.';
    case 'unsafe':
      return `The largest allowance is ${formatCount(MAX_ALLOWANCE)} tokens.`;
    case 'format':
      return 'Enter a whole number of tokens.';
  }
}
