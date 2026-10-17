"""Trusted, bounded multi-package diagnostic using the existing FreshSession.

The public single-project exporter stays fresh-only. This diagnostic shares only
proven identical units in one disposable macOS session, builds every player lib
and executable freshly, then removes every build/input copy before CPU startup.
It neither renders captures nor opens windows.
"""
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import tempfile
import time

import export_native
import native_process
from native_package import read, require, sha256, ulid_hex
from native_wasm import selection
from shipping_session import FreshSession

MAX_FILES = 4096
MAX_INPUT_BYTES = 256 * 1024**2
MAX_ALL_INPUT_BYTES = 512 * 1024**2
MAX_LOG_BYTES = 64 * 1024**2


def inventory(root, cancel=lambda: None):
    root = Path(root)
    require(root.is_dir() and not root.is_symlink(), 'Missing or linked authoring directory')
    files = {}
    total = 0
    for path in sorted(root.rglob('*')):
        cancel()
        info = path.lstat()
        require(not path.is_symlink() and not getattr(path, 'is_junction', lambda: False)(),
                'Linked authoring input')
        if stat.S_ISDIR(info.st_mode):
            continue
        require(stat.S_ISREG(info.st_mode) and len(files) < MAX_FILES, 'Invalid authoring file/count')
        value = read(path, MAX_INPUT_BYTES - total)
        total += len(value)
        require(total <= MAX_INPUT_BYTES, 'Authoring input byte limit')
        files[path.relative_to(root).as_posix()] = {'bytes': len(value), 'sha256': sha256(value)}
    require(files, 'Empty authoring input')
    return files


def jobs_from_file(path):
    jobs = json.loads(read(path, 16 * 1024))
    require(isinstance(jobs, list) and 1 <= len(jobs) <= 4, 'Expected one to four package jobs')
    names = set()
    result = []
    total = 0
    for job in jobs:
        require(isinstance(job, dict) and {'name', 'project'} <= job.keys() and
                not job.keys() - {'name', 'project', 'scene', 'camera'}, 'Unknown package job fields')
        name = job['name']
        require(isinstance(name, str) and re.fullmatch(r'[A-Za-z][A-Za-z0-9_-]{0,31}', name)
                and name not in names, 'Invalid or duplicate job name')
        names.add(name)
        require(isinstance(job['project'], str), 'Expected an absolute project path')
        project = Path(job['project'])
        require(project.is_absolute() and '..' not in project.parts, 'Expected an absolute project path')
        for key in ('scene', 'camera'):
            if job.get(key) is not None:
                ulid_hex(job[key])
        require(selection(project, job.get('scene')) is None,
                'Selected-frame package session supports bytecode scenes only')
        document = json.loads(read(project, 64 * 1024**2))
        scene = job.get('scene') or next(iter(document['scenes']))
        ulid_hex(scene)
        files = inventory(project.parent)
        require(project.name in files, 'Project is absent from authoring inventory')
        total += sum(row['bytes'] for row in files.values())
        require(total <= MAX_ALL_INPUT_BYTES, 'Combined authoring input byte limit')
        result.append(dict(job, project=project, scene=scene, inventory=files))
    return result


def create_json(path, value):
    with Path(path).open('x', encoding='utf-8') as stream:
        json.dump(value, stream, indent=2, ensure_ascii=False)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def copy_authoring(job, destination, cancel):
    require(inventory(job['project'].parent, cancel) == job['inventory'], 'Authoring changed before copy')
    destination.mkdir(parents=True)
    for relative, row in job['inventory'].items():
        cancel()
        value = read(job['project'].parent / relative, row['bytes'])
        require(len(value) == row['bytes'] and sha256(value) == row['sha256'], 'Authoring changed while copying')
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as stream:
            stream.write(value)
    require(inventory(destination, cancel) == job['inventory'], 'Copied authoring differs')
    return destination / job['project'].name


def managed_compile(logs, cancel):
    def execute(command, *, cwd, env, capture_output, text, encoding):
        require(capture_output and text and encoding == 'utf-8', 'Unexpected shared build transport')
        def bounded():
            cancel()
            require(shutil.disk_usage(logs).free >= 6 * 1024**3, 'Compilation free-space floor')
            require(all(path.stat().st_size <= MAX_LOG_BYTES for path in logs.glob('cargo.*')),
                    'Compilation log byte limit')
        with (logs / 'cargo.stdout.jsonl').open('xb') as stdout, (logs / 'cargo.stderr.log').open('xb') as stderr:
            result = native_process.run(command, cwd=cwd, env=env, stdout=stdout,
                                        stderr=stderr, timeout=2400, cancel=bounded)
        bounded()
        values = []
        for name in ('cargo.stdout.jsonl', 'cargo.stderr.log'):
            values.append(read(logs / name, MAX_LOG_BYTES).decode('utf-8'))
        return subprocess.CompletedProcess(command, result.returncode, *values)
    return execute


def compiler():
    rustc = Path(export_native.execute(['rustup', 'which', 'rustc']).stdout.strip())
    version = export_native.execute([rustc, '-vV']).stdout
    host = re.search(r'^host: (.+)$', version, re.MULTILINE)
    require(host and host[1].endswith('-apple-darwin'), 'Package session currently verifies macOS only')
    return rustc, host[1]


