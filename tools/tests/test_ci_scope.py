import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('ci_scope', ROOT / 'tools/ci_scope.py')
scope = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scope)


class ScopeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'CI fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        for name in scope.DOCUMENTS:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('Existing documentation.\n')
        (self.root / 'source.rs').write_text('fn main() {}\n')
        self.base = self.commit()

    def git(self, *args):
        return subprocess.check_output(['git', '-c', 'core.hooksPath=/dev/null', *args], cwd=self.root, stderr=subprocess.PIPE).decode().strip()

    def commit(self):
        self.git('add', '-A')
        self.git('commit', '-qm', 'fixture')
        return self.git('rev-parse', 'HEAD')

    def event(self, head):
        return {'pull_request': {'base': {'sha': self.base}, 'head': {'sha': head}}}

    def select(self, head):
        return scope.select('pull_request', self.event(head), self.root)

    def test_plan_and_generated_conventions_take_docs_path(self):
        for name in scope.DOCUMENTS:
            (self.root / name).write_text('Revision 8 local-first rules.\n')
        result = self.select(self.commit())
        self.assertEqual(result['scope'], 'docs')
        self.assertEqual(set(result['changed']), scope.DOCUMENTS)
        scope.validate_documents(result['base'], result['head'], self.root)

    def test_recorded_plan_and_source_commits(self):
        fixture = json.loads((ROOT / 'tools/tests/fixtures/docs-scope-history.json').read_text())
        for case in fixture['cases']:
            event = {'pull_request': {'base': {'sha': case['base']}, 'head': {'sha': case['head']}}}
            with self.subTest(commit=case['head']), patch.object(scope, 'git', side_effect=[case['base'].encode(), case['raw_diff'].encode()]):
                self.assertEqual(scope.select('pull_request', event)['scope'], case['expected'])

    def test_unreviewed_adr_is_not_exempt(self):
        path = self.root / 'docs/adr/9999-unknown.md'
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('Unreviewed future input.\n')
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_code_plus_docs_requires_full(self):
        (self.root / 'PLAN.md').write_text('Updated plan.\n')
        (self.root / 'source.rs').write_text('fn main() { panic!(); }\n')
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_unknown_markdown_is_not_exempt(self):
        (self.root / 'README.md').write_text('Possibly an include_str input.\n')
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_spike_fixture_is_not_exempt(self):
        path = self.root / 'docs/spikes/fixture.md'
        path.parent.mkdir(parents=True)
        path.write_text('Possible build or test input.\n')
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_deletion_requires_full(self):
        (self.root / 'PLAN.md').unlink()
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_rename_requires_full(self):
        (self.root / 'PLAN.md').rename(self.root / 'new-plan.md')
        self.assertEqual(self.select(self.commit())['scope'], 'full')

    def test_executable_mode_requires_full(self):
        self.git('update-index', '--chmod=+x', 'PLAN.md')
        self.git('commit', '-qm', 'mode')
        self.assertEqual(self.select(self.git('rev-parse', 'HEAD'))['scope'], 'full')

    @unittest.skipIf(os.name == 'nt', 'symlink creation requires extra Windows privilege')
    def test_symlink_requires_full_and_validation_refuses_it(self):
        (self.root / 'PLAN.md').unlink()
        (self.root / 'PLAN.md').symlink_to('source.rs')
        head = self.commit()
        self.assertEqual(self.select(head)['scope'], 'full')
        with self.assertRaises(ValueError):
            scope.validate_documents(self.base, head, self.root)

    def test_empty_diff_missing_commit_and_dispatch_require_full(self):
        for name, event in [('pull_request', self.event(self.base)),
                            ('pull_request', self.event('a' * 40)),
                            ('push', {'before': '0' * 40, 'after': self.base}),
                            ('workflow_dispatch', {})]:
            with self.subTest(event=name, data=event):
                self.assertEqual(scope.select(name, event, self.root)['scope'], 'full')

    def test_push_docs_and_force_push(self):
        (self.root / 'STATUS.md').write_text('Current state.\n')
        head = self.commit()
        self.assertEqual(scope.select('push', {'before': self.base, 'after': head}, self.root)['scope'], 'docs')
        self.assertEqual(scope.select('push', {'before': head, 'after': self.base}, self.root)['scope'], 'full')

    def test_missing_merge_base_in_shallow_history_requires_full(self):
        # The real Git failure path also covers unavailable shallow-history objects.
        with patch.object(scope, 'git', side_effect=subprocess.CalledProcessError(1, 'git')):
            self.assertEqual(self.select(self.base)['scope'], 'full')

    def test_malformed_diff_and_unavailable_git_require_full(self):
        for malformed in [b':100644 100644 bad bad M\0PLAN.md\0', b'broken', b'\xff\0']:
            with self.subTest(raw=malformed), patch.object(scope, 'git', side_effect=[self.base.encode(), malformed]):
                self.assertEqual(self.select(self.base)['scope'], 'full')
        with patch.object(scope, 'git', side_effect=FileNotFoundError()):
            self.assertEqual(self.select(self.base)['scope'], 'full')

    def test_bad_revision_cannot_inject_git_options_or_outputs(self):
        for bad in ['--output=x', 'a' * 40 + '\nscope=docs', None, [], 42]:
            with self.subTest(value=bad):
                self.assertEqual(self.select(bad)['scope'], 'full')

    def test_document_validation_refuses_missing_empty_nontext_and_whitespace(self):
        for content in ['', '\0', 'bad trailing space \n']:
            with self.subTest(content=content):
                (self.root / 'PLAN.md').write_text(content)
                head = self.commit()
                with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                    scope.validate_documents(self.base, head, self.root)
        (self.root / 'PLAN.md').write_bytes(b'\xff')
        with self.assertRaises(UnicodeError):
            scope.validate_documents(self.base, self.commit(), self.root)
        (self.root / 'PLAN.md').unlink()
        with self.assertRaises(ValueError):
            scope.validate_documents(self.base, self.commit(), self.root)

    def test_cli_bad_event_writes_only_full_fixed_outputs(self):
        event = self.root / 'event.json'
        output = self.root / 'outputs'
        event.write_text('{broken')
        env = dict(os.environ, GITHUB_EVENT_PATH=str(event), GITHUB_EVENT_NAME='pull_request', GITHUB_OUTPUT=str(output))
        result = subprocess.run([os.sys.executable, str(ROOT / 'tools/ci_scope.py'), 'select'], env=env, text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(result.stdout)['scope'], 'full')
        self.assertEqual(output.read_text(), 'scope=full\nbase=\nhead=\n')

    def test_generated_convention_drift_fails_unchanged_validator(self):
        destination = self.root / 'tools/generate_conventions.py'
        destination.parent.mkdir()
        shutil.copyfile(ROOT / 'tools/generate_conventions.py', destination)
        subprocess.run([os.sys.executable, str(destination)], check=True, capture_output=True)
        subprocess.run([os.sys.executable, str(destination), '--check'], check=True, capture_output=True)
        (self.root / 'AGENTS.md').write_text('Stale generated content.\n')
        result = subprocess.run([os.sys.executable, str(destination), '--check'], capture_output=True)
        self.assertNotEqual(result.returncode, 0)


