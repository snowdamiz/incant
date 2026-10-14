/** Native transport for the Claude-owned UI contract. All document writes use incant_cmd. */
import type { ProjectSchema } from '../../sdk/ts/src/generated';
import type {
  BridgeSnapshot,
  BridgeError,
  BridgeResult,
  EditorBridge,
  EditorCommand,
  HostRequest,
  Ulid,
  HierarchyNode,
  EntityDetail,
  ComponentSchema,
  FieldSchema,
  Origin,
  WindowChrome,
  ProviderState,
  PlayStatus,
  PlayLogBatch,
  ConsoleEntry,
  PlayEntity,
  PlayRequest,
  AgentState,
  AgentSessionSnapshot,
  AgentTurnRequest,
  ProjectFileRequest,
  ProjectFileResponse,
} from "./contract";

type Entity = {
  id: string;
  name: string;
  parent: string | null;
  components: Record<string, Record<string, unknown>>;
};
type Project = {
  id: string;
  name: string;
  settings?: unknown;
  assets?: Record<string, ProjectSchema.Asset>;
  scenes: Record<
    string,
    { id: string; name: string; entities: Record<string, Entity> }
  >;
};
type History = {
  id: string;
  description: string;
  actor: {
    origin: "user" | "agent" | "script" | "import";
    actor: string;
    model: string | null;
    conversation_id: string | null;
  };
};
export interface EngineRead {
  status?: "ready";
  project: Project;
  revision: number;
  can_redo: boolean;
  history: History[];
  applied: number;
  schemas: Record<
    string,
    { title?: string; properties?: Record<string, Record<string, unknown>>;
      $defs?: Record<string, Record<string, unknown>>; required?: readonly string[];
      order?: readonly string[] }
  >;
  console: { id: string; level: "info" | "error"; message: string }[];
  viewport_error: string | null;
  source_diagnostics?: { asset_id: string; source: string; message: string }[];
  asset_import?: { available: boolean; reason?: string };
  /** Explicitly present only on hosts with the durable budget adapter. */
  agent_budget?: { available: boolean };
  project_file?: { available: boolean };
  /** Optional on older hosts; their Start request remains authoritative. */
  play_start?: { available: boolean; reason?: string };
}
export type EngineResponse = EngineRead
  | { status: "loading" }
  | { status: "error"; error: BridgeError };
