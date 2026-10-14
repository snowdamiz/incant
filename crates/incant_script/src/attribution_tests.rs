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
