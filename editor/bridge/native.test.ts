import { describe, expect, it } from 'vitest';
import { NativeBridge, snapshotFromEngine } from './native';
import type { EngineRead, Invoke } from './native';
import type { Ulid } from '../ui/src/bridge/contract';
const scene = '00000000000000000000000002';
const entity = '00000000000000000000000010';
function read(): EngineRead {
  return {project:{id:'00000000000000000000000001',name:'Test',scenes:{[scene]:{id:scene,name:'Main',entities:{[entity]:{id:entity,name:'Cube',parent:null,components:{Transform:{translation:[0,0,0],rotation:[0,0,0,1],scale:[1,1,1]}}}}}},revision:7,can_redo:false,history:[],applied:0,schemas:{Transform:{properties:{translation:{type:'array',items:{type:'number'},minItems:3,maxItems:3}}}},console:[],viewport_error:null};
}
describe('native bridge',()=>{
  it('projects the Rust scene hierarchy and component schema without inventing capabilities',()=>{
    const value=snapshotFromEngine(read());
    expect(value.hierarchy.status).toBe('ready');
    if(value.hierarchy.status!=='ready') throw new Error('not ready');
    expect(value.hierarchy.value.roots).toEqual([scene]);
    expect(value.hierarchy.value.nodes[entity]?.parent).toBe(scene);
    expect(value.entities[entity]?.components[0]?.type).toBe('Transform');
    expect(value.agent.status).toBe('unavailable');
  });
  it('sends a rename and observed revision through IPC, then uses only the returned engine state',async()=>{
    const calls: {command:string;args:Record<string,unknown>|undefined}[]=[];
    const invoke:Invoke=async<T>(command:string,args?:Record<string,unknown>)=>{
      calls.push({command,args}); const result=read();
      if(command==='engine_execute') {result.project.scenes[scene]!.entities[entity]!.name='Renamed';result.revision=8;}
      return result as T;
    };
    const bridge=new NativeBridge(invoke);await bridge.start();
    const before=bridge.getSnapshot();expect(bridge.getSnapshot()).toBe(before);
    expect(Object.isFrozen(before.entities[entity]?.components[0]?.value)).toBe(true);
    expect(await bridge.dispatch({type:'entity.rename',entity:entity as Ulid,name:'Renamed'})).toEqual({ok:true});
    expect(calls[1]?.command).toBe('engine_execute');
    expect(calls[1]?.args?.expectedRevision).toBe(7);
    expect(calls[1]?.args?.commands).toEqual([{op:'rename_entity',scene_id:scene,entity_id:entity,name:'Renamed'}]);
    expect(bridge.getSnapshot().entities[entity]?.name).toBe('Renamed');
    expect(before.entities[entity]?.name).toBe('Cube');
  });
  it('rejects unsupported host requests without issuing IPC',async()=>{
    let calls=0;const invoke:Invoke=async<T>()=>{calls++;return read() as T;};
    const bridge=new NativeBridge(invoke);
    const result=await bridge.request({type:'agent.send',text:'anything'});
    expect(result.ok).toBe(false);expect(calls).toBe(0);
  });
  it('converts viewport bounds to physical pixels without mutating the document',async()=>{
    let request:unknown;const invoke:Invoke=async<T>(command,args)=>{request={command,args};return undefined as T;};
    const bridge=new NativeBridge(invoke);
    expect(await bridge.request({type:'viewport.bounds',rect:{x:100,y:40,width:500,height:300},devicePixelRatio:2})).toEqual({ok:true});
    expect(request).toEqual({command:'viewport_bounds',args:{rect:[200,80,1000,600]}});
  });
  it('keeps engine errors visible and does not apply optimistic document edits',async()=>{
    const invoke:Invoke=async<T>(command)=>{if(command==='engine_execute') throw 'revision conflict';return read() as T;};
    const bridge=new NativeBridge(invoke);await bridge.start();
    const result=await bridge.dispatch({type:'entity.rename',entity:entity as Ulid,name:'Not applied'});
    expect(result.ok).toBe(false);expect(bridge.getSnapshot().entities[entity]?.name).toBe('Cube');
  });
});