interface EngineImportResponse {
  snapshot: EngineRead;
  commit: { revision: number; transaction: string | null };
}
const fileId = (value: unknown): value is string => typeof value === 'string' && /^[0-7][0-9A-HJKMNP-TV-Z]{25}$/.test(value);
const fileHash = (value: unknown): value is string => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
function projectFileResponse(raw: unknown, operation: string, revision: number): ProjectFileResponse | undefined {
  if (!record(raw) || raw.operation !== operation) return;
  if (operation === 'review_pin') {
    const r = raw.review;
    if (!record(r) || !fileId(r.review_id) || r.expected_revision !== revision
      || !fileHash(r.source_sha256) || !fileHash(r.candidate_sha256)
      || typeof r.candidate_text !== 'string' || new TextEncoder().encode(r.candidate_text).length > 32 * 1024 * 1024
      || !record(r.command) || r.command.op !== 'set_engine_version'
      || typeof r.command.requirement !== 'string' || !/^=[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(r.command.requirement)) return;
  } else if (operation === 'apply_pin') {
    if (!fileId(raw.transaction_id) || raw.revision !== revision + 1) return;
  } else if (operation === 'save') {
    const r = raw.receipt;
    if (!record(r) || r.revision !== revision || typeof r.changed !== 'boolean'
      || !fileHash(r.source_sha256) || !fileHash(r.candidate_sha256)) return;
  } else return;
  return raw as unknown as ProjectFileResponse;
}
function isReady(read: EngineResponse): read is EngineRead {
  return read.status === undefined || read.status === "ready";
}
/** Resolve local definitions, nullable fields and explicit tagged object unions for display.
 * Recursive/unknown schema forms remain explicit unsupported fields. No fetches.
 */
function inspectorField(
  value: unknown, defs: Record<string, Record<string, unknown>>,
  depth = 0,
): FieldSchema {
  const unsupported: FieldSchema = { type: "unsupported" };
  if (depth > 16 || !value || typeof value !== "object" || Array.isArray(value)) return unsupported;
  const raw = value as Record<string, unknown>;
  const metadata = {
    ...(typeof raw.title === "string" ? { title: raw.title } : {}),
    ...(typeof raw.description === "string" ? { description: raw.description } : {}),
    ...(Object.hasOwn(raw, "default") ? { default: raw.default } : {}),
  };
  if (typeof raw.$ref === "string") {
    const name = raw.$ref.startsWith("#/$defs/") ? raw.$ref.slice(8) : "";
    const target = Object.hasOwn(defs, name) ? defs[name] : undefined;
    return target ? { ...inspectorField(target, defs, depth + 1), ...metadata } : unsupported;
  }
  if (Array.isArray(raw.anyOf)) {
    const alternatives = raw.anyOf as Record<string, unknown>[];
    if (alternatives.length !== 2 || alternatives.some(v => !v || typeof v !== "object")) return unsupported;
    const concrete = alternatives.filter(v => v.type !== "null");
    if (concrete.length !== 1) return unsupported;
    return { ...inspectorField(concrete[0]!, defs, depth + 1), ...metadata, nullable: true };
  }
  if (Array.isArray(raw.oneOf)) {
    const variants = raw.oneOf as Record<string, unknown>[];
    if (variants.length < 2 || variants.length > 32 || variants.some(v => !v || v.type !== 'object' || !v.properties || typeof v.properties !== 'object' || !Array.isArray(v.required))) return unsupported;
    const first = variants[0]!.properties as Record<string, Record<string, unknown>>;
    const tags = Object.keys(first).filter(key => variants.every(v => {
      const property = (v.properties as Record<string, Record<string, unknown>>)[key];
      return (v.required as unknown[]).includes(key) && property?.type === 'string' && typeof property.const === 'string';
    }));
    const discriminator = tags.find(key => new Set(variants.map(v => (v.properties as Record<string, Record<string, unknown>>)[key]!.const)).size === variants.length);
    if (!discriminator) return unsupported;
    return { type: 'tagged-union', ...metadata, discriminator,
      variants: Object.fromEntries(variants.map(v => [
        String((v.properties as Record<string, Record<string, unknown>>)[discriminator]!.const), inspectorField(v, defs, depth + 1),
      ])),
    };
  }
  if (Array.isArray(raw.type)) {
    // Schemars emits nullable primitives as type: [T, "null"]. Preserve
    // constraints and recurse so nullable objects/array items resolve normally.
    if (raw.type.length !== 2 || !raw.type.includes("null")) return unsupported;
    const concrete = raw.type.find(type => type !== "null");
    if (typeof concrete !== "string" || !["boolean", "integer", "number", "string", "array", "object"].includes(concrete)) return unsupported;
    return { ...inspectorField({ ...raw, type: concrete }, defs, depth + 1), nullable: true };
  }
  if (typeof raw.type !== "string") return unsupported;
  const field = { ...raw };
  if (raw.type === "string" && typeof raw.const === "string") field.enum = [raw.const];
  if (raw.type === "object" && raw.properties && typeof raw.properties === "object") {
    const required = Array.isArray(raw.required) ? raw.required : [];
    field.properties = Object.fromEntries(Object.entries(raw.properties).map(([key, child]) => [key, {
      ...inspectorField(child as Record<string, unknown>, defs, depth + 1),
      optional: !required.includes(key),
    }]));
  } else if (raw.type === "array" && raw.items && typeof raw.items === "object") {
    field.items = inspectorField(raw.items as Record<string, unknown>, defs, depth + 1);
  }
  return field as FieldSchema;
}
export type Invoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;
type Listen = <T>(
  event: string,
  listener: (event: { payload: T }) => void,
) => Promise<() => void>;
const id = (value: string) => value as Ulid;
function freeze<T>(value: T): T {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}
function timestamp(ulid: string): string {
  const alphabet = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  const value = [...ulid.slice(0, 10)].reduce(
    (n, c) => n * 32 + alphabet.indexOf(c),
    0,
  );
  return new Date(value).toISOString();
}
function origin(actor: History["actor"]): Origin {
  switch (actor.origin) {
    case "agent":
      return {
        kind: "agent",
        model: actor.model ?? "unknown",
        conversation: actor.conversation_id ?? "unknown",
      };
    case "script":
      return { kind: "script", script: actor.actor };
    case "import":
      return { kind: "import", source: actor.actor };
    default:
      return { kind: "user" };
  }
}
export function snapshotFromEngine(read: EngineResponse): BridgeSnapshot {
  if (!isReady(read)) {
    const error = read.status === "error" ? read.error : undefined;
    return {
      connection: error ? { status: "error", error } : { status: "connecting" },
      hierarchy: error ? { status: "error", error } : { status: "loading" },
      schemas: {}, entities: {}, history: { entries: [], applied: 0 }, console: [],
      assets: error ? { status: 'error', error } : { status: 'loading' },
      assetImport: { available: false, reason: error?.message ?? 'Loading project.' },
      diagnostics: error ? [{ id: "project-load", severity: "error", message: error.message,
        entity: null, component: null, path: null }] : [],
      provider: { status: "not-connected", provider: "openai" },
      agent: { status: "unavailable", reason: error?.message ?? "Loading project." },
      viewport: error ? { status: "error", error }
        : { status: "not-attached", reason: "Loading project." },
    };
  }
  const budget = projectAgentBudget(read);
  const playStart = playStartAvailability(read);
  const nodes: Record<string, HierarchyNode> = {};
  const entities: Record<string, EntityDetail> = {};
  const roots: Ulid[] = [];
  for (const scene of Object.values(read.project.scenes)) {
    roots.push(id(scene.id));
    nodes[scene.id] = {
      id: id(scene.id),
      name: scene.name,
      kind: "scene",
      parent: null,
      children: Object.values(scene.entities)
        .filter((e) => !e.parent)
        .map((e) => id(e.id)),
    };
    entities[scene.id] = {
      id: id(scene.id),
      name: scene.name,
      kind: "scene",
      components: [],
    };
    const children = new Map<string, Ulid[]>();
    for (const entity of Object.values(scene.entities)) {
      if (entity.parent)
        children.set(entity.parent, [
          ...(children.get(entity.parent) ?? []),
          id(entity.id),
        ]);
    }
    for (const entity of Object.values(scene.entities)) {
      const kind =
        "Camera" in entity.components
          ? "camera"
          : "MeshRenderer" in entity.components
            ? "mesh"
            : "Script" in entity.components
              ? "script"
              : "entity";
      nodes[entity.id] = {
        id: id(entity.id),
        name: entity.name,
        kind,
        parent: id(entity.parent ?? scene.id),
        children: children.get(entity.id) ?? [],
      };
      entities[entity.id] = {
        id: id(entity.id),
        name: entity.name,
        kind,
        components: Object.entries(entity.components).map(([type, value]) => ({
          type,
          schemaVersion: 1,
          value,
        })),
      };
    }
  }
  const schemas: Record<string, ComponentSchema> = {};
  for (const [type, schema] of Object.entries(read.schemas)) {
    schemas[type] = {
      type,
      version: 1,
      title: schema.title ?? type,
      properties: Object.fromEntries(Object.entries(schema.properties ?? {}).map(([key, field]) => [key, {
        ...inspectorField(field, schema.$defs ?? {}),
        optional: schema.required !== undefined && !schema.required.includes(key),
      }])),
      ...(schema.order ? { order: schema.order } : {}),
    };
  }
  return {
    connection: {
      status: "ready",
      project: { id: id(read.project.id), name: read.project.name },
    },
    hierarchy: { status: "ready", value: { roots, nodes } },
    schemas,
    entities,
    ...(budget.status === 'ready' ? { projectAgentBudget: { maxTokens: budget.maxTokens } } : {}),
    ...(read.project.assets ? { assets: { status: 'ready' as const, value: Object.values(read.project.assets).map((asset) => ({
      id: id(asset.id), name: asset.name, path: asset.path, kind: asset.kind, fingerprint: asset.sha256,
      ...(asset.import_settings?.type === 'texture' ? { textureUsage: asset.import_settings.usage } : {}),
    })) } } : {}),
    assetImport: read.asset_import ?? { available: false, reason: 'This host does not expose asset importing.' },
    ...(playStart ? { playStart } : {}),
    diagnostics: [
      ...(budget.status === 'error' ? [{ id: 'agent-budget', severity: 'error' as const,
        message: budget.error.message, entity: null, component: null, path: 'settings.agent_budget' }] : []),
      ...(read.viewport_error ? [{ id: 'native-viewport', severity: 'error' as const,
        message: read.viewport_error, entity: null, component: null, path: null }] : []),
      ...(read.source_diagnostics ?? []).map((issue) => ({
        id: `asset-source:${issue.asset_id}`, severity: 'error' as const,
        message: issue.message, entity: null, component: null, path: issue.source,
      })),
    ],
    history: {
      entries: read.history.map((tx) => ({
        transaction: id(tx.id),
        description: tx.description,
        origin: origin(tx.actor),
        at: timestamp(tx.id),
      })),
      applied: read.applied,
    },
    console: read.console.map((event) => ({
      ...event,
      source: "native",
      at: timestamp(event.id),
    })),
    provider: { status: "not-connected", provider: "openai" },
    agent: {
      status: "unavailable",
      reason: "The agent is not available in the editor yet.",
    },
    viewport: read.viewport_error
      ? {
          status: "error",
          error: { code: "viewport.failed", message: read.viewport_error },
        }
      : { status: "attached", surface: "Native wgpu" },
  };
}
function failure(error: unknown): BridgeResult {
  if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error && typeof error.code === 'string' && typeof error.message === 'string') {
    return { ok: false, error: { code: error.code, message: error.message } };
  }
  return {
    ok: false,
    error: {
      code: "engine.request",
      message: typeof error === "string" ? error : "The engine request failed.",
    },
  };
}
export interface EngineAgentRead {
  /** Null means the requested session version is unchanged. */
  readonly session: AgentSessionSnapshot | null;
  readonly project_revision: number | null;
}
type AgentRequest = Extract<HostRequest, { type: `agent.${string}` }>;
const AGENT_ACTIVE = ['running', 'awaiting_approval', 'stopping'];
const AGENT_FAILURES = ['provider', 'tool', 'budget_exhausted', 'step_limit', 'project_unavailable',
  'project_changed', 'worker_panicked', 'worker_spawn', 'budget_accounting', 'output_limit',
  'approval_count_limit', 'approval_payload_limit', 'approval_total_limit', 'approval_invalid',
  'approval_already_pending', 'context_limit'];
const bytes = (value: string) => new TextEncoder().encode(value).length;
const count = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
const sequence = (value: unknown): value is number => count(value) && value > 0;
const boundedText = (value: unknown, limit = 128): value is string => typeof value === 'string' && value.length <= limit && bytes(value) <= limit;
function fields(value: unknown, keys: readonly string[]): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null)
    && Object.keys(value).length === keys.length && keys.every(key => Object.hasOwn(value, key));
}

type ProjectAgentBudget = { status: 'unsupported' } | { status: 'ready'; maxTokens: number }
  | { status: 'error'; error: BridgeError };
const budgetInvalidResponse = (): BridgeError => ({ code: 'engine.invalid-response', message: 'The project’s agent token allowance is invalid. Refresh the project to retry.' });
function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}
function playStartAvailability(read: EngineRead): BridgeSnapshot['playStart'] {
  if (!Object.hasOwn(read, 'play_start')) return undefined;
  const value = read.play_start;
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || typeof value.available !== 'boolean'
    || (Object.hasOwn(value, 'reason') && (typeof value.reason !== 'string' || !value.reason.trim()))) {
    return { available: false, reason: 'Native Play availability is invalid. Reopen the project to retry.' };
  }
  return { available: value.available, ...(value.reason ? { reason: value.reason } : {}) };
}

