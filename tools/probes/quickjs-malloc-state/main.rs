use rquickjs::{ArrayBuffer, CatchResultExt, Context, Runtime, runtime::MallocState};
use std::sync::Arc;

const LIMIT: usize = 32 * 1024 * 1024;
const MIB: usize = 1024 * 1024;

fn evaluate(context: &Context, source: &str) {
    context.with(|ctx| {
        ctx.eval::<(), _>(source).catch(&ctx).unwrap();
    });
}

fn sample(runtime: &Runtime, name: &str) -> (MallocState, i64) {
    let (before, full, after) = runtime.diagnostic_malloc_state_with_usage();
    assert_eq!(before, after, "full scan changed scalar state: {name}");
    assert_eq!(
        before.allocated_bytes,
        usize::try_from(full.malloc_size).unwrap(),
        "bytes: {name}"
    );
    assert_eq!(
        before.allocation_count,
        usize::try_from(full.malloc_count).unwrap(),
        "count: {name}"
    );
    assert_eq!(
        before.limit_bytes,
        usize::try_from(full.malloc_limit).unwrap(),
        "limit: {name}"
    );
    assert_eq!(before, runtime.malloc_state());
    println!(
        "{}",
        serde_json::json!({"kind":"observation", "name":name,
        "allocated_bytes":before.allocated_bytes,"allocation_count":before.allocation_count,
        "limit_bytes":before.limit_bytes,"full_scan_estimated_used_bytes":full.memory_used_size,
        "same_lock_exact_parity":true,"continuous_peak":null})
    );
    (before, full.memory_used_size)
}

fn vm(name: &str) -> (Runtime, Context) {
    let runtime = Runtime::new().unwrap(); // Default libc, no custom allocator.
    runtime.set_memory_limit(LIMIT);
    runtime.set_max_stack_size(MIB);
    sample(&runtime, &format!("{name}/before-context"));
    let context = Context::full(&runtime).unwrap();
    sample(&runtime, &format!("{name}/after-context"));
    (runtime, context)
}

