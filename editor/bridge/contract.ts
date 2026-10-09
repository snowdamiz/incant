/** Shared UI/native transport contract. Engine state and edits remain owned by incant_cmd. */

export const UI_PROTOCOL_VERSION = 1 as const;

/** A ULID issued by the engine. Names are labels; ids are the only keys. */
export type Ulid = string & { readonly __brand: 'Ulid' };

/** Every capability the UI knows how to use. Unknown strings are ignored and reported. */
export const KNOWN_CAPABILITIES = [
  'entity.rename',
  'entity.delete',
  'history.undo',
  'history.redo',
  'provider.connect',
  'agent.send',
  'viewport.bounds',
  'window.drag',
  'window.minimize',
  'window.maximize',
  'window.close',
] as const;
export type Capability = (typeof KNOWN_CAPABILITIES)[number];

export interface BridgeError {
  /** Stable machine code, e.g. `schema.invalid`, `io.unreadable`, `unsupported`. */
  readonly code: string;
  /** Human-readable message. Must never contain credential values. */
  readonly message: string;
}

export type LoadState<T> =
  | { readonly status: 'loading' }
  | { readonly status: 'ready'; readonly value: T }
  | { readonly status: 'error'; readonly error: BridgeError };

export interface ProjectInfo {
  readonly id: Ulid;
  readonly name: string;
}

export type ConnectionState =
  | { readonly status: 'connecting' }
  | { readonly status: 'ready'; readonly project: ProjectInfo }
  | { readonly status: 'error'; readonly error: BridgeError };

/** Entity kinds the UI has icons for. Anything else renders with a generic icon. */
export type EntityKind =
  | 'scene'
  | 'group'
  | 'camera'
  | 'light'
  | 'mesh'
  | 'prefab'
  | 'script'
  | 'audio'
  | (string & {});

export interface HierarchyNode {
  readonly id: Ulid;
  readonly name: string;
  readonly kind: EntityKind;
  readonly parent: Ulid | null;
  readonly children: readonly Ulid[];
}

export interface HierarchyTree {
  readonly roots: readonly Ulid[];
  readonly nodes: Readonly<Record<string, HierarchyNode>>;
}

export type Severity = 'error' | 'warning' | 'info';

/** A validation result from the engine. `path` is a JSON pointer into the component. */
export interface Diagnostic {
  readonly id: string;
  readonly severity: Severity;
  readonly message: string;
  readonly entity: Ulid | null;
  readonly component: string | null;
  readonly path: string | null;
}

/**
 * The subset of JSON Schema the inspector can present. The engine's schema
 * registry is the source of truth; anything outside this subset is shown as an
 * explicit "unsupported field" row, never silently dropped.
 */
export type FieldSchema =
  | {
      readonly type: 'string';
      readonly title?: string;
      readonly description?: string;
      readonly enum?: readonly string[];
      readonly format?: 'ulid' | 'asset-ref' | 'color' | (string & {});
    }
  | {
      readonly type: 'number' | 'integer';
      readonly title?: string;
      readonly description?: string;
      readonly minimum?: number;
      readonly maximum?: number;
      readonly 'x-incant-unit'?: string;
    }
  | { readonly type: 'boolean'; readonly title?: string; readonly description?: string }
  | {
      readonly type: 'array';
      readonly title?: string;
      readonly description?: string;
      readonly items: FieldSchema;
      readonly minItems?: number;
      readonly maxItems?: number;
      readonly 'x-incant-widget'?: 'vec2' | 'vec3' | 'vec4' | 'quat' | (string & {});
    }
  | {
      readonly type: 'object';
      readonly title?: string;
      readonly description?: string;
      readonly properties: Readonly<Record<string, FieldSchema>>;
    }
  | { readonly type: string & {}; readonly title?: string; readonly description?: string };

export interface ComponentSchema {
  readonly type: string;
  readonly version: number;
  readonly title: string;
  readonly description?: string;
  readonly properties: Readonly<Record<string, FieldSchema>>;
  /** Property display order. Properties missing from this list follow alphabetically. */
  readonly order?: readonly string[];
}

export interface ComponentValue {
  readonly type: string;
  readonly schemaVersion: number;
  readonly value: Readonly<Record<string, unknown>>;
}

export interface EntityDetail {
  readonly id: Ulid;
  readonly name: string;
  readonly kind: EntityKind;
  readonly components: readonly ComponentValue[];
}

export type Origin =
  | { readonly kind: 'user' }
  | { readonly kind: 'agent'; readonly model: string; readonly conversation: string }
  | { readonly kind: 'script'; readonly script: string }
  | { readonly kind: 'import'; readonly source: string };

export interface HistoryEntry {
  readonly transaction: Ulid;
  readonly description: string;
  readonly origin: Origin;
  /** ISO 8601 timestamp. */
  readonly at: string;
}

export interface HistoryState {
  readonly entries: readonly HistoryEntry[];
  /** Number of entries currently applied; entries at or after this index are redoable. */
  readonly applied: number;
}

export type ConsoleLevel = 'error' | 'warn' | 'info' | 'debug';

export interface ConsoleEntry {
  readonly id: string;
  readonly level: ConsoleLevel;
  readonly source: string;
  readonly message: string;
  readonly at: string;
}