/** Defaults require both the real host adapter and its published document schema. */
function projectAgentBudget(read: EngineRead): ProjectAgentBudget {
  if (!Object.hasOwn(read, 'agent_budget')) return { status: 'unsupported' };
  const invalid = (): ProjectAgentBudget => ({ status: 'error', error: budgetInvalidResponse() });
  if (!fields(read.agent_budget, ['available']) || typeof read.agent_budget.available !== 'boolean') return invalid();
  if (!read.agent_budget.available) return { status: 'unsupported' };
  const schema = read.schemas.Project;
  const defs = schema?.$defs;
  const settingsSchema = defs?.ProjectSettings;
  const budgetSchema = defs?.AgentBudgetSettings;
  if (!record(settingsSchema) || settingsSchema.type !== 'object' || !record(settingsSchema.properties)
    || schema?.properties?.settings?.$ref !== '#/$defs/ProjectSettings'
    || !record(budgetSchema) || budgetSchema.type !== 'object' || budgetSchema.additionalProperties !== false
    || !Array.isArray(budgetSchema.required) || !budgetSchema.required.includes('max_tokens') || !record(budgetSchema.properties)) return invalid();
  const budgetProperty = settingsSchema.properties.agent_budget;
  const capSchema = budgetSchema.properties.max_tokens;
  if (!record(budgetProperty) || budgetProperty.$ref !== '#/$defs/AgentBudgetSettings'
    || !fields(budgetProperty.default, ['max_tokens']) || budgetProperty.default.max_tokens !== 1_000_000
    || !record(capSchema) || capSchema.type !== 'integer' || capSchema.minimum !== 0
    || capSchema.maximum !== Number.MAX_SAFE_INTEGER) return invalid();
  const settings = read.project.settings;
  if (!record(settings) || !count(settings.tick_rate) || settings.tick_rate > 4_294_967_295
    || Object.keys(settings).some(key => !Object.hasOwn(settingsSchema.properties as object, key))) return invalid();
  const budget = Object.hasOwn(settings, 'agent_budget') ? settings.agent_budget : budgetProperty.default;
  if (!fields(budget, ['max_tokens']) || !count(budget.max_tokens)) return invalid();
  return { status: 'ready', maxTokens: budget.max_tokens };
}
function budgetScope(read: EngineRead | undefined): string {
  if (!read) return 'unavailable';
  const budget = projectAgentBudget(read);
  return `${read.project.id}:${budget.status}:${budget.status === 'ready' ? budget.maxTokens : ''}`;
}

