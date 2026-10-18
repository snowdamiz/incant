/**
 * Plain-language presentation of the native agent session (editor/bridge/contract.ts).
 * Pure functions with no state and no bridge calls. Provider text, tool names and tool
 * arguments are untrusted: they are returned as strings for React to render as text,
 * never as markup.
 */
import type {
  AgentApprovalDecision,
  AgentApprovalMode,
  AgentFailure,
  AgentTurnSnapshot,
  BridgeSnapshot,
  JsonValue,
} from '../../bridge/contract';
import { formatCount } from './tokenAllowance';

/** The native host's defaults (editor/bridge/native.ts). The composer starts with them. */
export const DEFAULT_MODEL = 'gpt-6-astra';
export const DEFAULT_APPROVAL: AgentApprovalMode = 'destructive';
/** The request limits the native bridge enforces, checked here for early feedback. */
export const PROMPT_LIMIT_BYTES = 16 * 1024;
export const MODEL_LIMIT_BYTES = 128;

export const utf8Bytes = (text: string) => new TextEncoder().encode(text).length;

/** The modes the native agent implements (incant_agent ApprovalMode), in menu order. */
export const APPROVAL_MODES: readonly { readonly value: AgentApprovalMode; readonly label: string; readonly detail: string }[] = [
  {
    value: 'destructive',
    label: 'Ask before edits',
    detail: 'The agent asks before it changes the project, imports assets or replaces a script.',
  },
  {
    value: 'always',
    label: 'Ask every time',
    detail: 'The agent asks before every tool it uses, including reads, screenshots and test runs.',
  },
  {
    value: 'auto',
    label: 'Never ask',
    detail: 'The agent edits without asking. Every edit still lands in History and can be undone.',
  },
];

export function approvalModeLabel(mode: string): string {
  return APPROVAL_MODES.find((entry) => entry.value === mode)?.label ?? mode;
}

const ACTIVE = new Set<string>(['running', 'awaiting_approval', 'stopping']);
/** A turn the native worker is still publishing. Unknown statuses are not treated as active. */
export function isActive(status: string): boolean {
  return ACTIVE.has(status);
}

const FAILURES: Readonly<Record<AgentFailure, string>> = {
  provider: 'The model provider did not complete the request.',
  tool: 'A tool the agent used failed.',
  // Neutral: the allowance is the project's on hosts with one, the session's on older hosts.
  budget_exhausted: 'The token allowance ran out.',
  step_limit: 'The agent reached its step limit for one request.',
  project_unavailable: 'The project was not available.',
  project_changed: 'The project changed while the agent was working.',
  worker_panicked: 'The agent stopped unexpectedly.',
  worker_spawn: 'The agent could not start.',
  budget_accounting: 'Token use could not be recorded.',
  output_limit: 'The reply was longer than the output limit.',
  approval_count_limit: 'The agent asked for too many approvals in one request.',
  approval_payload_limit: 'An approval request was too large to review.',
  approval_total_limit: 'This request’s approvals were too large in total.',
  approval_invalid: 'The agent sent an approval request that could not be read.',
  approval_already_pending: 'The agent asked for a second approval before the first was decided.',
  context_limit: 'The conversation is too long for the model.',
};

/** A recorded denial is a tool result, not evidence of why a later turn failed. */
export function failureSentence(turn: Pick<AgentTurnSnapshot, 'failure' | 'approvals'>): string {
  const failure = turn.failure as string | null;
  return failure !== null && Object.hasOwn(FAILURES, failure) ? FAILURES[failure as AgentFailure] : 'The request failed.';
}

export function tokensText(value: number): string {
  return `${formatCount(value)} ${value === 1 ? 'token' : 'tokens'}`;
}

/**
 * What a finished turn cost the allowance. An uncertain charge is marked as estimated;
 * an uncertain turn with no charge is "unknown", never "0 tokens". The allowance itself
 * is read with `allowanceOf` (tokenAllowance.ts).
 */
export function turnCost(turn: Pick<AgentTurnSnapshot, 'charged_tokens' | 'accounting_uncertain'>): string {
  if (!turn.accounting_uncertain) return tokensText(turn.charged_tokens);
  return turn.charged_tokens > 0 ? `${tokensText(turn.charged_tokens)} (estimated)` : 'Token use unknown';
}

