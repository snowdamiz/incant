"""One bounded, source-bound actual Mac isolation qualification. Not yet run.

This is trusted test infrastructure, not an engine-agent command capability.
The canary's extra executable is admitted only in its separately recorded policy.
No project source is executed natively; its compilation always uses fixed wasm32.
"""
import argparse
from dataclasses import replace
import json
import os
from pathlib import Path
import signal
import socket
import shutil
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
incant_module::module!(on_update);
fn on_update(tick: &mut Tick<'_>, _: f64) -> Result<()> { tick.log(tuning::MESSAGE, incant_module::LogLevel::Info) }
'''
PROC_MACRO = '''#![crate_type="proc-macro"]
extern crate proc_macro;
#[proc_macro]
pub fn forbidden(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream { tokens }
'''


def validate_control(controls):
    """Exact untrusted canary protocol; ordinary connection failures are not denial evidence."""
    require(isinstance(controls, dict) and set(controls) == {
        'criterion', 'read_denied', 'write_denied', 'exec_denied', 'clean_environment',
        'sockets_created', 'socket_creation_denied', 'network'}, 'Exact canary protocol fields')
    require(controls['criterion'] == 'network-io-v2', 'Explicit network criterion revision')
    require(all(controls[name] is True for name in (
        'read_denied', 'write_denied', 'exec_denied', 'clean_environment')), 'Actual OS isolation controls')
    require(type(controls['sockets_created']) is int and controls['sockets_created'] == 4
            and controls['socket_creation_denied'] is False, 'Four created sockets; no creation-denial claim')
    network = controls['network']
    require(isinstance(network, dict) and set(network) == {
        'tcp_connect', 'tcp_bind', 'udp_bind', 'udp_send_to'}, 'Exact network operation inventory')
    for name, row in network.items():
        require(isinstance(row, dict) and set(row) == {'return', 'errno'}
                and type(row['return']) is int and row['return'] == -1
                and type(row['errno']) is int and row['errno'] in (1, 13),
                'Network operation must fail EPERM/EACCES: ' + name)
    return controls


def run_control(owner, tools, valid_work, output, plan):
    """Trusted canary only; no project native execution or inherited network descriptors."""
    secret = output / 'private-sentinel'
    control = output / 'sandbox-control'
    env_run = output / 'control-run'
    env_run.mkdir(mode=0o700)
    for name in ('home', 'tmp'):
        (env_run / name).mkdir(mode=0o700)
    environment = clean_environment(env_run, tools.rustc)
    command = [str(tools.rustc), str(ROOT / 'tools/probes/rust-guest-sandbox-control.rs'),
               '--edition=2024', '--crate-name=incant_sandbox_control', '-C', 'opt-level=0',
               '-C', 'debuginfo=0', '-C', 'linker=' + plan['native_linker'],
               '-C', 'link-arg=-isysroot', '-C', 'link-arg=' + plan['macos_sdk'], '-o', str(control)]
    reuse = plan.get('trusted_canary_reuse')
    if reuse is None:
        compiled = owner.run(command, cwd=env_run, env=environment, label='trusted-control-build')
        require(compiled.returncode == 0, 'Trusted native control did not compile')
    else:
        require(set(reuse) == {'path', 'bytes', 'sha256', 'source_sha256', 'build_receipt'}, 'Exact trusted canary reuse fields')
        control = service.trusted_file(Path(reuse['path']))
        data = service.read_file(control, 16 * service.MIB)
        require(len(data) == reuse['bytes'] and service.digest(data) == reuse['sha256'], 'Trusted canary bytes changed')
        require(service.digest(service.read_file(Path(command[1]), service.MIB)) == reuse['source_sha256'],
                'Trusted canary source changed')
        built = json.loads(service.read_file(Path(reuse['build_receipt']), service.MIB))
        require(built['returncode'] == 0 and built['group_absent'] is True
                and built['argv'] == [*command[:-1], str(control)], 'Exact prior canary compilation recipe')
        service.write_json(output / 'trusted-canary-reuse.json', reuse)
    initial = service.read_file(valid_work / 'result.json', service.MIB)
    initial = json.loads(initial)
    rustc, linker, validator = (Path(row['path']) for row in initial['tools'])
    policy = MacPolicy(control, linker, validator, rustc.parent.parent,
                       valid_work / 'sdk', valid_work / 'source', env_run)
    policy_path = output / 'control-policy.sb'
    policy_path.write_text(policy.text())
    endpoints = []
    receipt = {'criterion': 'network-io-v2', 'network_policy_changed': False,
               'external_addresses_or_dns': False, 'tcp_listen_called': False,
               'sink_wait_seconds': 0.05, 'endpoint_count_bound': 2}
    try:
        tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        endpoints.append(tcp)
        udp = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        endpoints.append(udp)
        for endpoint in endpoints:
            endpoint.set_inheritable(False)
            require(not endpoint.get_inheritable(), 'Network descriptors must not be inherited')
            endpoint.bind(('127.0.0.1', 0))
            address, port = endpoint.getsockname()
            require(address == '127.0.0.1' and 0 < port <= 65535, 'Owned loopback endpoint required')
        require((tcp.family, tcp.type) == (socket.AF_INET, socket.SOCK_STREAM)
                and (udp.family, udp.type) == (socket.AF_INET, socket.SOCK_DGRAM),
                'Exact owned endpoint protocols')
        # No listen() call exists in this trusted setup. SO_ACCEPTCONN querying
        # returned ENOPROTOOPT on the tested Mac; it is not an I/O test.
        receipt['endpoint_protocols'] = ['AF_INET/SOCK_STREAM', 'AF_INET/SOCK_DGRAM']
        receipt['tcp_reservation'] = tcp.getsockname()
        receipt['udp_sink'] = udp.getsockname()
        udp.settimeout(receipt['sink_wait_seconds'])
        executed = owner.run([str(MacPolicy.backend()), '-f', str(policy_path), str(control), str(secret),
                              str(output / 'must-not-write'), str(env_run / 'actual-write'),
                              str(tcp.getsockname()[1]), str(udp.getsockname()[1])],
                             cwd=env_run, env=environment, label='trusted-control-run')
        try:
            payload, address = udp.recvfrom(256)
            receipt.update(sink_datagrams=1, sink_payload_bytes=len(payload),
                           sink_payload_sha256=service.digest(payload), sink_peer=address)
        except (TimeoutError, BlockingIOError):
            receipt.update(sink_datagrams=0, sink_payload_bytes=0)
        require(receipt['sink_datagrams'] == 0, 'Loopback sink received a datagram')
        require(executed.returncode == 0, 'Actual sandbox escape control failed')
        controls = validate_control(json.loads(executed.stdout))
        if reuse is not None:
            require(service.digest(service.read_file(control, 16 * service.MIB)) == reuse['sha256'],
                    'Trusted canary changed during execution')
        require((env_run / 'actual-write').read_bytes() == b'control executed'
                and not (output / 'must-not-write').exists(), 'Actual canary write boundary')
        receipt['status'] = 'complete'
        return controls
    except BaseException as error:
        receipt.update(status='failed', error_type=type(error).__name__, error=str(error))
        raise
    finally:
        def close_and_record():
            errors = []
            for endpoint in endpoints:
                try:
                    endpoint.close()
                    require(endpoint.fileno() == -1, 'Owned loopback endpoint remains open')
                except BaseException as error:
                    errors.append(error)
            receipt['endpoints_closed'] = all(endpoint.fileno() == -1 for endpoint in endpoints)
            try:
                service.write_json(output / 'control-network.json', receipt)
            except BaseException as error:
                errors.append(error)
            if errors:
                raise BaseExceptionGroup('Loopback endpoint cleanup/receipt failures', errors)
        service.native_process._cleanup(close_and_record, sys.exception())


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
    require(shutil.disk_usage(output.parent).free >= plan['bounds']['floor_bytes']
            + plan['bounds']['scratch_bytes'], 'Initial private-scratch reserve above disk floor')
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
        case('proc-macro-refused', PROC_MACRO, 'Sandboxed Rust compiler refused')
        case('external-macro-refused', VALID + '\nextern crate project_macro;\n', 'Sandboxed Rust compiler refused')
        unknown = VALID.replace('tick.log(tuning::MESSAGE, incant_module::LogLevel::Info)', 'unsafe { forbidden(); } Ok(())')
        unknown += '\n#[link(wasm_import_module="wasi_snapshot_preview1")]\nunsafe extern "C" { fn forbidden(); }\n'
        case('normal-validator-refused', unknown, 'Normal Wasm cook validator refused')
        case('cancelled-compiler', VALID, 'Build cancelled', cancel=True)

        result['os_control'] = run_control(owner, tools, output / 'valid-relative-sdk/work', output, plan)
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
