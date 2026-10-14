use rquickjs::{ArrayBuffer, Context, Function, Runtime, Value};
use std::{path::Path, time::Instant};

fn semantics(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    runtime.set_memory_limit(32 * 1024 * 1024);
    let context = Context::full(&runtime)?;
    let source = std::fs::read(root.join("semantics.js"))?;
    let rows = context.with(|ctx| -> rquickjs::Result<String> {
        let integer = Value::new_int(ctx.clone(), 7);
        let float = Value::new_float(ctx.clone(), 7.0);
        assert!(integer.is_int() && float.is_float());
        ctx.globals().set("tagInt", integer)?;
        ctx.globals().set("tagFloat", float)?;
        ctx.eval(source)
    })?;
    let rows: serde_json::Value = serde_json::from_str(&rows)?;
    assert_eq!(rows.as_array().unwrap().len(), 58);
    // Match the real host's explicit ArrayBuffer detachment boundary with the
    // safe rquickjs owner. This is not an Incant loan/rollback integration test.
    context.with(|ctx| -> rquickjs::Result<()> {
        let mut buffer = ArrayBuffer::new(ctx.clone(), vec![111.0f64, 222.0])?;
        ctx.globals().set("owned", buffer.clone())?;
        ctx.eval::<(), _>("globalThis.retained = new Float64Array(owned); retained[0] = 7.5;")?;
        buffer.detach();
        ctx.eval::<(), _>("retained[0]=9.5; if(retained.length!==0 || retained[0]!==undefined || owned.byteLength!==0) throw Error('revoked buffer revived')")?;
        Ok(())
    })?;
    println!(
        "{}",
        serde_json::json!({"kind":"semantics", "cases":rows,"owned_buffer_detach":true})
    );
    Ok(())
}

fn stress(root: &Path, measure: bool) -> Result<(), Box<dyn std::error::Error>> {
    for name in ["omitted", "nanY", "nanZ"] {
        let runtime = Runtime::new()?;
        runtime.set_memory_limit(32 * 1024 * 1024);
        let context = Context::full(&runtime)?;
        let source = std::fs::read(root.join(format!("{name}.js")))?;
        context.with(|ctx| -> rquickjs::Result<()> {
            ctx.eval::<(), _>(source)?;
            let update: Function = ctx.globals().get("update")?;
            let verify: Function = ctx.globals().get("verify")?;
            update.call::<_, ()>((1u32,))?;
            assert!(
                verify.call::<_, ()>((1u32,)).is_err(),
                "negative escaped: {name}"
            );
            Ok(())
        })?;
        println!(
            "{}",
            serde_json::json!({"kind":"oracle_negative", "name":name,"refused":true})
        );
    }
    let runtime = Runtime::new()?;
    runtime.set_memory_limit(32 * 1024 * 1024);
    let context = Context::full(&runtime)?;
    let source = std::fs::read(root.join("baseline.js"))?;
    let result = context.with(|ctx| -> rquickjs::Result<Vec<u64>> {
        ctx.eval::<(), _>(source)?;
        let update: Function = ctx.globals().get("update")?;
        let verify: Function = ctx.globals().get("verify")?;
        let transitions: Function = ctx.globals().get("transitions")?;
        let mut samples = Vec::with_capacity(if measure { 500 } else { 0 });
        for tick in 1..=500u32 {
            let started = measure.then(Instant::now);
            update.call::<_, ()>((tick,))?;
            if let Some(started) = started {
                samples.push(started.elapsed().as_nanos() as u64);
            }
            // Every row, padding cell and finite checksum is checked outside
            // the update interval on every tick, not just sampled checkpoints.
            verify.call::<_, ()>((tick,))?;
        }
        transitions.call::<_, ()>(())?;
        Ok(samples)
    })?;
    println!(
        "{}",
        serde_json::json!({"kind":"stress", "diagnostic_only":true,
        "ticks":500,"rows":9999,"every_tick_verified":true,"transitions":3,"ns":result,
        "quickjs_heap_used":runtime.memory_usage().memory_used_size,
        "scope":"Standalone identical stress callback; no native host, shipping or gate claim"})
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() == 3 && ["--functional", "--measure"].contains(&args[2].as_str()));
    let root = Path::new(&args[1]);
    semantics(root)?;
    stress(root, args[2] == "--measure")
}
