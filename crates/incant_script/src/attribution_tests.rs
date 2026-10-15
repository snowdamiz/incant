//! Actual host behavior while the explicit diagnostic feature is selected.
use super::{
    CompiledScript, ScriptHost,
    tests::{check, id, world},
};

#[test]
fn attribution_covers_success_and_retains_rollback_and_detachment_on_failure() {
    let code = CompiledScript::compile("exports.default={update({chunks}){globalThis.saved=chunks[0].transforms;for(const c of chunks)c.transforms[0]+=1;}};").unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    let capture = incant_profile::Capture::start().unwrap();
    incant_profile::frame(|| host.tick(&mut world, 1. / 60.)).unwrap();
    check(
        &host,
        "if(saved.length!==0||saved.buffer.byteLength!==0)throw Error('not detached')",
    );
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 1.);
    let report = capture.report();
    let trace = &report.script_phase_diagnostic;
    assert!(trace.complete);
    assert_eq!(trace.rows.len(), 1);
    assert_eq!(trace.rows[0].numeric_calls, [1, 1, 1]);
    assert_eq!(trace.rows[0].source_calls.iter().sum::<u16>(), 1);
    assert_eq!(report.scripts[0].totals.calls, 1);
    drop(capture);

    let code = CompiledScript::compile("exports.default={update({chunks}){globalThis.saved=chunks[0].transforms;for(const c of chunks)c.transforms[0]=NaN;}};").unwrap();
    let mut failed = ScriptHost::new(code).unwrap();
    let before = world.snapshot();
    let capture = incant_profile::Capture::start().unwrap();
    assert!(incant_profile::frame(|| failed.tick(&mut world, 1. / 60.)).is_err());
    assert_eq!(world.snapshot(), before);
    check(
        &failed,
        "if(saved.length!==0||saved.buffer.byteLength!==0)throw Error('not detached')",
    );
    let report = capture.report();
    assert!(!report.script_phase_diagnostic.complete);
    assert_eq!(report.failed_frames, 1);
    assert_eq!(report.script_phase_diagnostic.rows.len(), 1);
    assert!(!report.script_phase_diagnostic.rows[0].success);
}

#[cfg(feature = "diagnostic-script-budget")]
#[test]
fn budget_attribution_observes_real_vm_interrupt_and_explicit_checks_without_changing_writes() {
    use std::time::Duration;
    let code = CompiledScript::compile("exports.default={update({chunks,physics}){globalThis.saved=chunks[0].transforms;let sum=0;for(let i=0;i<50000;i++)sum+=i;physics.triggerEvents();chunks[0].transforms[0]=sum%17;}};").unwrap();
    let mut host = ScriptHost::with_budget(code, Duration::from_secs(1)).unwrap();
    let mut world = world();
    let capture = incant_profile::Capture::start().unwrap();
    incant_profile::frame(|| host.tick(&mut world, 1. / 60.)).unwrap();
    check(
        &host,
        "if(saved.length!==0||saved.buffer.byteLength!==0)throw Error('not detached')",
    );
    assert_eq!(
        world.inspect(id(1)).unwrap().transform.translation[0],
        (0..50000_u64).sum::<u64>() as f64 % 17.
    );
    let report = capture.report();
    let diagnostic = report.script_phase_diagnostic;
    assert!(diagnostic.complete);
    let counters = &diagnostic.rows[0].budget_checks;
    assert!(counters.calls[0] > 0); // real QuickJS interrupt callback
    assert_eq!(counters.calls[1], 2); // before/after the unchanged native query
    assert!(counters.calls[3] >= 3); // actual explicit host boundaries
    assert_eq!(counters.refused, [0; 4]);
    assert_eq!(counters.calls, counters.clock_reads);
    drop(capture);

    let code = CompiledScript::compile("exports.default={update({chunks}){globalThis.saved=chunks[0].transforms;chunks[0].transforms[0]=99;for(;;){}}};").unwrap();
    let mut host = ScriptHost::with_budget(code, Duration::from_millis(50)).unwrap();
    let before = world.snapshot();
    let capture = incant_profile::Capture::start().unwrap();
    assert!(incant_profile::frame(|| host.tick(&mut world, 1. / 60.)).is_err());
    assert_eq!(world.snapshot(), before);
    assert!(host.failed);
    // Test-only inspection must not itself reuse the deliberately expired budget.
    host.deadline.lock().unwrap().reset().unwrap();
    check(
        &host,
        "if(saved.length!==0||saved.buffer.byteLength!==0)throw Error('not detached')",
    );
    let report = capture.report();
    let diagnostic = report.script_phase_diagnostic;
    assert!(!diagnostic.complete && !diagnostic.rows[0].success);
    assert!(diagnostic.rows[0].budget_checks.calls[0] > 0);
    assert_eq!(diagnostic.rows[0].budget_checks.refused[0], 1);
    // Diagnostic error extraction can repeat the latched check with no new read.
    assert!(diagnostic.rows[0].budget_checks.refused[3] >= 1);
}