class WorkflowContractTests(unittest.TestCase):
    def test_failed_or_missing_classification_keeps_full_job_and_commands(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        checks = workflow.split('  checks:\n', 1)[1]
        self.assertIn('needs: classify\n', checks)
        self.assertIn('if: ${{ !cancelled() }}', checks)
        self.assertIn("needs.classify.result == 'success' && needs.classify.outputs.scope == 'docs' && 'ubuntu-latest' || 'macos-latest'", checks)
        self.assertIn("needs.classify.result == 'success' && needs.classify.outputs.scope == 'docs' && '0' || '1'", checks)
        blocks = re.split(r'(?m)^      - ', checks.split('    steps:\n', 1)[1])[1:]
        allowed = ['uses: actions/checkout@', 'uses: actions/setup-python@',
                   'run: python3 tools/generate_conventions.py --check',
                   'name: Validate classified documentation inputs',
                   'name: Verify documentation selection failure cases']
        for block in blocks:
            if not any(block.startswith(prefix) for prefix in allowed):
                with self.subTest(step=block.splitlines()[0]):
                    self.assertIn("(needs.classify.result != 'success' || needs.classify.outputs.scope != 'docs')", block)
        convention = next(block for block in blocks if block.startswith('run: python3 tools/generate_conventions.py --check'))
        self.assertNotIn('if:', convention)
        diagnostics = next(block for block in blocks if block.startswith('name: Preserve Cargo unit timing diagnostics'))
        self.assertIn("if: always() && (needs.classify.result != 'success' || needs.classify.outputs.scope != 'docs')", diagnostics)

    def test_failed_classifier_with_retained_docs_output_selects_every_full_step(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        checks = workflow.split('  checks:\n', 1)[1]
        runner = next(line.split('${{', 1)[1].split('}}', 1)[0].strip()
                      for line in checks.splitlines() if 'runs-on:' in line)
        conditions = [line.strip()[4:] for line in checks.splitlines()
                      if line.strip().startswith('if: ') and 'needs.classify.' in line]
        self.assertTrue(conditions)

        def evaluate(expression, result, output):
            # Evaluate this trusted repository workflow's boolean/string expressions,
            # using synthetic job outcomes; no event or project text becomes code.
            expression = expression.removeprefix('${{').removesuffix('}}').strip()
            expression = expression.replace('!cancelled()', 'True')
            expression = expression.replace('needs.classify.result', 'result')
            expression = expression.replace('needs.classify.outputs.scope', 'output')
            expression = expression.replace('always()', 'True')
            expression = expression.replace('&&', ' and ').replace('||', ' or ')
            return eval(expression, {'__builtins__': {}}, {'result': result, 'output': output})

        for result in ('success', 'failure', 'skipped', 'cancelled'):
            for output in ('docs', 'full', ''):
                with self.subTest(result=result, output=output):
                    docs = result == 'success' and output == 'docs'
                    self.assertEqual(evaluate(runner, result, output),
                                     'ubuntu-latest' if docs else 'macos-latest')
                    for condition in conditions:
                        is_docs_step = "outputs.scope == 'docs'" in condition
                        self.assertEqual(bool(evaluate(condition, result, output)),
                                         docs if is_docs_step else not docs, condition)

    def test_new_literal_document_readers_require_policy_review(self):
        # Guard common include_str/read/build-input mistakes in later source edits.
        # Dynamic reads still require review; this is not a general program analysis.
        allowed = {'tools/ci_scope.py', 'tools/tests/test_ci_scope.py',
                   'tools/generate_conventions.py', 'tools/handoff/main.py'}
        names = '|'.join(re.escape(Path(name).name) for name in scope.DOCUMENTS)
        literal = re.compile(r'''["'][^"'\n]*(?:\b(?:''' + names + r''')\b|docs/conventions)[^"'\n]*["']''')
        tracked = subprocess.check_output(['git', 'ls-files', '-z', '--', 'crates', 'editor', 'tools'], cwd=ROOT).split(b'\0')
        readers = []
        for raw in tracked:
            if not raw:
                continue
            name = raw.decode()
            path = ROOT / name
            if name in allowed or path.suffix not in {'.rs', '.c', '.h', '.py', '.js', '.mjs', '.ts', '.tsx', '.toml'}:
                continue
            if literal.search(path.read_text(encoding='utf-8')):
                readers.append(name)
        self.assertEqual(readers, [], 'New documentation readers need an audited policy/validator update')


if __name__ == '__main__':
    unittest.main()
