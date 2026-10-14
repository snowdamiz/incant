import { expect, it } from 'vitest';
import { NativeBridge, type EngineRead, type Invoke } from './native';
import type { HostRequest, Ulid } from './contract';
const project='01ARZ3NDEKTSV4RRFFQ69G5FAX';
const review='01ARZ3NDEKTSV4RRFFQ69G5FAY';
const read = (): EngineRead => ({ project:{id:project,name:'Test',scenes:{}}, revision:0,can_redo:false,history:[],applied:0,schemas:{},console:[],viewport_error:null,project_file:{available:true} });
it('uses only the current host revision and retained review ID, with no caller path', async () => {
  const requests: unknown[]=[];
  const current=read();
  const invoke: Invoke=async <T>(name: string,args?: Record<string,unknown>) => {
    if (name==='engine_read') return current as T;
    expect(name).toBe('engine_project_file'); requests.push(args);
    return {snapshot:current,response:{operation:'review_pin',review:{review_id:review,expected_revision:0,source_sha256:'a'.repeat(64),candidate_sha256:'b'.repeat(64),candidate_text:'{}',command:{op:'set_engine_version',requirement:'=0.0.1'}}}} as T;
  };
  const bridge=new NativeBridge(invoke); await bridge.start();
  expect(bridge.capabilities).toContain('project.file');
  const result=await bridge.request({type:'project.pin.review'});
  expect(result.ok).toBe(true); if(result.ok) expect(result.projectFile?.operation).toBe('review_pin');
  expect(requests).toEqual([{request:{operation:'review_pin',expected_revision:0}}]);
  expect((await bridge.request({type:'project.pin.apply',reviewId:review as Ulid,path:'evil'} as unknown as HostRequest)).ok).toBe(false);
  expect(requests).toHaveLength(1);
});
it('does not advertise old-host file authority or accept a malformed success receipt', async () => {
  const current=read(); delete current.project_file;
  const bridge=new NativeBridge(async <T>()=>current as T); await bridge.start();
  expect(bridge.capabilities).not.toContain('project.file');
  expect((await bridge.request({type:'project.save'})).ok).toBe(false);
  const available=read();
  const malformed=new NativeBridge(async <T>(name: string)=> (name==='engine_read'?available:{snapshot:available,response:{operation:'save',receipt:{revision:1,changed:true,source_sha256:'a'.repeat(64),candidate_sha256:'b'.repeat(64)}}}) as T);
  await malformed.start(); expect((await malformed.request({type:'project.save'})).ok).toBe(false);
});

it('publishes successful apply and save receipts on the same host revision', async () => {
  let current=read();
  const requests: unknown[]=[];
  const bridge=new NativeBridge(async <T>(name: string,args?: Record<string,unknown>) => {
    if(name==='engine_read') return current as T;
    expect(name).toBe('engine_project_file'); requests.push(args);
    const request=args?.request as {operation:string; expected_revision:number};
    expect(request.expected_revision).toBe(current.revision);
    if(request.operation==='apply_pin') {
      current={...read(),revision:1};
      return {snapshot:current,response:{operation:'apply_pin',transaction_id:review,revision:1}} as T;
    }
    return {snapshot:current,response:{operation:'save',receipt:{revision:1,changed:false,source_sha256:'a'.repeat(64),candidate_sha256:'a'.repeat(64)}}} as T;
  });
  await bridge.start();
  expect(await bridge.request({type:'project.pin.apply',reviewId:review as Ulid})).toMatchObject({ok:true,projectFile:{operation:'apply_pin',revision:1}});
  expect(await bridge.request({type:'project.save'})).toMatchObject({ok:true,projectFile:{operation:'save',receipt:{revision:1,changed:false}}});
  expect(requests).toEqual([{request:{operation:'apply_pin',expected_revision:0,review_id:review}},{request:{operation:'save',expected_revision:1}}]);
});
it('rejects duplicate pending requests and refreshes actual host state after failure', async () => {
  let current=read();
  let reads=0, calls=0;
  let rejectFile: (reason: unknown)=>void=()=>{throw new Error('no pending operation');};
  const bridge=new NativeBridge(async <T>(name: string,args?: Record<string,unknown>) => {
    if(name==='engine_read') {++reads;return current as T;}
    expect(name).toBe('engine_project_file');
    expect(args).toEqual({request:{operation:'save',expected_revision:current.revision}}); ++calls;
    return await new Promise<T>((_,reject)=>{rejectFile=reject;});
  });
  await bridge.start();
  const saving=bridge.request({type:'project.save'});
  expect((await bridge.request({type:'project.pin.review'})).ok).toBe(false);
  expect(calls).toBe(1);
  current={...read(),revision:2};
  rejectFile({code:'project.conflict',message:'Host rejected stale file identity.'});
  expect(await saving).toMatchObject({ok:false,error:{code:'project.conflict'}});
  expect(reads).toBe(2);
  const next=bridge.request({type:'project.save'});
  expect(calls).toBe(2);
  rejectFile('another refusal'); await next;
});

it.each(['invalid receipt', 'rejected reply'] as const)('does not infer rollback after a committed pin and %s', async (kind) => {
  let current = read();
  let writes = 0;
  const original = {code:'transport.closed', message:'The reply channel closed.'};
  const bridge = new NativeBridge(async <T>(name: string) => {
    if (name === 'engine_read') return current as T;
    expect(name).toBe('engine_project_file');
    ++writes;
    // The host committed, but the caller cannot verify its completion receipt.
    current = {...read(), revision:1};
    if (kind === 'rejected reply') throw original;
    return {snapshot:current, response:{operation:'apply_pin', transaction_id:'invalid', revision:1}} as T;
  });
  await bridge.start();
  const result = await bridge.request({type:'project.pin.apply', reviewId:review as Ulid});
  expect(result).toEqual({ok:false, outcomeUnknown:true, error:kind === 'rejected reply' ? original : {code:'engine.request', message:'Invalid project file response.'}});
  expect(writes).toBe(1);
  expect(bridge.getSnapshot().connection).toMatchObject({status:'ready', project:{id:project}});
  // An invalid caller request never reaches the host and has no uncertain write.
  const invalid = await bridge.request({type:'project.pin.apply',reviewId:'invalid' as Ulid});
  expect(invalid).toMatchObject({ok:false});
  expect('outcomeUnknown' in invalid).toBe(false);
  expect(writes).toBe(1);
});