def startup(player, report, directory, session, cancel):
    session.assert_ready()
    cancel()
    pcm = directory / 'startup.pcm'
    require(not pcm.exists(), 'Startup PCM must not preexist')
    env = {'PATH': str(player.parent), 'TMPDIR': str(directory),
           'TMP': str(directory), 'TEMP': str(directory)}
    # This is the ordinary offline CPU package path, never --window or captures.
    result = native_process.run([str(player), '--package', str(player.parent / 'data'),
                                '--ticks', '1', '--output', str(pcm),
                                '--ui-viewport', '640x480'], cwd=player.parent, env=env,
                                timeout=60, cancel=cancel)
    require(result.returncode == 0, 'Packaged player CPU startup failed: ' + result.stderr[-4000:])
    value = json.loads(result.stdout)
    require(value.get('tick') == 1 and value.get('render_input', {}).get('gpu_submitted') is False,
            'Expected one CPU spatial startup tick without GPU submission')
    report.update(startup=value, build_storage_removed_before_startup=True,
                  startup_backend='CPU render input; no window or GPU device',
                  startup_ui_viewport='640x480')
    pcm.unlink()
    return report


def prepare(jobs, output, *, seconds=3600):
    require(type(seconds) is int and 60 <= seconds <= 7200, 'Session deadline must be 60..7200 seconds')
    output = Path(output).absolute()
    require(not output.exists() and not output.is_symlink() and output.parent.is_dir(),
            'Choose a new output directory with an existing parent')
    require(all(not output.is_relative_to(job['project'].parent) for job in jobs),
            'Diagnostic output must be outside authoring inputs')
    started = time.monotonic()

    def cancel():
        require(time.monotonic() - started < seconds, 'Cooperative multi-package session deadline')

    output.mkdir()
    create_json(output / 'partial.json', {'complete': False, 'jobs': [j['name'] for j in jobs]})
    prepared = []
    try:
        rustc, host = compiler()
        with tempfile.TemporaryDirectory(prefix='incant-package-session-') as temporary:
            scratch = Path(temporary).resolve()
            storage = scratch / 'build'
            session = FreshSession(export_native.ROOT, storage / 'target', rustc, host, 'packed')
            for index, job in enumerate(jobs):
                cancel()
                if index:
                    session.begin_next_player()
                copied = scratch / 'authoring' / job['name']
                project = copy_authoring(job, copied, cancel)
                session.input_directory(copied)
                staging = output / job['name']
                staging.mkdir()
                create_json(staging / 'authoring-inputs.json', {
                    'original_project': str(job['project']), 'files': job['inventory']})
                player, report = export_native.build(project, staging, storage,
                    job.get('scene'), job.get('camera'), 'window', session=session,
                    session_execute=managed_compile(staging, cancel))
                require(inventory(copied, cancel) == job['inventory'], 'Authoring changed during build')
                require(report['project_sha256'] == job['inventory'][job['project'].name]['sha256']
                        and report['scene_id'] == job['scene']
                        and report['startup_source_parsing'] is False, 'Wrong scene-bound package producer')
                if job.get('camera'):
                    require(report['camera'] == ulid_hex(job['camera']), 'Wrong package camera')
                require(report['shipping_session_build'] == index, 'Wrong package session build')
                report['project_build_sha256'] = sha256(read(staging / 'project-build.json', 4 * 1024**2))
                session.packaged_player(player)
                create_json(staging / 'build-receipt.json', report)
                prepared.append((job, player, report, staging))
            for path in session.inputs:
                shutil.rmtree(path)
            session.seal()
            create_json(output / 'session.json', session.report())
            # Ledger-owned logs survive deletion of all build storage. Per-job
            # cargo logs also remain exact and unmodified beside each package.
            logs = output / 'session-logs'
            logs.mkdir()
            for path in storage.glob('build-*'):
                require(path.is_file(), 'Unexpected session log')
                shutil.copy2(path, logs / path.name)
        require(not scratch.exists(), 'Private build storage remains before startup')
        for job, player, report, staging in prepared:
            require(inventory(job['project'].parent, cancel) == job['inventory'], 'Original authoring changed')
            report = startup(player, report, staging, session, cancel)
            report['diagnostic_scope'] = 'One ephemeral multi-package session; shared unit origins recorded once; fresh scene-bound player; no GPU/window/performance claim'
            create_json(staging / 'export.json', report)
        result = {'complete': True, 'source': session.head.decode(), 'target': host,
                  'jobs': [{'name': j['name'], 'project_sha256': r['project_sha256'],
                            'scene_id': r['scene_id'], 'program': r['program'],
                            'project_build_sha256': r['project_build_sha256'],
                            'player_sha256': r['player_sha256'], 'startup': r['startup']}
                           for j, _, r, _ in prepared],
                  'build_storage_removed_before_any_player': True,
                  'GPU_window_capture_executions': 0}
        create_json(output / 'manifest.json', result)
        return result
    except BaseException as error:
        create_json(output / 'failure.json', {'complete': False, 'error': repr(error),
                                             'built_jobs': [j['name'] for j, _, _, _ in prepared]})
        raise