/** Provider connection metadata only. There is deliberately no field for a key or token. */
export type ProviderState =
  | { readonly status: 'not-connected'; readonly provider: 'openai' }
  | { readonly status: 'connecting'; readonly provider: 'openai'; readonly method: 'oauth' | 'api-key' }
  | {
      readonly status: 'connected';
      readonly provider: 'openai';
      readonly method: 'oauth' | 'api-key';
      readonly accountLabel: string;
    }
  | { readonly status: 'error'; readonly provider: 'openai'; readonly error: BridgeError };

export type AgentState =
  | { readonly status: 'unavailable'; readonly reason: string }
  | { readonly status: 'idle' }
  | { readonly status: 'running'; readonly conversation: string };

export type ViewportState =
  | { readonly status: 'not-attached'; readonly reason: string }
  | { readonly status: 'attached'; readonly surface: string }
  | { readonly status: 'error'; readonly error: BridgeError };

/**
 * Host window chrome. Absent when the UI runs outside the native host (plain
 * browser), in which case the titlebar reserves no insets and shows no window
 * controls. See editor/ui/TITLEBAR.md for platform behavior.
 */
export interface WindowChrome {
  readonly platform: 'macos' | 'windows' | 'linux';
  /**
   * `native-overlay`: the OS draws the window buttons over the webview (macOS
   * traffic lights with an overlay titlebar); the UI only reserves an inset.
   * `custom`: the UI draws minimize/maximize/close and sends window.* requests.
   */
  readonly controls: 'native-overlay' | 'custom';
  readonly maximized: boolean;
  readonly fullscreen: boolean;
  /** False when another window has focus; the titlebar dims like native chrome. */
  readonly focused: boolean;
  /** CSS px kept clear at the leading edge (e.g. 78 for macOS traffic lights; 0 in fullscreen). */
  readonly leadingInset: number;
  /** CSS px kept clear at the trailing edge for OS-drawn controls (0 when `custom`). */
  readonly trailingInset: number;
}

export interface BridgeSnapshot {
  readonly connection: ConnectionState;
  readonly hierarchy: LoadState<HierarchyTree>;
  readonly schemas: Readonly<Record<string, ComponentSchema>>;
  /**
   * Inspector data. Spike-sized: the whole set is in the snapshot. A real bridge
   * should page this by selection (see NATIVE_VIEWPORT.md open questions).
   */
  readonly entities: Readonly<Record<string, EntityDetail>>;
  readonly diagnostics: readonly Diagnostic[];
  readonly history: HistoryState;
  readonly console: readonly ConsoleEntry[];
  readonly provider: ProviderState;
  readonly agent: AgentState;
  readonly viewport: ViewportState;
  /** Present only inside the native host. */
  readonly window?: WindowChrome;
}

/** Document edits. Each becomes one undoable transaction on incant_cmd with origin `user`. */
export type EditorCommand =
  | { readonly type: 'entity.rename'; readonly entity: Ulid; readonly name: string }
  | { readonly type: 'entity.delete'; readonly entity: Ulid }
  | { readonly type: 'history.undo' }
  | { readonly type: 'history.redo' };

/** Requests to the host that are not document edits and are not journaled. */
export type HostRequest =
  | { readonly type: 'provider.connect'; readonly method: 'oauth' | 'api-key' }
  | { readonly type: 'agent.send'; readonly text: string }
  | {
      readonly type: 'viewport.bounds';
      /** CSS pixels relative to the webview's top-left, plus the device pixel ratio. */
      readonly rect: { readonly x: number; readonly y: number; readonly width: number; readonly height: number };
      readonly devicePixelRatio: number;
      /**
       * CSS px corner radii [top-left, top-right, bottom-right, bottom-left] the host should
       * clip the native surface to, so it matches the rounded viewport island.
       */
      readonly cornerRadii: readonly [number, number, number, number];
    }
  /** Begin an OS window drag from a titlebar mousedown (Tauri: Window::start_dragging). */
  | { readonly type: 'window.drag' }
  | { readonly type: 'window.minimize' }
  /** Toggle maximize (Windows/Linux) or zoom (macOS); also sent on titlebar double-click. */
  | { readonly type: 'window.maximize' }
  /** Request close; the host decides about unsaved-work prompts. */
  | { readonly type: 'window.close' };

export type BridgeResult =
  | { readonly ok: true }
  | { readonly ok: false; readonly error: BridgeError };

export interface EditorBridge {
  readonly protocolVersion: number;
  /** Raw advertised capability strings. The UI filters these against KNOWN_CAPABILITIES. */
  readonly capabilities: readonly string[];
  /** Human-readable origin of the data, shown in the shell (e.g. "Sample fixture"). */
  readonly label: string;
  /** True when the data is a sample fixture and not engine state. The shell labels it. */
  readonly isFixture: boolean;
  getSnapshot(): BridgeSnapshot;
  subscribe(listener: () => void): () => void;
  dispatch(command: EditorCommand): Promise<BridgeResult>;
  request(request: HostRequest): Promise<BridgeResult>;
}

declare global {
  interface Window {
    /** Injected by the Tauri host (editor/bridge) before the UI module runs. */
    __INCANT_BRIDGE__?: EditorBridge;
  }
}
