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
}
export type EngineResponse = EngineRead
  | { status: "loading" }
  | { status: "error"; error: BridgeError };
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
    ...(read.project.assets ? { assets: { status: 'ready' as const, value: Object.values(read.project.assets).map((asset) => ({
      id: id(asset.id), name: asset.name, path: asset.path, kind: asset.kind, fingerprint: asset.sha256,
      ...(asset.import_settings?.type === 'texture' ? { textureUsage: asset.import_settings.usage } : {}),
    })) } } : {}),
    assetImport: read.asset_import ?? { available: false, reason: 'This host does not expose asset importing.' },
    diagnostics: [
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
      reason: "Use the headless agent command for the Phase 0 provider spike.",
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
export class NativeBridge implements EditorBridge {
  readonly protocolVersion = 1;
  readonly capabilities = [
    "entity.rename",
    "entity.delete",
    "asset.import",
    "history.undo",
    "history.redo",
    "provider.connect",
    "provider.cancel",
    "provider.disconnect",
    "provider.switch",
    "viewport.bounds",
    "window.drag",
    "window.minimize",
    "window.maximize",
    "window.close",
  ];
  readonly label = "Native engine";
  readonly isFixture = false;
  private read: EngineRead | undefined;
  private listeners = new Set<() => void>();
  private historyListeners = new Set<(action: 'undo' | 'redo') => void>();
  private pending = false;
  private importing = false;
  private refreshId = 0;
  private chrome: WindowChrome | undefined;
  private provider: ProviderState = { status: "checking", provider: "openai" };
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
    this.read = isReady(read) ? freeze(read) : undefined;
    const snapshot = { ...snapshotFromEngine(read), provider: this.provider };
    this.snapshot = freeze(
      this.chrome ? { ...snapshot, window: this.chrome } : snapshot,
    );
    this.listeners.forEach((fn) => fn());
  }
  updateWindow(chrome: WindowChrome) {
    this.chrome = freeze(chrome);
    this.snapshot = freeze({ ...this.snapshot, window: this.chrome });
    this.listeners.forEach((fn) => fn());
  }
  updateProvider(provider: ProviderState) {
    this.provider = freeze(provider);
    this.snapshot = freeze({ ...this.snapshot, provider: this.provider });
    this.listeners.forEach((fn) => fn());
  }
  async startProviderUpdates(listen?: Listen) {
    if (listen) await listen<ProviderState>("incant:provider-changed", (event) => this.updateProvider(event.payload));
    this.updateProvider(await this.invoke<ProviderState>("provider_read"));
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
  async start() {
    const request = ++this.refreshId;
    try {
      const read = await this.invoke<EngineResponse>("engine_read");
      if (request === this.refreshId) this.publish(read);
    } catch (error) {
      if (request !== this.refreshId) return;
      const result = failure(error);
      if (!result.ok) this.publish({ status: "error", error: result.error });
    }
  }
  async dispatch(command: EditorCommand): Promise<BridgeResult> {
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
    if (importing) this.importing = true;
    else this.pending = true;
    try {
      // A pending background read cannot overwrite the mutation response.
      ++this.refreshId;
      let read: EngineRead;
      if (command.type === 'asset.import') {
        if (!this.read.asset_import?.available) return failure(this.read.asset_import?.reason ?? 'Open a saved project to import assets.');
        read = await this.invoke<EngineRead>('engine_import', {
          requests: command.sources.map((source) => ({ source: source.source, texture_usage: source.textureUsage ?? null })),
          expectedRevision: this.read.revision,
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
      this.publish(read);
      return { ok: true };
    } catch (error) {
      await this.start();
      return failure(error);
    } finally {
      if (importing) this.importing = false;
      else this.pending = false;
    }
  }
  async request(request: HostRequest): Promise<BridgeResult> {
    if (request.type === "provider.connect" || request.type === "provider.cancel" || request.type === "provider.disconnect" || request.type === "provider.switch") {
      if (request.type === "provider.connect" && request.method !== "oauth") return failure("Use incant auth api-key for the hidden API-key prompt.");
      try {
        await this.invoke("provider_action", { action: request.type.slice("provider.".length), accountId: "accountId" in request ? request.accountId ?? null : null, add: "add" in request ? request.add ?? false : false });
        this.updateProvider(await this.invoke<ProviderState>("provider_read"));
        return { ok: true };
      } catch (error) { return failure(error); }
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
  void bridge.startEngineUpdates(host.__TAURI__?.event?.listen);
  void bridge.startHistoryRequests(host.__TAURI__?.event?.listen)
    .catch(() => console.error("Native history menu could not be connected."));
  void bridge.startProviderUpdates(host.__TAURI__?.event?.listen).catch(() => bridge.updateProvider({ status: "error", provider: "openai", error: { code: "provider.transport", message: "Could not read the saved OpenAI connection. Restart Incant to retry." } }));
  void bridge
    .startWindowUpdates(host.__TAURI__?.event?.listen)
    .catch(() => console.error("Native window state could not be connected."));
  return bridge;
}