fn main() {
    let (runtime, context) = vm("first");
    let initial = sample(&runtime, "initial").0;
    evaluate(
        &context,
        "globalThis.rows=[]; for(let i=0;i<10000;i++) rows.push({x:i,y:i+1,z:'v'+i});",
    );
    let populated = sample(&runtime, "arena-populated").0;
    assert!(populated.allocated_bytes > initial.allocated_bytes);
    assert!(populated.allocation_count > initial.allocation_count);
    evaluate(&context, "for(let i=0;i<rows.length;i+=2)rows[i]=null;");
    runtime.run_gc();
    let half = sample(&runtime, "arena-half-freed").0;
    assert!(half.allocated_bytes < populated.allocated_bytes);
    evaluate(
        &context,
        "for(let i=0;i<rows.length;i+=2)rows[i]={x:i,y:i+1,z:'v'+i};",
    );
    sample(&runtime, "arena-recycled");
    evaluate(
        &context,
        "rows=null; globalThis.cycles=[]; for(let i=0;i<1000;i++){let a={};a.self=a;cycles.push(a);}",
    );
    sample(&runtime, "cycles-rooted");
    evaluate(&context, "cycles=null;");
    let before_gc = sample(&runtime, "cycles-unrooted").0;
    runtime.run_gc();
    let after_gc = sample(&runtime, "cycles-collected").0;
    assert!(after_gc.allocated_bytes < before_gc.allocated_bytes);

    // Native resizable buffers use js_array_buffer_resize -> realloc_func ->
    // js_realloc_rt. Preserve real native allocation/limit behavior, no allocator shim.
    evaluate(
        &context,
        "globalThis.rab=new ArrayBuffer(4096,{maxByteLength:64*1024*1024}); globalThis.view=new Uint8Array(rab); view[0]=90;view[4095]=165;",
    );
    let small = sample(&runtime, "realloc-small").0;
    evaluate(
        &context,
        "rab.resize(1024*1024); if(view[0]!==90||view[4095]!==165||view[900000]!==0)throw Error('growth bytes');",
    );
    let grown = sample(&runtime, "realloc-grown").0;
    assert!(grown.allocated_bytes > small.allocated_bytes);
    evaluate(
        &context,
        "rab.resize(4096); if(view.length!==4096||view[0]!==90||view[4095]!==165)throw Error('shrink bytes');",
    );
    let shrunk = sample(&runtime, "realloc-shrunk").0;
    assert!(shrunk.allocated_bytes < grown.allocated_bytes);
    evaluate(
        &context,
        "{let failed=false;try{rab.resize(64*1024*1024);}catch(e){failed=String(e).includes('out of memory');} if(!failed||rab.byteLength!==4096||view[0]!==90||view[4095]!==165)throw Error('realloc limit/old allocation');}",
    );
    assert_eq!(sample(&runtime, "realloc-refused").0.limit_bytes, LIMIT);
    evaluate(
        &context,
        "{let failed=false;try{new ArrayBuffer(32*1024*1024);}catch(e){failed=String(e).includes('out of memory');}if(!failed)throw Error('hard memory limit absent');}",
    );
    sample(&runtime, "allocation-refused");
    evaluate(&context, "rab=null;view=null;");
    runtime.run_gc();

    // A retained external Arc safely lends immutable bytes to JS. This covers
    // external backing accounting without introducing an unsafe raw-loan wrapper.
    // It is not a native ECS mutable-loan or rollback qualification.
    let backing: Arc<[u8]> = vec![71u8; MIB].into();
    let external_before = sample(&runtime, "external-before");
    context.with(|ctx| {
        let external =
            ArrayBuffer::from_source_immutable(ctx.clone(), Arc::clone(&backing)).unwrap();
        ctx.globals().set("external", external).unwrap();
    });
    let external_after = sample(&runtime, "external-attached");
    assert!(external_after.0.allocated_bytes - external_before.0.allocated_bytes < MIB);
    assert!(external_after.1 - external_before.1 >= MIB as i64);
    assert!(Arc::strong_count(&backing) > 1);
    evaluate(
        &context,
        "if(new Uint8Array(external)[123]!==71)throw Error('external data');globalThis.extView=new Uint8Array(external);",
    );
    context.with(|ctx| {
        let mut external: ArrayBuffer = ctx.globals().get("external").unwrap();
        external.detach();
    });
    evaluate(
        &context,
        "if(extView.length!==0||external.byteLength!==0)throw Error('external detach'); external=null;extView=null;",
    );
    runtime.run_gc();
    sample(&runtime, "external-revoked");
    assert_eq!(Arc::strong_count(&backing), 1);
    assert_eq!(backing[123], 71);

    let owned_before = sample(&runtime, "owned-before");
    evaluate(&context, "globalThis.owned=new ArrayBuffer(1024*1024);");
    let owned_after = sample(&runtime, "owned-attached");
    assert!(owned_after.0.allocated_bytes - owned_before.0.allocated_bytes >= MIB);
    assert!(owned_after.1 - owned_before.1 >= MIB as i64);
    evaluate(&context, "owned=null;");
    runtime.run_gc();
    sample(&runtime, "owned-freed");

    // A sampled scalar counter must not pretend to observe a transient peak.
    evaluate(&context, "globalThis.transient=null;");
    let before = sample(&runtime, "transient-before").0;
    evaluate(&context, "transient=new ArrayBuffer(2*1024*1024);");
    let during = sample(&runtime, "transient-during").0;
    evaluate(&context, "transient=null;");
    runtime.run_gc();
    let after = sample(&runtime, "transient-after").0;
    assert!(during.allocated_bytes > before.allocated_bytes.max(after.allocated_bytes) + MIB);

    let (other, other_context) = vm("second");
    let untouched = sample(&other, "independent-before").0;
    evaluate(&context, "transient=new ArrayBuffer(1024*1024);");
    sample(&runtime, "first-vm-mutated");
    assert_eq!(untouched, sample(&other, "independent-after").0);
    other.set_memory_limit(0);
    assert_eq!(sample(&other, "zero-means-unlimited").0.limit_bytes, 0);
    other.set_memory_limit(LIMIT);
    assert_eq!(sample(&other, "restored-limit").0.limit_bytes, LIMIT);
    drop(other_context);
    sample(&other, "second-context-dropped");
    drop(context);
    runtime.run_gc();
    sample(&runtime, "first-context-dropped");
    println!(
        "{}",
        serde_json::json!({"kind":"success","diagnostic_only":true,
        "default_libc":true,"hard_limit_bytes":LIMIT,"scalar_fullscan_exact":true,
        "timing_samples":0,"allocator_changed":false,"continuous_peak_claim":false,
        "raw_native_loan_qualification":false})
    );
}