export function sizeText(bytes: number): string {
  if (bytes < 1024) return `${bytes} ${bytes === 1 ? 'byte' : 'bytes'}`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export const DECISION_WORDS: Readonly<Record<Exclude<AgentApprovalDecision, 'pending'>, string>> = {
  allowed: 'Allowed',
  denied: 'Denied',
  cancelled: 'Cancelled',
};

/* ---- Approval review ---- */

/** A run of review text: plain words, or an untrusted name shown isolated (and mono for IDs and paths). */
export type ReviewPart = string | { readonly name: string; readonly mono?: boolean };
export type ReviewKind = 'add' | 'remove' | 'change' | 'unknown';
export interface ReviewRow {
  readonly kind: ReviewKind;
  readonly parts: readonly ReviewPart[];
}
export interface ApprovalReview {
  /** The request as a question, e.g. "Apply 3 changes to the project?" */
  readonly title: readonly ReviewPart[];
  /** The agent's own description of its change, when the tool takes one. */
  readonly description: string | null;
  /** What the arguments actually do, one row per operation. */
  readonly rows: readonly ReviewRow[];
  /** Operations not listed in `rows`. */
  readonly more: number;
}

/** Looks up names the project already has, so a review can say "Crate 03" for an ID. */
export interface Names {
  readonly entity: (id: string) => string | undefined;
  readonly asset: (id: string) => string | undefined;
}

export function namesFrom(hierarchy: BridgeSnapshot['hierarchy'] | undefined, projectAssets: BridgeSnapshot['assets']): Names {
  const nodes = hierarchy?.status === 'ready' ? hierarchy.value.nodes : {};
  const assets = projectAssets?.status === 'ready' ? projectAssets.value : [];
  return {
    entity: (id) => (Object.hasOwn(nodes, id) ? nodes[id]?.name : undefined),
    asset: (id) => assets.find((asset) => asset.id === id)?.name,
  };
}

/** The review shows at most this many operations; the exact arguments list every one. */
export const REVIEW_ROW_LIMIT = 8;
const NAME_LIMIT = 120;
const COMPONENT_LIMIT = 6;

/**
 * Bidirectional overrides and zero-width characters in untrusted text could make a
 * name read differently from what it is. A review shows them as visible escapes.
 */
const HIDDEN = /[؜​-‏‪-‮⁠-⁤⁦-⁩﻿]/g;
export function reveal(text: string): string {
  return text.replace(HIDDEN, (char) => `\\u${char.charCodeAt(0).toString(16).toUpperCase().padStart(4, '0')}`);
}

function clip(text: string): string {
  const shown = reveal(text);
  return shown.length > NAME_LIMIT ? `${shown.slice(0, NAME_LIMIT - 1)}…` : shown;
}

const record = (value: JsonValue | undefined): { readonly [key: string]: JsonValue } | null =>
  value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as { readonly [key: string]: JsonValue }) : null;
const text = (value: JsonValue | undefined): string | null => (typeof value === 'string' ? value : null);
const finite = (value: JsonValue | undefined): number | null => (typeof value === 'number' && Number.isFinite(value) ? value : null);

const name = (value: string): ReviewPart => ({ name: clip(value) });
const id = (value: string): ReviewPart => ({ name: clip(value), mono: true });
/** A project entity (or scene) by ID: its current name when the project has one, else the ID. */
function entity(names: Names, value: JsonValue | undefined): ReviewPart {
  const raw = text(value);
  if (raw === null) return 'an unnamed entity';
  const known = names.entity(raw);
  return known !== undefined ? name(known) : id(raw);
}
function asset(names: Names, value: JsonValue | undefined): ReviewPart {
  const raw = text(value);
  if (raw === null) return 'an asset';
  const known = names.asset(raw);
  return known !== undefined ? name(known) : id(raw);
}
const plural = (n: number, one: string, many: string) => `${n.toLocaleString()} ${n === 1 ? one : many}`;