function validAgentRequest(value: unknown): value is AgentTurnRequest {
  return fields(value, ['prompt', 'model', 'approval']) && boundedText(value.prompt, 16 * 1024)
    && value.prompt.trim().length > 0 && boundedText(value.model) && value.model.trim().length > 0
    && ['auto', 'destructive', 'always'].includes(value.approval as string);
}
/** Mirror native approval depth/node/byte limits before recursively freezing untrusted JSON. */
function validApprovalArguments(value: unknown): boolean {
  const pending: { value: unknown; depth: number }[] = [{ value, depth: 0 }];
  const seen = new Set<object>();
  let visited = 0;
  let jsonBytes = 0;
  while (pending.length) {
    const node = pending.pop()!;
    if (++visited > 4096 || node.depth > 32) return false;
    if (node.value === null || typeof node.value === 'boolean' || typeof node.value === 'string' || typeof node.value === 'number') {
      if (typeof node.value === 'number' && !Number.isFinite(node.value)
        || typeof node.value === 'string' && !boundedText(node.value, 2 * 1024 * 1024)) return false;
      jsonBytes += bytes(JSON.stringify(node.value));
    } else {
      if (typeof node.value !== 'object' || seen.has(node.value)) return false;
      if (!Array.isArray(node.value) && Object.getPrototypeOf(node.value) !== Object.prototype && Object.getPrototypeOf(node.value) !== null) return false;
      seen.add(node.value);
      const entries = Object.entries(node.value);
      if (Array.isArray(node.value) && (entries.length !== node.value.length || entries.some(([key], index) => key !== String(index)))) return false;
      jsonBytes += 2 + Math.max(0, entries.length - 1);
      for (const [key, child] of entries) {
        if (!Array.isArray(node.value)) {
          if (!boundedText(key, 2 * 1024 * 1024)) return false;
          jsonBytes += bytes(JSON.stringify(key)) + 1;
        }
        if (pending.length + visited >= 4096) return false;
        pending.push({ value: child, depth: node.depth + 1 });
      }
    }
    if (jsonBytes > 2 * 1024 * 1024) return false;
  }
  return true;
}
function validAgentReport(value: unknown): boolean {
  return fields(value, ['steps', 'usage', 'transaction_count', 'tool_call_count', 'transaction_ids', 'tool_calls', 'truncated'])
    && count(value.steps) && fields(value.usage, ['input_tokens', 'output_tokens'])
    && count(value.usage.input_tokens) && count(value.usage.output_tokens)
    && count(value.transaction_count) && count(value.tool_call_count) && typeof value.truncated === 'boolean'
    && Array.isArray(value.transaction_ids) && value.transaction_ids.length <= Math.min(128, value.transaction_count)
    && value.transaction_ids.every(id => boundedText(id))
    && Array.isArray(value.tool_calls) && value.tool_calls.length <= Math.min(128, value.tool_call_count)
    && value.tool_calls.every(tool => fields(tool, ['name', 'succeeded', 'selected_camera'])
      && (tool.name === null || boundedText(tool.name)) && typeof tool.succeeded === 'boolean'
      && (tool.selected_camera === null || boundedText(tool.selected_camera)));
}
function validAgentSession(value: unknown): value is AgentSessionSnapshot {
  if (!fields(value, ['version', 'budget', 'accounting_uncertain', 'worker_running', 'turns'])
    || !count(value.version) || !fields(value.budget, ['max_tokens', 'used_tokens'])
    || !count(value.budget.max_tokens) || !count(value.budget.used_tokens)
    || typeof value.accounting_uncertain !== 'boolean' || typeof value.worker_running !== 'boolean'
    || !Array.isArray(value.turns) || value.turns.length > 16) return false;
  let previous = 0;
  for (const [index, turn] of value.turns.entries()) {
    if (!fields(turn, ['id', 'request', 'status', 'text', 'approvals', 'pending', 'report', 'failure', 'charged_tokens', 'accounting_uncertain'])
      || !sequence(turn.id) || turn.id <= previous || !validAgentRequest(turn.request)
      || ![...AGENT_ACTIVE, 'completed', 'cancelled', 'failed'].includes(turn.status as string)
      || !boundedText(turn.text, 64 * 1024) || !count(turn.charged_tokens) || typeof turn.accounting_uncertain !== 'boolean'
      || (turn.failure !== null && !AGENT_FAILURES.includes(turn.failure as string))
      || (turn.report !== null && !validAgentReport(turn.report))
      || !Array.isArray(turn.approvals) || turn.approvals.length > 32) return false;
    previous = turn.id;
    const ids = new Set<number>();
    let approvalBytes = 0;
    for (const approval of turn.approvals) {
      if (!fields(approval, ['id', 'tool', 'bytes', 'decision']) || !sequence(approval.id) || ids.has(approval.id)
        || !boundedText(approval.tool) || !count(approval.bytes) || approval.bytes > 2 * 1024 * 1024
        || !['pending', 'allowed', 'denied', 'cancelled'].includes(approval.decision as string)) return false;
      ids.add(approval.id);
      approvalBytes += approval.bytes;
    }
    if (approvalBytes > 8 * 1024 * 1024) return false;
    if (AGENT_ACTIVE.includes(turn.status as string) && (index !== value.turns.length - 1 || !value.worker_running)) return false;
    if (turn.pending !== null) {
      const pending = turn.pending;
      if (turn.status !== 'awaiting_approval' || !fields(pending, ['turn', 'id', 'tool', 'arguments'])
        || pending.turn !== turn.id || !sequence(pending.id) || !boundedText(pending.tool)
        || !turn.approvals.some(approval => approval.id === pending.id && approval.tool === pending.tool && approval.decision === 'pending')
        || turn.approvals.filter(approval => approval.decision === 'pending').length !== 1
        || !validApprovalArguments(pending.arguments)) return false;
    } else if (turn.status === 'awaiting_approval' || turn.approvals.some(approval => approval.decision === 'pending')) return false;
  }
  return !value.worker_running || value.turns.length > 0;
}
function agentFailure(error: unknown): BridgeError {
  if (fields(error, ['code', 'message']) && typeof error.code === 'string' && /^agent\.[a-z-]+$/.test(error.code)
    && typeof error.message === 'string') return { code: error.code, message: error.message };
  return { code: 'agent.transport', message: 'Could not contact the native agent. Refresh the session to retry.' };
}
const invalidAgentResponse = () => ({ code: 'agent.invalid-response', message: 'The native agent returned an invalid response. Refresh the session to retry.' });
const providerTransportError = (): BridgeError => ({ code: 'provider.transport', message: 'Could not read the saved OpenAI connection. Retry the account operation.' });
/** Native pending states may retain the prior account's safe display metadata. */
function validProviderState(value: unknown): value is ProviderState {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const state = value as Record<string, unknown>;
  const allowed = ['status', 'provider', 'accounts', 'activeAccount', 'message', 'method', 'phase', 'accountLabel', 'error'];
  if (Object.keys(state).some(key => !allowed.includes(key)) || state.provider !== 'openai'
    || !['checking', 'not-connected', 'connecting', 'connected', 'error'].includes(state.status as string)) return false;
  if (state.accounts !== undefined && (!Array.isArray(state.accounts)
    || state.accounts.some(account => !fields(account, ['id', 'label']) || typeof account.id !== 'string' || typeof account.label !== 'string'))) return false;
  if (['activeAccount', 'message', 'accountLabel'].some(key => state[key] !== undefined && typeof state[key] !== 'string')) return false;
  if (state.method !== undefined && !['oauth', 'api-key'].includes(state.method as string)
    || state.phase !== undefined && !['browser', 'validating'].includes(state.phase as string)) return false;
  if (state.error !== undefined && (!fields(state.error, ['code', 'message'])
    || typeof state.error.code !== 'string' || typeof state.error.message !== 'string')) return false;
  if (state.status === 'connected') return state.method !== undefined && typeof state.accountLabel === 'string';
  if (state.status === 'connecting') return state.method !== undefined;
  return state.status !== 'error' || state.error !== undefined;
}
const PLAY_LOG_ROWS = 1024;
const PLAY_LOG_BYTES = 256 * 1024;
const PLAY_LOG_BATCH_ROWS = 64;
const PLAY_LOG_BATCH_BYTES = 16 * 1024;
const playLogError = (): BridgeError => ({ code: 'play.logs', message: 'Native Play logs are unavailable. Refresh Play to retry.' });
const consoleId = (value: unknown): value is string => typeof value === 'string' && /^[0-7][0-9A-HJKMNP-TV-Z]{25}$/.test(value);
function validPlayLogSummary(value: unknown): value is NonNullable<PlayStatus['logs']> {
  return fields(value, ['count', 'dropped']) && count(value.count) && value.count <= PLAY_LOG_ROWS && count(value.dropped);
}
function validPlayLogBatch(value: unknown): value is PlayLogBatch {
  if (!fields(value, ['generation', 'from', 'next', 'dropped', 'warningId', 'entries'])
    || !count(value.generation) || value.generation > 0xffff_ffff || !count(value.from) || !count(value.next)
    || value.next > PLAY_LOG_ROWS || !count(value.dropped) || !consoleId(value.warningId)
    || !Array.isArray(value.entries) || value.entries.length > PLAY_LOG_BATCH_ROWS
    || value.next !== value.from + value.entries.length) return false;
  let total = 0;
  const ids = new Set<string>();
  return value.entries.every(entry => {
    if (!fields(entry, ['id', 'tick', 'level', 'message']) || !consoleId(entry.id) || entry.id === value.warningId
      || ids.has(entry.id) || !count(entry.tick) || !['debug', 'info', 'warn', 'error'].includes(entry.level as string)
      || !boundedText(entry.message, 1024)) return false;
    ids.add(entry.id);
    total += bytes(entry.message);
    return total <= PLAY_LOG_BATCH_BYTES;
  });
}
function samePlayStatus(previous: PlayStatus, next: PlayStatus): boolean {
  const keys = Object.keys(previous) as (keyof PlayStatus)[];
  return keys.length === Object.keys(next).length && keys.every(key => key === 'logs'
    ? previous.logs?.count === next.logs?.count && previous.logs?.dropped === next.logs?.dropped
    : previous[key] === next[key]);
}
export class NativeBridge implements EditorBridge {
  readonly protocolVersion = 1;
  private readonly nativeCapabilities = [
    "entity.rename",
    "entity.delete",
    "asset.import",
    "history.undo",
    "history.redo",
    "provider.connect",
    "provider.cancel",
    "provider.disconnect",
    "provider.switch",
    "agent.send",
    "agent.cancel",
    "agent.approve",
    "viewport.bounds",
    "play.control",
    "play.inspect",
    "play.input",
    "window.drag",
    "window.minimize",
    "window.maximize",
    "window.close",
  ];
  get capabilities(): readonly string[] {
    return [...this.nativeCapabilities,
      ...(this.read && projectAgentBudget(this.read).status === 'ready' ? ['agent.budget.set'] : []),
      ...(this.read?.project_file?.available === true ? ['project.file'] : [])];
  }
  readonly label = "Native engine";
  readonly isFixture = false;
  private read: EngineRead | undefined;
  private play: PlayStatus | undefined;
  private playError: BridgeError | undefined;
  private playRead: Promise<void> | undefined;
  private playUpdateId = 0;
  private playLogScope = 0;
  private playLogGeneration: number | undefined;
  private playLogOwner: string | undefined;
  private playLogProject: string | undefined;
  private playLogIndex = 0;
  private playLogDropped = 0;
  private playLogGenerationBytes = 0;
  private playLogOmitted = 0;
  private playLogWarningId: string | undefined;
  private playLogIds = new Set<string>();
  private playLogRetainedIds = new Set<string>();
  private playLogRows: ConsoleEntry[] = [];
  private playLogBytes = 0;
  private listeners = new Set<() => void>();
  private historyListeners = new Set<(action: 'undo' | 'redo') => void>();
  private pending = false;
  private importing = false;
  private refreshId = 0;
  private chrome: WindowChrome | undefined;
  private provider: ProviderState = { status: "checking", provider: "openai" };
  private providerGeneration = 0;
  private providerReadSequence = 0;
  private providerReadPending: { generation: number; sequence: number } | undefined;
  private agentSession: AgentSessionSnapshot | undefined;
  private agentVerified = false;
  private agentBudgetPending = false;
  private agentError: BridgeError | undefined;
  private agentGeneration = 0;
  private agentRead: { generation: number; promise: Promise<void> } | undefined;
  private agentStarting = false;
  private agentReserved: number | undefined;
  private agentDecisions = new Set<string>();
  private agentCancelling = new Set<number>();
  private providerPending = 0;
  private agentPolling = false;
  private agentTimer: ReturnType<typeof setTimeout> | undefined;
  private snapshot: BridgeSnapshot = snapshotFromEngine({ status: "loading" });
  constructor(private invoke: Invoke) {}
  getSnapshot = () => this.snapshot;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  subscribeHistoryRequests = (listener: (action: 'undo' | 'redo') => void) => {
    this.historyListeners.add(listener);
    return () => { this.historyListeners.delete(listener); };
  };
  async startHistoryRequests(listen?: Listen) {
    if (listen) await listen<unknown>("incant:history-request", ({ payload }) => {
      if (payload !== 'undo' && payload !== 'redo') return;
      this.historyListeners.forEach((listener) => listener(payload));
    });
  }
  private publish(read: EngineResponse) {
    // A render/status event can read while a command is still in flight. Its
    // older document must not replace the command's newer revision on arrival.
    if (isReady(read) && this.read && read.project.id === this.read.project.id && read.revision < this.read.revision) return;
    const next = isReady(read) ? freeze(read) : undefined;
    const scopeChanged = budgetScope(this.read) !== budgetScope(next);
    if (scopeChanged) { ++this.agentGeneration; this.agentVerified = false; }
    if (!next || (this.playLogProject !== undefined && next.project.id !== this.playLogProject)) ++this.playLogScope;
    if (next && next.project.id !== this.playLogProject) {
      // A first project read can follow Play bootstrap. Later project switches cannot
      // attach the old world's log prefix to the new project.
      if (this.playLogProject === undefined) this.playLogOwner = next.project.id;
      this.playLogProject = next.project.id;
      this.playLogRows = [];
      this.playLogRetainedIds.clear();
      this.playLogBytes = 0;
      this.playLogOmitted = 0;
      this.playLogWarningId = undefined;
    }
    this.read = next;
    const snapshot = { ...snapshotFromEngine(read), provider: this.provider, ...this.agentFields(), ...(this.play ? { play: this.play } : {}), ...(this.playError ? { playError: this.playError } : {}) };
    const withConsole = { ...snapshot, console: this.playConsole(snapshot.console) };
    this.snapshot = freeze(
      this.chrome ? { ...withConsole, window: this.chrome } : withConsole,
    );
    this.listeners.forEach((fn) => fn());
    if (scopeChanged && !this.agentBudgetPending && (this.agentSession || this.agentPolling)) void this.refreshAgent();
  }
  updateWindow(chrome: WindowChrome) {
    this.chrome = freeze(chrome);
    this.snapshot = freeze({ ...this.snapshot, window: this.chrome });
    this.listeners.forEach((fn) => fn());
  }
  updateProvider(provider: ProviderState) {
    const pendingRead = this.providerReadPending?.generation === this.providerGeneration;
    ++this.providerGeneration;
    this.providerReadPending = undefined;
    const previous = this.provider;
    // Even an unchanged account must invalidate session reads begun before its authoritative read completed.
    if (pendingRead || previous.status !== provider.status || previous.activeAccount !== provider.activeAccount
      || (previous.status === 'connected' && provider.status === 'connected'
        && (previous.accountLabel !== provider.accountLabel || previous.method !== provider.method))) {
      ++this.agentGeneration;
      this.agentVerified = false;
    }
    this.provider = freeze(provider);
    this.snapshot = freeze({ ...this.snapshot, provider: this.provider });
    this.publishAgent();
    if ((this.agentSession || this.agentPolling) && !this.providerPending) void this.refreshAgent();
  }
  async startProviderUpdates(listen?: Listen) {
    const generation = this.providerGeneration;
    try {
      if (listen) await listen<unknown>("incant:provider-changed", () => {
        // Events are hints, including null/late payloads. Only a current native read is authoritative.
        void this.refreshProvider().catch(() => console.error('Native provider update could not be published.'));
      });
      await this.refreshProvider();
    } catch {
      if (generation === this.providerGeneration) this.updateProvider({ status: 'error', provider: 'openai', error: providerTransportError() });
    }
  }
  private async refreshProvider(generation = this.providerGeneration): Promise<BridgeResult> {
    if (generation !== this.providerGeneration) return { ok: true };
    const pending = { generation, sequence: ++this.providerReadSequence };
    this.providerReadPending = pending;
    ++this.agentGeneration;
    this.agentVerified = false;
    this.publishAgent();
    try {
      const provider = await this.invoke<unknown>('provider_read');
      if (!validProviderState(provider)) throw providerTransportError();
      if (generation === this.providerGeneration && pending.sequence === this.providerReadSequence) this.updateProvider(provider);
      return { ok: true };
    } catch {
      const error = providerTransportError();
      if (generation === this.providerGeneration && pending.sequence === this.providerReadSequence) this.updateProvider({ status: 'error', provider: 'openai', error });
      return { ok: false, error };
    } finally {
      // Only the latest read can release its barrier; obsolete reads may remain in flight.
      if (this.providerReadPending === pending) this.providerReadPending = undefined;
    }
  }
  async startWindowUpdates(listen?: Listen) {
    if (listen)
      await listen<WindowChrome>("incant:window-changed", (event) =>
        this.updateWindow(event.payload),
      );
    this.updateWindow(await this.invoke<WindowChrome>("window_read"));
  }
  async startEngineUpdates(listen?: Listen) {
    // Subscribe before the initial read, so completion during setup cannot be lost.
    if (!listen) {
      this.publish({ status: "error", error: { code: "engine.events", message: "Native project updates are unavailable. Restart Incant to retry." } });
      return;
    }
    try {
      await listen<unknown>("incant:engine-changed", () => { void this.start(); });
    } catch {
      this.publish({ status: "error", error: { code: "engine.events", message: "Could not connect native project updates. Restart Incant to retry." } });
      return;
    }
    await this.start();
  }
  async start(safeErrors = false) {
    const request = ++this.refreshId;
    try {
      const read = await this.invoke<EngineResponse>("engine_read");
      if (request === this.refreshId) this.publish(read);
    } catch (error) {
      if (request !== this.refreshId) return;
      const result = failure(safeErrors && typeof error === 'string' ? undefined : error);
      if (!result.ok) this.publish({ status: "error", error: result.error });
    }
  }
  private agentNeedsPoll() {
    return this.agentStarting || this.agentReserved !== undefined || this.agentSession?.worker_running === true
      || AGENT_ACTIVE.includes(this.agentSession?.turns.at(-1)?.status ?? '');
  }
  private providerIsPending() {
    return this.providerPending > 0 || this.providerReadPending?.generation === this.providerGeneration;
  }
  private agentBudgetReady(): boolean {
    if (this.agentBudgetPending) return false;
    const budget: ProjectAgentBudget = this.read ? projectAgentBudget(this.read) : { status: 'unsupported' };
    return budget.status === 'unsupported' || budget.status === 'ready' && budget.maxTokens === this.agentSession?.budget.max_tokens;
  }
  private agentState(): AgentState {
    if (this.agentNeedsPoll()) return { status: 'running', conversation: String(this.agentReserved ?? this.agentSession?.turns.at(-1)?.id ?? 'starting'), approvalReady: this.agentApprovalReady() };
    if (!this.read) return { status: 'unavailable', reason: 'Wait for the project to load.', code: 'loading' };
    if (this.providerIsPending() || this.provider.status !== 'connected') return { status: 'unavailable', reason: 'Connect an OpenAI account to use the agent.', code: this.providerIsPending() ? 'refreshing' : 'signed-out' };
    if (this.agentError) return { status: 'unavailable', reason: this.agentError.message, code: 'transport' };
    if (!this.agentSession) return { status: 'unavailable', reason: 'The native agent session has not been read yet.', code: 'unread' };
    if (!this.agentVerified || !this.agentBudgetReady()) return { status: 'unavailable', reason: 'The agent session has not caught up with a recent account or project change.', code: 'refreshing' };
    if (this.agentSession.budget.used_tokens >= this.agentSession.budget.max_tokens) return { status: 'unavailable', reason: this.exhaustedReason(), code: 'exhausted' };
    return { status: 'idle' };
  }
  private agentApprovalReady(): boolean {
    return Boolean(this.read && !this.providerIsPending() && this.provider.status === 'connected'
      && !this.agentError && this.agentSession && this.agentVerified && this.agentBudgetReady());
  }
  /** Scoped to where the allowance belongs: the project's authored cap, or an older host's session. */
  private exhaustedReason(): string {
    return this.read && projectAgentBudget(this.read).status === 'ready'
      ? 'The project’s agent token allowance is used up.'
      : 'This agent session’s token allowance is used up.';
  }
  private agentFields() {
    return { agent: this.agentState(), ...(this.agentSession ? { agentSession: this.agentSession } : {}),
      ...(this.agentError ? { agentError: this.agentError } : {}) };
  }
  private publishAgent() {
    const { agentSession: _session, agentError: _error, ...rest } = this.snapshot;
    this.snapshot = freeze({ ...rest, ...this.agentFields() });
    this.listeners.forEach((fn) => fn());
    this.scheduleAgentPoll();
  }
  private scheduleAgentPoll() {
    if (!this.agentPolling || !this.agentNeedsPoll()) {
      if (this.agentTimer !== undefined) clearTimeout(this.agentTimer);
      this.agentTimer = undefined;
      return;
    }
    if (this.agentTimer !== undefined || this.agentRead?.generation === this.agentGeneration) return;
    this.agentTimer = setTimeout(() => { this.agentTimer = undefined; void this.refreshAgent(); }, 250);
  }
  /** Bootstrap/recovery is explicit; normal polling reads at most one bounded session at a time. */
  async refreshAgent() {
    const generation = this.agentGeneration;
    if (this.agentRead?.generation === generation) return this.agentRead.promise;
    const pending = { generation, promise: this.readAgent(generation) };
    this.agentRead = pending;
    try { await pending.promise; } finally {
      if (this.agentRead === pending) this.agentRead = undefined;
      this.scheduleAgentPoll();
    }
  }
  private async readAgent(generation: number) {
    const version = this.agentSession?.version;
    try {
      const response = await this.invoke<unknown>('engine_agent_read', version === undefined ? {} : { afterVersion: version });
      if (generation !== this.agentGeneration) return;
      if (!fields(response, ['session', 'project_revision']) || (response.project_revision !== null && !count(response.project_revision))) throw invalidAgentResponse();
      const session = response.session;
      let updated = false;
      if (session === null) {
        if (version === undefined || version !== this.agentSession?.version) throw invalidAgentResponse();
      } else {
        if (!validAgentSession(session)) throw invalidAgentResponse();
        if (this.agentSession && session.version < this.agentSession.version) return;
        if (!this.agentSession || session.version > this.agentSession.version) { this.agentSession = freeze(session); updated = true; }
        const pending = this.agentSession!.turns.at(-1)?.pending;
        for (const key of this.agentDecisions) if (key !== `${pending?.turn}:${pending?.id}`) this.agentDecisions.delete(key);
      }
      if (this.agentReserved !== undefined && this.agentSession!.turns.at(-1)?.id !== undefined
        && this.agentSession!.turns.at(-1)!.id >= this.agentReserved) this.agentReserved = undefined;
      const projectChanged = response.project_revision !== null && this.read !== undefined && response.project_revision > this.read.revision;
      // A new revision may carry a cap edit. Do not reopen admission against a cached document.
      const verified = !this.providerIsPending() && this.agentBudgetReady() && !projectChanged;
      const changed = this.agentError !== undefined || updated || this.agentVerified !== verified;
      this.agentVerified = verified;
      this.agentError = undefined;
      if (changed) this.publishAgent();
      // Text chunks and unchanged versions never clone/re-read the project.
      // A newer project read already covers an older revision observed by this poll.
      if (projectChanged) {
        await this.start(true);
        if (generation !== this.agentGeneration) return;
        const recovered = !this.providerIsPending() && this.agentBudgetReady();
        if (this.agentVerified !== recovered) { this.agentVerified = recovered; this.publishAgent(); }
      }
    } catch (error) {
      if (generation !== this.agentGeneration) return;
      this.agentVerified = false;
      this.agentError = agentFailure(error);
      this.publishAgent();
    }
  }
  startAgentUpdates(): () => void {
    this.agentPolling = true;
    void this.refreshAgent();
    return () => {
      this.agentPolling = false;
      if (this.agentTimer !== undefined) clearTimeout(this.agentTimer);
      this.agentTimer = undefined;
    };
  }
  private async requestAgent(request: AgentRequest): Promise<BridgeResult> {
    const reject = (code: string, message: string): BridgeResult => ({ ok: false, error: { code, message } });
    const current = this.agentSession?.turns.at(-1);
    if (request.type === 'agent.send') {
      if (this.agentNeedsPoll()) return reject('agent.busy', 'Wait for the current agent worker to stop.');
      if (!this.read || !this.agentSession || !this.agentVerified || !this.agentBudgetReady() || this.agentError || this.providerIsPending() || this.provider.status !== 'connected') {
        const state = this.agentState();
        return reject('agent.unavailable', state.status === 'unavailable' ? state.reason : 'The native agent is unavailable.');
      }
      const turnRequest = { prompt: request.text, model: request.model ?? 'gpt-6-astra', approval: request.approval ?? 'destructive' };
      if (!validAgentRequest(turnRequest)) return reject('agent.invalid-request', 'Enter a nonempty prompt up to 16 KiB and a model name up to 128 bytes.');
      if (this.agentSession.budget.used_tokens >= this.agentSession.budget.max_tokens) return reject('agent.budget', this.exhaustedReason());
      this.agentStarting = true;
      const generation = ++this.agentGeneration;
      this.publishAgent();
      try {
        const turnId = await this.invoke<unknown>('engine_agent_start', { request: turnRequest });
        if (!sequence(turnId) || (current && turnId <= current.id)) throw invalidAgentResponse();
        this.agentReserved = turnId;
        await this.refreshAgent();
        return { ok: true, agentTurnId: turnId };
      } catch (error) {
        if (generation === this.agentGeneration) {
          await this.refreshAgent();
          if (generation === this.agentGeneration) this.agentError = agentFailure(error);
        }
        return { ok: false, error: agentFailure(error) };
      } finally { this.agentStarting = false; this.publishAgent(); }
    }
    if (!sequence(request.turnId) || (!current || current.id !== request.turnId || !AGENT_ACTIVE.includes(current.status))
      && request.turnId !== this.agentReserved) return reject('agent.stale-turn', 'This agent turn is no longer active.');
    if (request.type === 'agent.cancel') {
      if (current?.status === 'stopping' || this.agentCancelling.has(request.turnId)) return reject('agent.busy', 'This agent turn is already stopping.');
      this.agentCancelling.add(request.turnId);
    } else {
      if (!this.agentApprovalReady())
        return reject('agent.unavailable', 'Connect an OpenAI account and refresh the native session before deciding an approval.');
      if (!sequence(request.approvalId) || typeof request.approved !== 'boolean' || current?.status !== 'awaiting_approval'
        || current.pending?.turn !== request.turnId || current.pending.id !== request.approvalId
        || this.agentCancelling.has(request.turnId)
        || this.agentDecisions.has(`${request.turnId}:${request.approvalId}`)) return reject('agent.stale-approval', 'This agent approval is no longer pending.');
      this.agentDecisions.add(`${request.turnId}:${request.approvalId}`);
    }
    const generation = ++this.agentGeneration;
    try {
      const result = request.type === 'agent.cancel'
        ? await this.invoke<unknown>('engine_agent_cancel', { turnId: request.turnId })
        : await this.invoke<unknown>('engine_agent_decide', { turnId: request.turnId, approvalId: request.approvalId, approved: request.approved });
      if (result !== null && result !== undefined) throw invalidAgentResponse();
      await this.refreshAgent();
      return { ok: true };
    } catch (error) {
      if (generation === this.agentGeneration) {
        await this.refreshAgent();
        if (generation === this.agentGeneration) {
          this.agentError = agentFailure(error);
          this.publishAgent();
        }
      }
      if (request.type === 'agent.approve') this.agentDecisions.delete(`${request.turnId}:${request.approvalId}`);
      return { ok: false, error: agentFailure(error) };
    } finally { this.agentCancelling.delete(request.turnId); }
  }
  updatePlay(status: PlayStatus) {
    if (this.play && (status.generation < this.play.generation || status.sequence < this.play.sequence)) return false;
    if (status.logs !== undefined && !validPlayLogSummary(status.logs)) {
      this.failPlayLogs();
      return false;
    }
    if (status.generation !== this.playLogGeneration) {
      ++this.playLogScope;
      this.playLogGeneration = status.generation;
      this.playLogOwner = this.read?.project.id;
      this.playLogIndex = 0;
      this.playLogDropped = 0;
      this.playLogGenerationBytes = 0;
      this.playLogIds.clear();
    } else if (status.phase === 'error' && this.play?.phase !== 'error') ++this.playLogScope;
    // An accepted duplicate still fences failures from earlier reads without replacing the snapshot.
    ++this.playUpdateId;
    const retained = this.play && samePlayStatus(this.play, status) ? this.play : undefined;
    if (retained && !this.playError) return true;
    this.play = retained ?? freeze(status);
    this.playError = undefined;
    const { playError: _error, ...rest } = this.snapshot;
    this.snapshot = freeze({ ...rest, play: this.play });
    this.listeners.forEach((fn) => fn());
    return true;
  }
  async refreshPlay() {
    if (this.playRead) return this.playRead;
    this.playRead = this.readPlay();
    try { await this.playRead; } finally { this.playRead = undefined; }
  }
  private async readPlay() {
    const published = this.playUpdateId;
    const scope = this.playLogScope;
    const project = this.read?.project.id;
    try {
      const status = await this.invoke<PlayStatus>('play_read');
      if (scope !== this.playLogScope || project !== this.read?.project.id) return;
      if (this.updatePlay(status)) await this.readPlayLogs();
    }
    catch (error) {
      if (this.playUpdateId !== published || scope !== this.playLogScope || project !== this.read?.project.id) return;
      const result = failure(error);
      if (!result.ok) {
        this.playError = result.error;
        this.snapshot = freeze({ ...this.snapshot, playError: result.error });
        this.listeners.forEach((fn) => fn());
      }
    }
  }
  private playConsole(native: readonly ConsoleEntry[]): readonly ConsoleEntry[] {
    if (!this.read) return native;
    const warning: ConsoleEntry[] = this.playLogWarningId && this.playLogOmitted > 0 ? [{
      id: this.playLogWarningId, level: 'warn', source: 'Play', at: timestamp(this.playLogWarningId),
      message: `Native Play Console omitted ${this.playLogOmitted.toLocaleString('en-US')} messages because its bounded history is full.`,
    }] : [];
    // Only rebuild Console when a project read or a new bounded delta arrives.
    return [...native, ...this.playLogRows].sort((a, b) => a.at.localeCompare(b.at)).concat(warning);
  }
  private failPlayLogs() {
    // A newer invalid status must fence any earlier tail still in flight.
    ++this.playLogScope;
    const error = playLogError();
    this.playError = error;
    this.snapshot = freeze({ ...this.snapshot, playError: error });
    this.listeners.forEach(fn => fn());
  }
  private async readPlayLogs() {
    const status = this.play;
    if (!status?.logs || !this.read || this.read.project.id !== this.playLogOwner
      || !['running', 'paused'].includes(status.phase)) return;
    if (status.logs.count <= this.playLogIndex && status.logs.dropped <= this.playLogDropped) return;
    const scope = this.playLogScope;
    const project = this.read.project.id;
    const from = this.playLogIndex;
    const current = () => scope === this.playLogScope && project === this.read?.project.id
      && status.generation === this.play?.generation && ['running', 'paused'].includes(this.play.phase);
    try {
      const batch = await this.invoke<unknown>('play_logs_read', { generation: status.generation, afterIndex: from });
      if (!current()) return;
      if (!validPlayLogBatch(batch) || batch.generation !== status.generation || batch.from !== from
        || batch.next < Math.min(status.logs.count, from + 1) || batch.dropped < this.playLogDropped
        || this.playLogGenerationBytes + batch.entries.reduce((sum, entry) => sum + bytes(entry.message), 0) > PLAY_LOG_BYTES
        || batch.entries.some(entry => this.playLogIds.has(entry.id) || this.playLogRetainedIds.has(entry.id))
        || (this.playLogWarningId !== undefined && batch.warningId !== this.playLogWarningId)) {
        this.failPlayLogs();
        return;
      }
      // Validate the whole batch before any Console publication or cursor movement.
      this.playLogWarningId = batch.warningId;
      this.playLogOmitted = Math.min(Number.MAX_SAFE_INTEGER, this.playLogOmitted + batch.dropped - this.playLogDropped);
      this.playLogDropped = batch.dropped;
      for (const entry of batch.entries) {
        this.playLogIds.add(entry.id);
        this.playLogRetainedIds.add(entry.id);
        this.playLogRows.push({ id: entry.id, level: entry.level, source: 'Play', message: entry.message, at: timestamp(entry.id) });
        this.playLogBytes += bytes(entry.message);
        this.playLogGenerationBytes += bytes(entry.message);
      }
      this.playLogIndex = batch.next;
      // Reserve one warning row and its bounded message, across every restart.
      while (this.playLogRows.length > PLAY_LOG_ROWS - 1 || this.playLogBytes > PLAY_LOG_BYTES - 256) {
        const removed = this.playLogRows.shift()!;
        this.playLogBytes -= bytes(removed.message);
        this.playLogRetainedIds.delete(removed.id);
        this.playLogOmitted = Math.min(Number.MAX_SAFE_INTEGER, this.playLogOmitted + 1);
      }
      const { playError: _error, ...rest } = this.snapshot;
      this.playError = undefined;
      this.snapshot = freeze({ ...rest, console: this.playConsole(this.read.console.map(event => ({ ...event, source: 'native', at: timestamp(event.id) }))) });
      this.listeners.forEach(fn => fn());
    } catch {
      if (current()) this.failPlayLogs();
    }
  }
  /** One metadata read in flight, at most four per second. No engine_read polling. */
  startPlayUpdates(): () => void {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      await this.refreshPlay();
      if (!stopped) timer = setTimeout(() => { void poll(); }, 250);
    };
    void poll();
    return () => { stopped = true; if (timer !== undefined) clearTimeout(timer); };
  }
  private async requestPlay(request: PlayRequest): Promise<BridgeResult> {
    const intendedGeneration = this.play?.generation;
    if (request.type === 'play.input' && (!this.play || this.playError || request.generation !== this.play.generation)) return failure('Game input session changed or is unavailable.');
    if (!this.play || this.playError) {
      await this.refreshPlay();
      if (!this.play || this.playError) return failure(this.playError ?? 'Native Play is unavailable.');
    }
    const generation = this.play.generation;
    if (intendedGeneration !== undefined && intendedGeneration !== generation) return failure('Play session changed; retry the operation.');
    if (request.type === 'play.start' && !this.read) return failure('Wait for the project to load.');
    const availability = this.read && playStartAvailability(this.read);
    if (request.type === 'play.start' && availability?.available === false) return failure(availability.reason ?? 'Native Play cannot start for this project.');
    if (request.type === 'play.input' && request.events.length > 128) return failure('Input packet exceeds 128 events.');
    const command = {
      op: request.type.slice('play.'.length), generation,
      ...(request.type === 'play.start' ? { expected_revision: this.read!.revision,
        options: { ...(request.scene ? { scene: request.scene } : {}), ...(request.camera ? { camera: request.camera } : {}) } } : {}),
      ...(request.type === 'play.inspect' ? { entity: request.entity } : {}),
      ...(request.type === 'play.input' ? { events: request.events } : {}),
    };
    try {
      if (request.type === 'play.inspect') {
        const entity = await this.invoke<PlayEntity>('play_command', { command });
        if (this.play.generation !== generation || entity.generation !== generation) return failure('Play session changed; inspect again.');
        return { ok: true, playEntity: freeze(entity) };
      }
      const status = await this.invoke<PlayStatus>('play_command', { command });
      // Input is a bounded write to the disposable world; polling publishes its
      // status at a fixed rate instead of making the whole UI render per event.
      if (request.type !== 'play.input') this.updatePlay(status);
      return { ok: true };
    } catch (error) {
      if (request.type !== 'play.input') await this.refreshPlay();
      return failure(error);
    }
  }
  async dispatch(command: EditorCommand): Promise<BridgeResult> {
    const budgetEdit = command.type === 'agent.budget.set';
    if (budgetEdit && !count(command.maxTokens)) return { ok: false, error: { code: 'schema.invalid', message: 'Enter a whole number of tokens from 0 to 9,007,199,254,740,991.' } };
    if (budgetEdit && !this.capabilities.includes('agent.budget.set')) return { ok: false, error: { code: 'unsupported', message: 'This host cannot change the project’s agent token allowance.' } };
    const importing = command.type === 'asset.import';
    if (!this.read || this.pending || (importing && this.importing))
      return {
        ok: false,
        error: {
          code: "engine.busy",
          message: "Wait for the current engine operation.",
        },
      };
    // Cooking owns a snapshot, so normal edits/history may continue. A changed
    // revision rejects the import at commit rather than locking the user out.
    const recoverBudget = budgetEdit || (command.type === 'history.undo' || command.type === 'history.redo')
      && projectAgentBudget(this.read).status !== 'unsupported';
    const previous = this.read;
    // Subscribers run synchronously. Reserve the mutation before publishing its
    // admission barrier so a subscriber cannot start a second document write.
    if (importing) this.importing = true;
    else this.pending = true;
    if (recoverBudget) {
      this.agentBudgetPending = true;
      ++this.agentGeneration;
      this.agentVerified = false;
      this.publishAgent();
    }
    try {
      // A pending background read cannot overwrite the mutation response.
      ++this.refreshId;
      let read: EngineRead;
      let result: BridgeResult = { ok: true };
      if (command.type === 'asset.import') {
        if (!this.read.asset_import?.available) return failure(this.read.asset_import?.reason ?? 'Open a saved project to import assets.');
        const imported = await this.invoke<EngineImportResponse>('engine_import', {
          requests: command.sources.map((source) => ({ source: source.source, texture_usage: source.textureUsage ?? null })),
          expectedRevision: this.read.revision,
        });
        read = imported.snapshot;
        result = { ok: true, assetImport: {
          revision: imported.commit.revision,
          transaction: imported.commit.transaction === null ? null : id(imported.commit.transaction),
        } };
      } else if (command.type === 'agent.budget.set') {
        read = await this.invoke<EngineRead>('engine_execute', {
          commands: [{ op: 'set_agent_budget', settings: { max_tokens: command.maxTokens } }],
          description: 'Set agent token allowance',
          expectedRevision: previous.revision,
        });
      } else if (command.type === "history.undo" || command.type === "history.redo") {
        read = await this.invoke<EngineRead>("engine_history", {
          redo: command.type === "history.redo",
        });
      } else {
        const scene = Object.values(this.read.project.scenes).find(
          (scene) => command.entity in scene.entities,
        );
        if (!scene)
          return {
            ok: false,
            error: {
              code: "entity.not-found",
              message:
                "This operation requires an entity. Scene edits are not exposed in this spike.",
            },
          };
        const op =
          command.type === "entity.rename"
            ? {
                op: "rename_entity",
                scene_id: scene.id,
                entity_id: command.entity,
                name: command.name,
              }
            : {
                op: "delete_entity",
                scene_id: scene.id,
                entity_id: command.entity,
              };
        read = await this.invoke<EngineRead>("engine_execute", {
          commands: [op],
          description:
            command.type === "entity.rename"
              ? `Rename entity to ${command.name}`
              : "Delete entity",
          expectedRevision: this.read.revision,
        });
      }
      if (recoverBudget) {
        if (!isReady(read) || !count(read.revision) || !record(read.project) || projectAgentBudget(read).status !== 'ready') throw budgetInvalidResponse();
        const cap = projectAgentBudget(read);
        if (budgetEdit && cap.status === 'ready' && cap.maxTokens !== command.maxTokens
          || read.project.id !== previous.project.id || read.revision < previous.revision
          || this.read && read.project.id === this.read.project.id && read.revision < this.read.revision) {
          throw { code: 'engine.stale', message: 'The project changed while saving the agent token allowance. Try again.' };
        }
      }
      this.publish(read);
      return result;
    } catch (error) {
      await this.start(recoverBudget);
      return failure(recoverBudget && typeof error === 'string' ? undefined : error);
    } finally {
      if (importing) this.importing = false;
      else this.pending = false;
      if (recoverBudget) {
        this.agentBudgetPending = false;
        ++this.agentGeneration;
        this.agentVerified = false;
        this.publishAgent();
        await this.refreshAgent();
      }
    }
  }
  private async requestProjectFile(request: ProjectFileRequest): Promise<BridgeResult> {
    if (!this.read || this.pending) return failure('Wait for the current engine operation.');
    if (this.read.project_file?.available !== true) return failure('Open a saved project to use file operations.');
    const keys = request.type === 'project.pin.apply' ? ['type', 'reviewId'] : ['type'];
    if (Object.keys(request).some((key) => !keys.includes(key))
      || (request.type === 'project.pin.apply' && !fileId(request.reviewId))) return failure('Invalid project file request.');
    const previous = this.read;
    const operation = request.type === 'project.pin.review' ? 'review_pin' : request.type === 'project.pin.apply' ? 'apply_pin' : 'save';
    this.pending = true;
    ++this.refreshId;
    try {
      const result = await this.invoke<{ response: unknown; snapshot: EngineResponse }>('engine_project_file', {
        request: { operation, expected_revision: previous.revision,
          ...(request.type === 'project.pin.apply' ? { review_id: request.reviewId } : {}) },
      });
      const response = projectFileResponse(result.response, operation, previous.revision);
      const read = result.snapshot;
      if (!response || !isReady(read) || !count(read.revision) || read.project.id !== previous.project.id
        || read.revision < previous.revision + (operation === 'apply_pin' ? 1 : 0)) throw 'Invalid project file response.';
      this.publish(read);
      return { ok: true, projectFile: freeze(response) };
    } catch (error) {
      await this.start();
      return failure(error);
    } finally { this.pending = false; }
  }
  async request(request: HostRequest): Promise<BridgeResult> {
    if (request.type === 'project.pin.review' || request.type === 'project.pin.apply' || request.type === 'project.save') return this.requestProjectFile(request);
    if (request.type === 'agent.send' || request.type === 'agent.cancel' || request.type === 'agent.approve') return this.requestAgent(request);
    if (request.type === 'play.start' || request.type === 'play.pause' || request.type === 'play.resume' || request.type === 'play.step' || request.type === 'play.stop' || request.type === 'play.inspect' || request.type === 'play.input') return this.requestPlay(request);
    if (request.type === "provider.connect" || request.type === "provider.cancel" || request.type === "provider.disconnect" || request.type === "provider.switch") {
      if (request.type === "provider.connect" && request.method !== "oauth") return failure("Use incant auth api-key for the hidden API-key prompt.");
      ++this.providerPending;
      const providerGeneration = ++this.providerGeneration;
      ++this.agentGeneration;
      this.agentVerified = false;
      this.publishAgent();
      try {
        await this.invoke("provider_action", { action: request.type.slice("provider.".length), accountId: "accountId" in request ? request.accountId ?? null : null, add: "add" in request ? request.add ?? false : false });
        return await this.refreshProvider(providerGeneration);
      } catch (error) {
        // A failed scheduling receipt may follow a native state transition.
        // Re-read for recovery without replacing the caller's original failure.
        await this.refreshProvider(providerGeneration);
        return failure(error);
      }
      finally {
        --this.providerPending;
        this.publishAgent();
        if ((this.agentSession || this.agentPolling) && !this.providerPending) void this.refreshAgent();
      }
    }
    if (
      request.type === "window.drag" ||
      request.type === "window.minimize" ||
      request.type === "window.maximize" ||
      request.type === "window.close"
    ) {
      try {
        await this.invoke("window_action", {
          action: request.type.slice("window.".length),
        });
        return { ok: true };
      } catch (error) {
        return failure(error);
      }
    }
    if (request.type !== "viewport.bounds")
      return {
        ok: false,
        error: {
          code: "unsupported",
          message: "This host capability is not connected.",
        },
      };
    const { rect, devicePixelRatio: dpr, cornerRadii } = request;
    if (!Number.isFinite(dpr) || dpr <= 0)
      return failure("Invalid device pixel ratio.");
    try {
      await this.invoke("viewport_bounds", {
        rect: [rect.x, rect.y, rect.width, rect.height].map((v) => v * dpr),
        cornerRadii: cornerRadii.map((v) => v * dpr),
      });
      return { ok: true };
    } catch (error) {
      return failure(error);
    }
  }
}
export function installNativeBridge(): NativeBridge | undefined {
  const host = window as Window & {
    __TAURI__?: { core?: { invoke?: Invoke }; event?: { listen?: Listen } };
  };
  const invoke = host.__TAURI__?.core?.invoke;
  if (!invoke || host.__INCANT_BRIDGE__) return undefined;
  const bridge = new NativeBridge(invoke);
  host.__INCANT_BRIDGE__ = bridge;
  const stopPlayUpdates = bridge.startPlayUpdates();
  const stopAgentUpdates = bridge.startAgentUpdates();
  window.addEventListener('pagehide', stopPlayUpdates, { once: true });
  window.addEventListener('pagehide', stopAgentUpdates, { once: true });
  void bridge.startEngineUpdates(host.__TAURI__?.event?.listen);
  void bridge.startHistoryRequests(host.__TAURI__?.event?.listen)
    .catch(() => console.error("Native history menu could not be connected."));
  void bridge.startProviderUpdates(host.__TAURI__?.event?.listen).catch(() => console.error('Native provider updates could not be published.'));
  void bridge
    .startWindowUpdates(host.__TAURI__?.event?.listen)
    .catch(() => console.error("Native window state could not be connected."));
  return bridge;
}
