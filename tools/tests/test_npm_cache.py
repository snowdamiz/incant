import hashlib
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
ACTION = ROOT / '.github/actions/npm-cache'


@unittest.skipIf(os.name == 'nt', 'The npm cache action runs only on Linux and macOS')
class NpmMetadataTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        # Exercise the real metadata script without touching the user's npm
        # configuration or network. Unexpected command arguments fail the test.
        for name, contents in {
            'npm': '#!/bin/sh\n[ "$*" = "config get cache" ] || exit 2\nprintf "%s" "$TEST_NPM_PATH"\n',
            'node': '#!/bin/sh\n[ "$1" = "-p" ] && [ "$2" = \'require("node:os").arch()\' ] || exit 2\nprintf "%s" "$TEST_NODE_ARCH"\n',
        }.items():
            path = self.bin / name
            path.write_text(contents)
            path.chmod(0o700)
        self.lock = self.root / 'package-lock.json'
        self.lock.write_bytes(b'{"lockfileVersion":3}\n')
        self.lock_hash = hashlib.sha256(hashlib.sha256(self.lock.read_bytes()).digest()).hexdigest()
        self.output = self.root / 'output'

    def invoke(self, **overrides):
        env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ['PATH'],
                   NPM_LOCK_FILE=str(self.lock), NPM_LOCK_HASH=self.lock_hash,
                   GITHUB_OUTPUT=str(self.output), RUNNER_OS='macOS',
                   NPM_CACHE_REF='refs/heads/main', NPM_CACHE_DEFAULT_BRANCH='main',
                   TEST_NODE_ARCH='arm64', TEST_NPM_PATH='/fixture with spaces/.npm')
        env.update(overrides)
        result = subprocess.run(['bash', str(ACTION / 'metadata.sh')], cwd=self.root,
                                env=env, capture_output=True, text=True)
        return result, self.output.read_text() if self.output.exists() else ''

    def test_existing_setup_node_namespace_uses_node_arch_and_configured_path(self):
        result, output = self.invoke()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(output, f'path=/fixture with spaces/.npm\nkey=node-cache-macOS-arm64-npm-{self.lock_hash}\nsave=true\n')
        self.output.unlink()
        result, output = self.invoke(RUNNER_OS='Linux', TEST_NODE_ARCH='x64')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f'key=node-cache-Linux-x64-npm-{self.lock_hash}\n', output)

    def test_nested_website_lock_has_independent_identity(self):
        nested = self.root / 'website/package-lock.json'
        nested.parent.mkdir()
        nested.write_bytes(b'{"lockfileVersion":3,"name":"website"}\n')
        digest = hashlib.sha256(hashlib.sha256(nested.read_bytes()).digest()).hexdigest()
        result, output = self.invoke(NPM_LOCK_FILE='website/package-lock.json', NPM_LOCK_HASH=digest)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f'-npm-{digest}\n', output)
        self.assertNotIn(self.lock_hash, output)

    def test_missing_lock_bad_hash_and_output_injection_fail_before_writing(self):
        cases = [dict(NPM_LOCK_FILE='missing.json'), dict(NPM_LOCK_HASH=''),
                 dict(NPM_LOCK_HASH='x' * 64), dict(NPM_LOCK_HASH=self.lock_hash + '\nkey=injected'),
                 dict(TEST_NPM_PATH=''), dict(TEST_NPM_PATH='/cache\nkey=injected'),
                 dict(TEST_NODE_ARCH='arm64\rkey=injected'), dict(RUNNER_OS='Linux\npath=/other')]
        for case in cases:
            with self.subTest(case=case):
                result, output = self.invoke(**case)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(output, '')

    def test_only_case_exact_default_and_frozen_candidate_refs_may_save(self):
        cases = [('refs/heads/main', 'main', True), ('refs/heads/trunk', 'trunk', True),
                 ('refs/heads/main', 'trunk', False), ('refs/heads/feature', 'main', False),
                 ('refs/heads/MAIN', 'main', False), ('refs/heads/main', 'MAIN', False),
                 ('refs/heads/codex/release-candidate', 'main', True),
                 ('refs/heads/codex/Release-Candidate', 'main', False),
                 ('refs/heads/codex/release-candidate-extra', 'main', False),
                 ('refs/pull/83/merge', 'main', False), ('refs/pull/83/head', 'main', False),
                 ('refs/tags/main', 'main', False), ('', 'main', False),
                 ('refs/heads/main', '', False), ('refs/heads/', '', False)]
        for ref, default, may_save in cases:
            with self.subTest(ref=ref, default=default):
                self.output.unlink(missing_ok=True)
                result, output = self.invoke(NPM_CACHE_REF=ref, NPM_CACHE_DEFAULT_BRANCH=default)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output.splitlines()[-1], 'save=' + str(may_save).lower())


class NpmCachePolicyTests(unittest.TestCase):
    def test_actual_action_conditions_use_only_the_exact_metadata_decision(self):
        text = (ACTION / 'action.yml').read_text()
        blocks = re.split(r'(?m)^    - ', text)[1:]
        cache_steps = [block for block in blocks if 'uses: actions/cache' in block]
        self.assertEqual(len(cache_steps), 2)
        for value in ('true', 'false', ''):
            with self.subTest(metadata=value):
                selected = []
                for block in cache_steps:
                    condition = next(line.strip()[4:] for line in block.splitlines()
                                     if line.strip().startswith('if: '))
                    # The shell emits only fixed true/false, never a branch name.
                    condition = condition.replace('steps.metadata.outputs.save', 'value')
                    if eval(condition, {'__builtins__': {}}, {'value': value}):
                        selected.append(next(line.strip()[6:].split()[0] for line in block.splitlines()
                                             if line.strip().startswith('uses: ')))
                suffix = '' if value == 'true' else '/restore'
                self.assertEqual(selected, [f'actions/cache{suffix}@55cc8345863c7cc4c66a329aec7e433d2d1c52a9'])
        self.assertNotIn('restore-keys:', text)
        self.assertNotIn('save-always:', text)

    def test_all_callers_use_local_action_and_preserve_windows_exclusion(self):
        expected = {'ci.yml', 'desktop.yml', 'nightly.yml', 'physics-candidates.yml', 'website.yml'}
        callers = set()
        for path in (ROOT / '.github/workflows').glob('*.yml'):
            text = path.read_text()
            self.assertNotRegex(text, r'\bcache:\s*(?:npm|[^\n]*[\'\"]npm[\'\"])', path.name)
            if 'uses: ./.github/actions/npm-cache' in text:
                callers.add(path.name)
                if path.name != 'ci.yml':
                    self.assertIn("'.github/actions/npm-cache/**'", text)
        self.assertEqual(callers, expected)
        desktop = (ROOT / '.github/workflows/desktop.yml').read_text()
        self.assertIn("uses: ./.github/actions/npm-cache\n        if: runner.os != 'Windows'", desktop)
        website = (ROOT / '.github/workflows/website.yml').read_text()
        self.assertIn('package-manager-cache: false', website)
        self.assertIn('with: {lock-file: website/package-lock.json}', website)


if __name__ == '__main__':
    unittest.main()