function describeOperation(op: JsonValue, names: Names): ReviewRow {
  const fields = record(op);
  const kind = text(fields?.op);
  if (!fields || kind === null) return { kind: 'unknown', parts: ['An operation this editor cannot read'] };
  switch (kind) {
    case 'create_scene': {
      const sceneName = text(record(fields.scene)?.name);
      return { kind: 'add', parts: ['Create scene ', sceneName === null ? 'without a name' : name(sceneName)] };
    }
    case 'delete_scene':
      return { kind: 'remove', parts: ['Delete scene ', entity(names, fields.scene_id)] };
    case 'create_entity': {
      const created = record(fields.entity);
      const entityName = text(created?.name);
      const components = Object.keys(record(created?.components) ?? {});
      const listed = components.slice(0, COMPONENT_LIMIT).map(clip).join(', ');
      const rest = components.length > COMPONENT_LIMIT ? ` and ${components.length - COMPONENT_LIMIT} more` : '';
      return {
        kind: 'add',
        parts: ['Create ', entityName === null ? 'an unnamed entity' : name(entityName), ...(components.length ? [` with ${listed}${rest}`] : [])],
      };
    }
    case 'delete_entity':
      return { kind: 'remove', parts: ['Delete ', entity(names, fields.entity_id)] };
    case 'rename_entity': {
      const next = text(fields.name);
      return { kind: 'change', parts: ['Rename ', entity(names, fields.entity_id), ' to ', next === null ? 'a missing name' : name(next)] };
    }
    case 'reparent_entity':
      return {
        kind: 'change',
        parts: ['Move ', entity(names, fields.entity_id), ' under ', fields.parent === null || fields.parent === undefined ? 'the scene root' : entity(names, fields.parent)],
      };
    case 'set_component':
      return { kind: 'change', parts: ['Set ', name(text(fields.component) ?? 'a component'), ' on ', entity(names, fields.entity_id)] };
    case 'remove_component':
      return { kind: 'remove', parts: ['Remove ', name(text(fields.component) ?? 'a component'), ' from ', entity(names, fields.entity_id)] };
    case 'upsert_asset': {
      const path = text(record(fields.asset)?.path);
      return { kind: 'change', parts: ['Register asset ', path === null ? 'without a path' : id(path)] };
    }
    case 'remove_asset':
      return { kind: 'remove', parts: ['Remove asset ', asset(names, fields.asset_id)] };
    case 'upsert_script': {
      const path = text(record(fields.script)?.path);
      return { kind: 'change', parts: ['Register script ', path === null ? 'without a path' : id(path)] };
    }
    case 'remove_script':
      return { kind: 'remove', parts: ['Remove script ', id(text(fields.script_id) ?? 'without an ID')] };
    case 'write_script': {
      const source = text(fields.source);
      return {
        kind: 'change',
        parts: ['Replace script ', id(text(fields.script_id) ?? 'without an ID'), ...(source === null ? [] : [` (${sizeText(utf8Bytes(source))})`])],
      };
    }
    case 'upsert_module_source': {
      const path = text(record(fields.module)?.path);
      return { kind: 'change', parts: ['Register Rust source ', path === null ? 'without a path' : id(path)] };
    }
    case 'remove_module_source':
      return { kind: 'remove', parts: ['Unregister Rust source ', id(text(fields.module_id) ?? 'without an ID'), ' (file retained)'] };
    case 'write_module_source': {
      const source = text(fields.source);
      return {
        kind: 'change',
        parts: ['Replace Rust source ', id(text(fields.module_id) ?? 'without an ID'), ...(source === null ? [] : [` (${sizeText(utf8Bytes(source))})`])],
      };
    }
    case 'set_memory':
      return { kind: 'change', parts: ['Update project memory ', name(text(fields.section) ?? 'without a section')] };
    case 'upsert_string_table': {
      const table = text(record(fields.table)?.name);
      return { kind: 'change', parts: ['Update string table ', table === null ? 'without a name' : name(table)] };
    }
    case 'remove_string_table':
      return { kind: 'remove', parts: ['Remove string table ', id(text(fields.table_id) ?? 'without an ID')] };
    case 'set_locale': {
      const locale = text(record(fields.settings)?.locale);
      return { kind: 'change', parts: ['Set the locale to ', locale === null ? 'a missing value' : id(locale)] };
    }
    case 'set_input_actions':
      return { kind: 'change', parts: ['Replace the input actions'] };
    default:
      return { kind: 'unknown', parts: ['Unrecognised operation ', id(kind)] };
  }
}

const MALFORMED: ReviewRow = { kind: 'unknown', parts: ['The arguments do not match this tool. Check them before allowing.'] };

