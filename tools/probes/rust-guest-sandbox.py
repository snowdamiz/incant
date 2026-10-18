"""One bounded, source-bound actual Mac isolation qualification. Not yet run.

This is trusted test infrastructure, not an engine-agent command capability.
The canary's extra executable is admitted only in its separately recorded policy.
No source is executed natively; project compilation always uses fixed wasm32.
"""
import argparse
from dataclasses import replace
import json
import os
from pathlib import Path
import signal
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import rust_guest_build as service
from rust_guest_sandbox import MacPolicy, Refused, clean_environment, require

ROOT = Path(__file__).resolve().parents[2]
VALID = '''#![no_std]
mod tuning;
use incant_module::{Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(update);
fn update(tick: &mut Tick<'_>, _: f64) -> Result<()> { tick.log(tuning::MESSAGE, incant_module::LogLevel::Info) }
'''


def check(plan_path, grant=None):
    plan_bytes = service.read_file(plan_path, service.MIB)
    plan = json.loads(plan_bytes)
    require(plan['version'] == 1 and plan['scope'] == 'untrusted-rust-macos-isolation', 'Probe plan scope')
    require(plan['bounds'] == {'seconds': 180, 'child_seconds': 90, 'scratch_bytes': 512 * service.MIB,
            'log_bytes': 16 * service.MIB, 'stream_bytes': 4 * service.MIB,
            'rss_bytes': 8 * 1024 * service.MIB, 'floor_bytes': 6 * 1024 * service.MIB}, 'Probe exact bounds')
    for row in plan['bindings']:
        value = service.read_file(Path(row['path']), row['bytes'])
        require(len(value) == row['bytes'] and service.digest(value) == row['sha256'], 'Probe binding changed: ' + row['path'])
    require(not Path(plan['output']).exists(), 'Create-only probe output already exists')
    if grant is not None:
        authority = json.loads(service.read_file(grant, 64 * 1024))
        require(authority.get('allowed') is True and authority.get('plan_sha256') == service.digest(plan_bytes)
                and authority.get('probe_sha256') == service.digest(Path(__file__).read_bytes()), 'Exact root grant required')
    return plan


