"""Portable boundary controls; no Rust build, player, GPU or window runs."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import native_export_session as tool


class PackageSessionControls(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def jobs(self, count=2):
        jobs = []
        for index in range(count):
            directory = self.root / f'authoring-{index}'
            directory.mkdir()
            project = directory / 'project.incant'
            project.write_text(json.dumps({'scenes': {'00000000000000000000000001': {}}}))
            (directory / 'script.ts').write_text(f'// locale {index}')
            jobs.append({'name': f'locale-{index}', 'project': str(project)})
        request = self.root / 'jobs.json'
        request.write_text(json.dumps(jobs))
        return request, jobs

    def test_exact_jobs_bind_all_authored_bytes(self):
        request, _ = self.jobs()
        jobs = tool.jobs_from_file(request)
        self.assertEqual(len(jobs), 2)
        self.assertEqual(set(jobs[0]['inventory']), {'project.incant', 'script.ts'})
        destination = self.root / 'copied'
        project = tool.copy_authoring(jobs[0], destination, lambda: None)
        self.assertEqual(project.name, 'project.incant')
        self.assertEqual(tool.inventory(destination), jobs[0]['inventory'])

    def test_arbitrary_build_options_and_duplicate_names_refuse(self):
        request, jobs = self.jobs()
        for field in ('command', 'shaders', 'features', 'mode'):
            with self.subTest(field=field):
                changed = dict(jobs[0], **{field: 'untrusted'})
                request.write_text(json.dumps([changed]))
                with self.assertRaisesRegex(ValueError, 'Unknown'):
                    tool.jobs_from_file(request)
        request.write_text(json.dumps([jobs[0], jobs[0]]))
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            tool.jobs_from_file(request)
        request.write_text(json.dumps(jobs * 3))
        with self.assertRaisesRegex(ValueError, 'one to four'):
            tool.jobs_from_file(request)

    def test_static_module_refuses_before_any_build(self):
        request, jobs = self.jobs(1)
        project = Path(jobs[0]['project'])
        project.write_text(json.dumps({'scenes': {'00000000000000000000000001': {
            'program': {'kind': 'wasm', 'module': 'asset'}, 'entities': {}}},
            'assets': {'asset': {'kind': 'wasm', 'sha256': 'a' * 64}}}))
        with self.assertRaisesRegex(ValueError, 'bytecode scenes only'):
            tool.jobs_from_file(request)

    def test_linked_inputs_and_changed_authoring_refuse_before_copy(self):
        request, _ = self.jobs(1)
        job, = tool.jobs_from_file(request)
        (job['project'].parent / 'script.ts').write_text('changed after inventory')
        destination = self.root / 'not-created'
        with self.assertRaisesRegex(ValueError, 'changed before copy'):
            tool.copy_authoring(job, destination, lambda: None)
        self.assertFalse(destination.exists())
        (job['project'].parent / 'link').symlink_to(request)
        with self.assertRaisesRegex(ValueError, 'Linked'):
            tool.jobs_from_file(request)

    def test_input_count_and_total_bytes_are_bounded(self):
        request, _ = self.jobs(1)
        with patch.object(tool, 'MAX_FILES', 1), self.assertRaisesRegex(ValueError, 'count'):
            tool.jobs_from_file(request)
        with patch.object(tool, 'MAX_ALL_INPUT_BYTES', 1), self.assertRaisesRegex(ValueError, 'Combined'):
            tool.jobs_from_file(request)

    def fake_session(self, events):
        class Session:
            def __init__(self, root, path, rustc, host, split):
                self.path = path
                path.mkdir(parents=True)
                self.inputs, self.players, self.builds = [], [], []
                self.head = b'synthetic-boundary-control'
                self.deleted = False

            def input_directory(self, path):
                self.inputs.append(path)

            def packaged_player(self, path):
                self.players.append(path)

            def begin_next_player(self):
                if len(self.players) != len(self.builds):
                    raise AssertionError('Prior player not packaged')
                events.append('retire-packaged-player')

            def seal(self):
                if any(path.exists() for path in self.inputs):
                    raise AssertionError('Input deletion barrier not reached')
                shutil.rmtree(self.path)
                self.deleted = True

            def assert_ready(self):
                if not self.deleted or self.path.exists() or any(p.exists() for p in self.inputs):
                    raise AssertionError('Build/input deletion barrier not reached')

            def report(self):
                self.assert_ready()
                return {'synthetic_boundary_control': True}
        return Session

    def build_stub(self, events, mutation=None):
        def build(project, staging, storage, scene, camera, mode, *, session, session_execute):
            self.assertEqual(mode, 'window')
            self.assertTrue(callable(session_execute))
            self.assertTrue(project.exists())
            events.append('build-' + staging.name)
            session.builds.append(staging.name)
            player = staging / 'player'
            player.write_bytes(('scene-bound-' + staging.name).encode())
            report = {'project_sha256': tool.sha256(project.read_bytes()),
                      'startup_source_parsing': False, 'shipping_session_build': len(session.builds) - 1,
                      'scene_id': scene, 'program': {'synthetic': staging.name},
                      'player_sha256': tool.sha256(player.read_bytes())}
            (staging / 'project-build.json').write_text(json.dumps({
                'synthetic_project_receipt': staging.name}))
            if mutation:
                mutation(project, report)
            return player, report
        return build

    def test_all_distinct_packages_build_and_delete_storage_before_any_startup(self):
        request, _ = self.jobs()
        events = []

        def start(player, report, directory, session, cancel):
            session.assert_ready()
            self.assertFalse(session.path.parent.parent.exists())
            self.assertEqual(len(session.builds), 2)
            events.append('startup-' + directory.name)
            report['startup'] = {'tick': 1, 'render_input': {'gpu_submitted': False}}
            return report

        with patch.object(tool, 'compiler', return_value=(Path('/pinned/rustc'), 'aarch64-apple-darwin')), \
                patch.object(tool, 'FreshSession', self.fake_session(events)), \
                patch.object(tool.export_native, 'build', side_effect=self.build_stub(events)), \
                patch.object(tool, 'startup', side_effect=start):
            result = tool.prepare(tool.jobs_from_file(request), self.root / 'output')
        self.assertEqual(events, ['build-locale-0', 'retire-packaged-player', 'build-locale-1',
                                  'startup-locale-0', 'startup-locale-1'])
        self.assertTrue(result['complete'])
        self.assertEqual(result['GPU_window_capture_executions'], 0)
        self.assertNotEqual(result['jobs'][0]['player_sha256'], result['jobs'][1]['player_sha256'])

    def test_mutated_build_input_leaves_partial_and_no_startup_or_complete_manifest(self):
        request, _ = self.jobs(1)
        events = []
        output = self.root / 'output'
        with patch.object(tool, 'compiler', return_value=(Path('/pinned/rustc'), 'aarch64-apple-darwin')), \
                patch.object(tool, 'FreshSession', self.fake_session(events)), \
                patch.object(tool.export_native, 'build', side_effect=self.build_stub(events,
                    lambda project, report: project.write_text('changed while compiling'))), \
                patch.object(tool, 'startup') as start:
            with self.assertRaisesRegex(ValueError, 'changed during build'):
                tool.prepare(tool.jobs_from_file(request), output)
            start.assert_not_called()
        self.assertTrue((output / 'partial.json').is_file())
        self.assertTrue((output / 'failure.json').is_file())
        self.assertFalse((output / 'manifest.json').exists())

    def test_existing_output_is_untouched_before_compiler_or_build(self):
        request, _ = self.jobs(1)
        output = self.root / 'existing'
        output.mkdir()
        sentinel = output / 'sentinel'
        sentinel.write_bytes(b'preserve')
        with patch.object(tool, 'compiler') as compiler:
            with self.assertRaisesRegex(ValueError, 'new output'):
                tool.prepare(tool.jobs_from_file(request), output)
            compiler.assert_not_called()
        self.assertEqual(sentinel.read_bytes(), b'preserve')
        self.assertEqual(list(output.iterdir()), [sentinel])

    def test_output_inside_project_and_invalid_deadlines_refuse_before_setup(self):
        request, _ = self.jobs(1)
        jobs = tool.jobs_from_file(request)
        output = jobs[0]['project'].parent / 'not-created'
        with patch.object(tool, 'compiler') as compiler:
            with self.assertRaisesRegex(ValueError, 'outside authoring'):
                tool.prepare(jobs, output)
            for seconds in (True, 0, 59, 7201):
                with self.subTest(seconds=seconds), self.assertRaisesRegex(ValueError, 'deadline'):
                    tool.prepare(jobs, self.root / 'other', seconds=seconds)
            compiler.assert_not_called()
        self.assertFalse(output.exists())
        self.assertFalse((self.root / 'other').exists())

    def test_wrong_project_receipt_prevents_any_startup_or_complete_manifest(self):
        request, _ = self.jobs(1)
        output = self.root / 'output'
        events = []
        with patch.object(tool, 'compiler', return_value=(Path('/pinned/rustc'), 'aarch64-apple-darwin')), \
                patch.object(tool, 'FreshSession', self.fake_session(events)), \
                patch.object(tool.export_native, 'build', side_effect=self.build_stub(events,
                    lambda project, report: report.update(project_sha256='0' * 64))), \
                patch.object(tool, 'startup') as start:
            with self.assertRaisesRegex(ValueError, 'Wrong scene-bound'):
                tool.prepare(tool.jobs_from_file(request), output)
            start.assert_not_called()
        self.assertFalse((output / 'manifest.json').exists())
        self.assertTrue((output / 'failure.json').is_file())

    def test_later_startup_failure_preserves_first_outcome_without_batch_completion(self):
        request, _ = self.jobs()
        output = self.root / 'output'
        events = []

        def start(player, report, directory, session, cancel):
            session.assert_ready()
            if directory.name == 'locale-1':
                raise ValueError('original startup refusal')
            report['startup'] = {'tick': 1, 'render_input': {'gpu_submitted': False}}
            return report

        with patch.object(tool, 'compiler', return_value=(Path('/pinned/rustc'), 'aarch64-apple-darwin')), \
                patch.object(tool, 'FreshSession', self.fake_session(events)), \
                patch.object(tool.export_native, 'build', side_effect=self.build_stub(events)), \
                patch.object(tool, 'startup', side_effect=start):
            with self.assertRaisesRegex(ValueError, 'original startup refusal'):
                tool.prepare(tool.jobs_from_file(request), output)
        self.assertTrue((output / 'locale-0/export.json').is_file())
        self.assertFalse((output / 'locale-1/export.json').exists())
        self.assertTrue((output / 'failure.json').is_file())
        self.assertFalse((output / 'manifest.json').exists())

    def test_managed_compile_preserves_failure_and_exact_raw_logs(self):
        logs = self.root / 'logs'
        logs.mkdir()
        cancel = Mock()

        def execute(command, **kwargs):
            kwargs['cancel']()
            self.assertEqual(kwargs['timeout'], 2400)
            kwargs['stdout'].write(b'{"compiler":"raw"}\n')
            kwargs['stderr'].write(b'original compiler failure\n')
            return subprocess.CompletedProcess(command, 101)

        with patch.object(tool.native_process, 'run', side_effect=execute), \
                patch.object(tool.shutil, 'disk_usage', return_value=Mock(free=7 * 1024**3)):
            result = tool.managed_compile(logs, cancel)(['cargo'], cwd=self.root, env={},
                capture_output=True, text=True, encoding='utf-8')
        self.assertEqual(result.returncode, 101)
        self.assertEqual(result.stdout, '{"compiler":"raw"}\n')
        self.assertEqual(result.stderr, 'original compiler failure\n')
        self.assertEqual((logs / 'cargo.stderr.log').read_bytes(), b'original compiler failure\n')
        self.assertGreater(cancel.call_count, 0)

    def test_running_compilation_log_limit_refuses_and_retains_original_bytes(self):
        logs = self.root / 'logs'
        logs.mkdir()

        def execute(command, **kwargs):
            kwargs['stderr'].write(b'oversized')
            kwargs['stderr'].flush()
            kwargs['cancel']()
            self.fail('Over-limit child must be cancelled')

        with patch.object(tool.native_process, 'run', side_effect=execute), \
                patch.object(tool.shutil, 'disk_usage', return_value=Mock(free=7 * 1024**3)), \
                patch.object(tool, 'MAX_LOG_BYTES', 4):
            with self.assertRaisesRegex(ValueError, 'log byte limit'):
                tool.managed_compile(logs, lambda: None)(['cargo'], cwd=self.root, env={},
                    capture_output=True, text=True, encoding='utf-8')
        self.assertEqual((logs / 'cargo.stderr.log').read_bytes(), b'oversized')

    def test_cpu_startup_never_selects_capture_window_or_gpu(self):
        directory = self.root / 'player'
        directory.mkdir()
        player = directory / 'incant_game_player'
        ready = Mock(spec=['assert_ready'])

        def execute(command, **kwargs):
            self.assertNotIn('--window', command)
            self.assertNotIn('--gpu-offscreen', command)
            self.assertNotIn('--capture-frames', command)
            self.assertEqual(command[command.index('--ticks') + 1], '1')
            self.assertEqual(kwargs['timeout'], 60)
            Path(command[command.index('--output') + 1]).write_bytes(b'PCM fixture')
            return subprocess.CompletedProcess(command, 0, json.dumps({
                'tick': 1, 'render_input': {'gpu_submitted': False}}), '')
        with patch.object(tool.native_process, 'run', side_effect=execute):
            report = tool.startup(player, {}, directory, ready, lambda: None)
        ready.assert_ready.assert_called_once()
        self.assertTrue(report['build_storage_removed_before_startup'])
        self.assertFalse((directory / 'startup.pcm').exists())


if __name__ == '__main__':
    unittest.main()