/** A concise, truthful reading of one pending approval's exact arguments. */
export function reviewApproval(tool: string, args: JsonValue, names: Names): ApprovalReview {
  const fields = record(args);
  const description = text(fields?.description);
  const quoted = description !== null && description.trim() ? reveal(description.trim()) : null;
  const review = (title: readonly ReviewPart[], rows: readonly ReviewRow[] = [], more = 0): ApprovalReview => ({
    title,
    description: quoted,
    rows,
    more,
  });
  switch (tool) {
    case 'doc_patch': {
      const ops = Array.isArray(fields?.ops) ? (fields.ops as readonly JsonValue[]) : null;
      if (!ops) return review(['Change the project?'], [MALFORMED]);
      const rows = ops.slice(0, REVIEW_ROW_LIMIT).map((op) => describeOperation(op, names));
      return review([`Apply ${plural(ops.length, 'change', 'changes')} to the project?`], rows, Math.max(0, ops.length - REVIEW_ROW_LIMIT));
    }
    case 'asset_import': {
      const sources = Array.isArray(fields?.sources) ? (fields.sources as readonly JsonValue[]) : null;
      if (!sources) return review(['Import assets?'], [MALFORMED]);
      const rows = sources.slice(0, REVIEW_ROW_LIMIT).map((source): ReviewRow => {
        const entry = record(source);
        const path = text(entry?.source);
        const usage = text(entry?.texture_usage);
        if (path === null) return MALFORMED;
        return { kind: 'add', parts: ['Import ', id(path), ...(usage ? [' as ', name(usage)] : [])] };
      });
      return review([`Import ${plural(sources.length, 'asset', 'assets')}?`], rows, Math.max(0, sources.length - REVIEW_ROW_LIMIT));
    }
    case 'script_write': {
      const script = text(fields?.script_id);
      const source = text(fields?.source);
      if (script === null || source === null) return review(['Replace a script?'], [MALFORMED]);
      return review(['Replace the source of script ', id(script), '?'], [{ kind: 'change', parts: [`New source, ${sizeText(utf8Bytes(source))}`] }]);
    }
    case 'doc_query': {
      const path = text(fields?.path);
      if (path === null) return review(['Read project data?'], [MALFORMED]);
      return review(path === '' ? ['Read the whole project?'] : ['Read project data at ', id(path), '?']);
    }
    case 'doc_schema': {
      const component = text(fields?.component);
      return component === null ? review(['Read a component schema?'], [MALFORMED]) : review(['Read the ', name(component), ' schema?']);
    }
    case 'view_screenshot': {
      const width = finite(fields?.width);
      const height = finite(fields?.height);
      const camera = fields?.camera;
      const size = width !== null && height !== null ? `${width} × ${height} ` : '';
      return review([
        `Capture a ${size}screenshot from `,
        camera === null || camera === undefined ? 'the editor preview' : entity(names, camera),
        '?',
      ]);
    }
    case 'asset_list': {
      const filter = text(fields?.filter);
      return review(filter ? ['List assets matching ', name(filter), '?'] : ['List the project’s assets?']);
    }
    case 'asset_inspect':
      return review(['Inspect asset ', asset(names, fields?.id), '?']);
    case 'play_run': {
      const seconds = finite(fields?.seconds);
      const scene = fields?.scene;
      const rows: ReviewRow[] = [];
      if (record(fields?.inputs_script)) rows.push({ kind: 'change', parts: ['Replays recorded input'] });
      if (record(fields?.observe)) rows.push({ kind: 'change', parts: ['Checks selected runtime values'] });
      return review(
        [
          'Run ',
          scene === null || scene === undefined ? 'the game' : entity(names, scene),
          seconds === null ? '?' : ` for ${seconds.toLocaleString()} s?`,
        ],
        rows,
      );
    }
    case 'play_state_query': {
      const path = text(fields?.path);
      return path === null ? review(['Read a runtime value?'], [MALFORMED]) : review(['Read the runtime value at ', id(path), '?']);
    }
    case 'script_read':
      return review(['Read script ', id(text(fields?.script_id) ?? 'without an ID'), '?']);
    default:
      return review(['Use the tool ', id(tool), '?']);
  }
}

/** The tool as a short noun phrase, for a decided approval whose review was never shown here. */
export function toolPhrase(tool: string): readonly ReviewPart[] {
  switch (tool) {
    case 'doc_patch':
      return ['Project changes'];
    case 'asset_import':
      return ['Asset import'];
    case 'script_write':
      return ['Script replacement'];
    case 'doc_query':
      return ['Project read'];
    case 'doc_schema':
      return ['Schema read'];
    case 'view_screenshot':
      return ['Screenshot'];
    case 'asset_list':
      return ['Asset list'];
    case 'asset_inspect':
      return ['Asset inspection'];
    case 'play_run':
      return ['Game run'];
    case 'play_state_query':
      return ['Runtime read'];
    case 'script_read':
      return ['Script read'];
    default:
      return ['Tool ', id(tool)];
  }
}

/** The title as a statement ("Apply 3 changes to the project"), for decided rows. */
export function asStatement(parts: readonly ReviewPart[]): readonly ReviewPart[] {
  const last = parts.at(-1);
  if (typeof last !== 'string' || !last.endsWith('?')) return parts;
  const trimmed = last.slice(0, -1);
  return trimmed ? [...parts.slice(0, -1), trimmed] : parts.slice(0, -1);
}

export function plainText(parts: readonly ReviewPart[]): string {
  return parts.map((part) => (typeof part === 'string' ? part : part.name)).join('');
}

/**
 * The exact arguments as indented JSON with hidden characters revealed. Bounded: the
 * first `limit` characters, and whether more exist.
 */
export function argumentsText(args: JsonValue, limit: number): { readonly text: string; readonly truncated: boolean; readonly length: number } {
  const full = reveal(JSON.stringify(args, null, 2) ?? 'null');
  return full.length > limit ? { text: full.slice(0, limit), truncated: true, length: full.length } : { text: full, truncated: false, length: full.length };
}