def run(plan, cancelled, *, started=None):
    output = Path(plan['output'])
    output.mkdir(mode=0o700)
    # Publish wrapper identity before any native/tool initialization.
    service.write_json(output / 'wrapper.json', {'pid': os.getpid(), 'ppid': os.getppid(),
                       'pgid': os.getpgrp(), 'argv': sys.argv, 'source': plan['source']})
    bounds = plan['bounds']
    limits = service.Limits(**bounds)
    owner = service.Owner(output, limits, cancelled.is_set)
    if started is not None:
        owner.started = started  # Include binding preflight in the native episode.
    result = {'version': 1, 'status': 'failed', 'scope': plan['scope'], 'source': plan['source'],
              'calls': owner.calls, 'cases': [], 'no_project_native_execution': True}
    secret = output / 'private-sentinel'
    secret.write_bytes(b'INCANT_PRIVATE_INCLUDE_SENTINEL_90843214')
    tools = service.Tools(Path(plan['rustc']), Path(plan['validator']))

    def guard():
        owner.check()
        logs = list(output.rglob('*.stdout')) + list(output.rglob('*.stderr'))
        require(all(path.stat().st_size <= limits.stream_bytes for path in logs)
                and sum(path.stat().st_size for path in logs) <= limits.log_bytes, 'Whole probe log cap')
        return False

    def case(name, source_text, expected=None, extra=None, cancel=False):
        guard()
        directory = output / name
        directory.mkdir(mode=0o700)
        package = directory / 'source'
        package.mkdir(mode=0o700)
        (package / 'module.rs').write_text(source_text)
        (package / 'tuning.rs').write_text('pub const MESSAGE: &str = "sandboxed SDK";\n')
        if extra:
            (package / extra).write_text('must never run as a build script')
        work = directory / 'work'
        publication = directory / 'module.wasm'
        def cancellation():
            guard()
            return cancel and any(work.glob('*-untrusted-compile.owner.json'))
        remaining = limits.seconds - (time.monotonic() - owner.started)
        row = {'name': name, 'expected_refusal': expected, 'status': 'pending'}
        result['cases'].append(row)
        try:
            built = service.build(service.Request(package, 'module.rs', publication, work), tools,
                                  limits=replace(limits, seconds=remaining,
                                                 child_seconds=min(limits.child_seconds, remaining)),
                                  cancel=cancellation)
        except (Refused, RuntimeError) as error:
            require(expected is not None and expected in str(error), f'Unexpected {name} refusal: {error}')
            row.update(status='expected-refusal', error=str(error))
            require(not publication.exists(), 'Failed child published raw Wasm')
        else:
            require(expected is None, 'Expected refusal unexpectedly succeeded: ' + name)
            row.update(status='complete', raw_sha256=built['raw_sha256'])
            require(publication.read_bytes() == (work / 'run/module.wasm').read_bytes(), 'Raw/prepared publication confusion')
        # Secret data must not be emitted in compiler diagnostics or raw modules.
        for path in directory.rglob('*'):
            if path.is_file() and path.suffix in ('.stdout', '.stderr', '.wasm'):
                require(secret.read_bytes() not in service.read_file(path, limits.stream_bytes), 'Private source leaked')
        guard()

    try:
        case('valid-relative-sdk', VALID)
        case('cargo-refused', VALID, 'non-Rust inputs', extra='Cargo.toml')
        case('build-script-refused', VALID, 'build scripts', extra='build.rs')
        case('private-include-refused', VALID + '\nconst PRIVATE: &[u8] = include_bytes!(' + json.dumps(str(secret)) + ');\n',
             'Sandboxed Rust compiler refused')
        case('proc-macro-refused', '#![crate_type="proc-macro"]\nextern crate proc_macro;\n',
             'Sandboxed Rust compiler refused')
        case('external-macro-refused', VALID + '\nextern crate project_macro;\n', 'Sandboxed Rust compiler refused')
        unknown = VALID.replace('tick.log(tuning::MESSAGE, incant_module::LogLevel::Info)', 'unsafe { forbidden(); } Ok(())')
        unknown += '\n#[link(wasm_import_module="wasi_snapshot_preview1")]\nunsafe extern "C" { fn forbidden(); }\n'
        case('normal-validator-refused', unknown, 'Normal Wasm cook validator refused')
        case('cancelled-compiler', VALID, 'Build cancelled', cancel=True)

        # The trusted OS canary uses the production policy with precisely one
        # executable substitution. No source-controlled program gains this rule.
        control = output / 'sandbox-control'
        env_run = output / 'control-run'
        env_run.mkdir(mode=0o700)
        for name in ('home', 'tmp'):
            (env_run / name).mkdir(mode=0o700)
        environment = clean_environment(env_run, tools.rustc)
        compiled = owner.run([str(tools.rustc), str(ROOT / 'tools/probes/rust-guest-sandbox-control.rs'),
                              '--edition=2024', '--crate-name=incant_sandbox_control', '-C', 'opt-level=0',
                              '-C', 'debuginfo=0', '-o', str(control)], cwd=env_run, env=environment, label='trusted-control-build')
        require(compiled.returncode == 0, 'Trusted native control did not compile')
        valid_work = output / 'valid-relative-sdk/work'
        initial = service.read_file(valid_work / 'result.json', service.MIB)
        initial = json.loads(initial)
        rustc, linker, validator = (Path(row['path']) for row in initial['tools'])
        policy = MacPolicy(control, linker, validator, rustc.parent.parent,
                           valid_work / 'sdk', valid_work / 'source', env_run)
        policy_path = output / 'control-policy.sb'
        policy_path.write_text(policy.text())
        executed = owner.run([str(MacPolicy.backend()), '-f', str(policy_path), str(control), str(secret),
                              str(output / 'must-not-write'), str(env_run / 'actual-write')],
                             cwd=env_run, env=environment, label='trusted-control-run')
        require(executed.returncode == 0, 'Actual sandbox escape control failed')
        controls = json.loads(executed.stdout)
        require(controls == dict.fromkeys(('read_denied', 'write_denied', 'exec_denied',
                                          'socket_denied', 'clean_environment'), True), 'Exact actual control outputs')
        require((env_run / 'actual-write').read_bytes() == b'control executed'
                and not (output / 'must-not-write').exists(), 'Actual canary write boundary')
        result['os_control'] = controls
        # Postflight input/tool bindings; output no longer absent, so compare
        # immutable rows directly rather than invoking launch-only check().
        for row in plan['bindings']:
            require(service.digest(service.read_file(Path(row['path']), row['bytes'])) == row['sha256'], 'Postflight input changed')
        guard()
        result['status'] = 'complete'
    except BaseException as error:
        result.update(error_type=type(error).__name__, error=str(error))
        raise
    finally:
        nested = []
        for path in output.glob('*/work/*.json'):
            if path.name in ('result.json', 'failure.json'):
                nested.append(json.loads(service.read_file(path, service.MIB)))
        logs = list(output.rglob('*.stdout')) + list(output.rglob('*.stderr'))
        result.update(elapsed_seconds=time.monotonic() - owner.started,
                      peak_scratch_bytes=max(owner.peak_scratch, service.inventory(output)),
                      sampled_peak_rss_bytes=max([owner.peak_rss, *(row.get('sampled_peak_rss_bytes', 0) for row in nested)]),
                      diagnostic_bytes=sum(path.stat().st_size for path in logs),
                      nested_build_receipts=[str(path.relative_to(output)) for path in output.glob('*/work/*.json')
                                             if path.name in ('result.json', 'failure.json')])
        service.write_json(output / 'result.json', result)
    return result


def main():
    started = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--grant', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    plan = check(args.plan, args.grant)
    if args.check:
        print(json.dumps({'status': 'ready', 'bindings': len(plan['bindings']), 'source': plan['source']}))
        return
    require(args.grant is not None, 'Native execution requires exact root grant')
    cancelled = threading.Event()
    for number in (signal.SIGINT, signal.SIGTERM):
        signal.signal(number, lambda *_: cancelled.set())
    print(json.dumps(run(plan, cancelled, started=started), indent=2))


if __name__ == '__main__':
    main()
