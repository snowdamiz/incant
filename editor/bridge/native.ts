/** Native transport for the Claude-owned UI contract. All document writes use incant_cmd. */
import type { BridgeSnapshot, BridgeResult, EditorBridge, EditorCommand, HostRequest, Ulid, HierarchyNode, EntityDetail, ComponentSchema, FieldSchema, Origin } from '../ui/src/bridge/contract';

type Entity = { id: string; name: string; parent: string | null; components: Record<string, Record<string, unknown>> };
type Project = { id: string; name: string; scenes: Record<string, {id: string; name: string; entities: Record<string, Entity>}> };
type History = { id: string; description: string; actor: {origin: 'user'|'agent'|'script'|'import'; actor: string; model: string | null; conversation_id: string | null} };
export interface EngineRead {
  project: Project; revision: number; can_redo: boolean; history: History[]; applied: number;
  schemas: Record<string, {title?: string; properties?: Record<string, FieldSchema>}>;
  console: string[]; viewport_error: string | null;
}
export type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
const id = (value: string) => value as Ulid;
function timestamp(ulid: string): string {
  const alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
  const value = [...ulid.slice(0, 10)].reduce((n, c) => n * 32 + alphabet.indexOf(c), 0);
  return new Date(value).toISOString();
}
function origin(actor: History['actor']): Origin {
  switch (actor.origin) {
    case 'agent': return {kind: 'agent', model: actor.model ?? 'unknown', conversation: actor.conversation_id ?? 'unknown'};
    case 'script': return {kind: 'script', script: actor.actor};
    case 'import': return {kind: 'import', source: actor.actor};
    default: return {kind: 'user'};
  }
}
export function snapshotFromEngine(read: EngineRead): BridgeSnapshot {
  const nodes: Record<string, HierarchyNode> = {};
  const entities: Record<string, EntityDetail> = {};
  const roots: Ulid[] = [];
  for (const scene of Object.values(read.project.scenes)) {
    roots.push(id(scene.id));
    nodes[scene.id] = {id: id(scene.id), name: scene.name, kind: 'scene', parent: null, children: Object.values(scene.entities).filter(e => !e.parent).map(e => id(e.id))};
    entities[scene.id] = {id: id(scene.id), name: scene.name, kind: 'scene', components: []};
    const children = new Map<string, Ulid[]>();
    for (const entity of Object.values(scene.entities)) {
      if (entity.parent) children.set(entity.parent, [...(children.get(entity.parent) ?? []), id(entity.id)]);
    }
    for (const entity of Object.values(scene.entities)) {
      const kind = 'Camera' in entity.components ? 'camera' : 'MeshRenderer' in entity.components ? 'mesh' : 'Script' in entity.components ? 'script' : 'group';
      nodes[entity.id] = {id: id(entity.id), name: entity.name, kind, parent: id(entity.parent ?? scene.id), children: children.get(entity.id) ?? []};
      entities[entity.id] = {id: id(entity.id), name: entity.name, kind, components: Object.entries(entity.components).map(([type,value])=>({type,schemaVersion:1,value}))};
    }
  }
  const schemas: Record<string, ComponentSchema> = {};
  for (const [type,schema] of Object.entries(read.schemas)) {
    schemas[type] = {type, version: 1, title: schema.title ?? type, properties: schema.properties ?? {}};
  }
  return {
    connection: {status:'ready',project:{id:id(read.project.id),name:read.project.name}},
    hierarchy: {status:'ready',value:{roots,nodes}}, schemas,entities,diagnostics:[],
    history: {entries:read.history.map(tx=>({transaction:id(tx.id),description:tx.description,origin:origin(tx.actor),at:timestamp(tx.id)})),applied:read.applied},
    console: read.console.map((message,i)=>({id:`native-${i}`,level:'info',source:'native',message,at:''})),
    provider: {status:'not-connected',provider:'openai'},
    agent: {status:'unavailable',reason:'Use the headless agent command for the Phase 0 provider spike.'},
    viewport: read.viewport_error ? {status:'error',error:{code:'viewport.failed',message:read.viewport_error}} : {status:'attached',surface:'Native wgpu'},
  };
}
function failure(error: unknown): BridgeResult {
  return {ok:false,error:{code:'engine.request',message:typeof error === 'string' ? error : 'The engine request failed.'}};
}
export class NativeBridge implements EditorBridge {
  readonly protocolVersion = 1;
  readonly capabilities = ['entity.rename','entity.delete','history.undo','history.redo','viewport.bounds'];
  readonly label = 'Native engine';
  readonly isFixture = false;
  private read: EngineRead | undefined;
  private listeners = new Set<() => void>();
  private pending = false;
  private snapshot: BridgeSnapshot = {connection:{status:'connecting'},hierarchy:{status:'loading'},schemas:{},entities:{},diagnostics:[],history:{entries:[],applied:0},console:[],provider:{status:'not-connected',provider:'openai'},agent:{status:'unavailable',reason:'Connecting to engine.'},viewport:{status:'not-attached',reason:'Connecting to native surface.'}};
  constructor(private invoke: Invoke) {}
  getSnapshot = () => this.snapshot;
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => {this.listeners.delete(listener);}; };
  private publish(read: EngineRead) { this.read=read; this.snapshot=snapshotFromEngine(read); this.listeners.forEach(fn=>fn()); }
  async start() {
    try { this.publish(await this.invoke<EngineRead>('engine_read')); }
    catch(error) {
      const result=failure(error); if(result.ok) return;
      this.snapshot={...this.snapshot,connection:{status:'error',error:result.error},hierarchy:{status:'error',error:result.error}};
      this.listeners.forEach(fn=>fn());
    }
  }
  async dispatch(command: EditorCommand): Promise<BridgeResult> {
    if(!this.read || this.pending) return {ok:false,error:{code:'engine.busy',message:'Wait for the current engine operation.'}};
    this.pending=true;
    try {
      let read: EngineRead;
      if(command.type === 'history.undo' || command.type === 'history.redo') {
        read=await this.invoke<EngineRead>('engine_history',{redo:command.type==='history.redo'});
      } else {
        const scene=Object.values(this.read.project.scenes).find(scene=>command.entity in scene.entities);
        if(!scene) return {ok:false,error:{code:'entity.not-found',message:'This operation requires an entity. Scene edits are not exposed in this spike.'}};
        const op=command.type==='entity.rename' ? {op:'rename_entity',scene_id:scene.id,entity_id:command.entity,name:command.name} : {op:'delete_entity',scene_id:scene.id,entity_id:command.entity};
        read=await this.invoke<EngineRead>('engine_execute',{commands:[op],description:command.type==='entity.rename' ? `Rename entity to ${command.name}` : 'Delete entity',expectedRevision:this.read.revision});
      }
      this.publish(read); return {ok:true};
    } catch(error) { await this.start(); return failure(error); }
    finally {this.pending=false;}
  }
  async request(request: HostRequest): Promise<BridgeResult> {
    if(request.type !== 'viewport.bounds') return {ok:false,error:{code:'unsupported',message:'This host capability is not connected.'}};
    const {rect,devicePixelRatio:dpr}=request;
    if(!Number.isFinite(dpr) || dpr<=0) return failure('Invalid device pixel ratio.');
    try { await this.invoke('viewport_bounds',{rect:[rect.x,rect.y,rect.width,rect.height].map(v=>v*dpr)}); return {ok:true}; }
    catch(error) {return failure(error);}
  }
}
export function installNativeBridge(): NativeBridge | undefined {
  const host=window as Window & {__TAURI__?: {core?: {invoke?: Invoke}}};
  const invoke=host.__TAURI__?.core?.invoke;
  if(!invoke || host.__INCANT_BRIDGE__) return undefined;
  const bridge=new NativeBridge(invoke);
  host.__INCANT_BRIDGE__=bridge;
  void bridge.start();
  return bridge;
}
