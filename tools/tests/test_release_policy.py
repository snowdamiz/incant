"""Keep development and frozen-checkpoint event boundaries explicit."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOWS = ROOT / '.github/workflows'
FULL = ('ci.yml', 'desktop.yml', 'credentials.yml', 'development-wasm.yml',
        'physics-candidates.yml', 'nightly.yml', 'website.yml', 'local-services.yml')


class ReleasePolicyTests(unittest.TestCase):
    def test_full_suites_cover_all_candidate_changes_without_duplicate_pr_or_main_runs(self):
        for name in FULL:
            with self.subTest(workflow=name):
                text = (WORKFLOWS / name).read_text()
                events = text.split('on:\n', 1)[1].split('permissions:', 1)[0]
                self.assertIn('  push:\n    branches: [codex/release-candidate]\n', events)
                self.assertIn('  workflow_dispatch:', events)
                for forbidden in ('pull_request', '[main]', 'paths:', 'paths-ignore:', 'schedule:'):
                    self.assertNotIn(forbidden, events)
        self.assertIn('python3 tools/ci_scope.py select --full', (WORKFLOWS / 'ci.yml').read_text())

    def test_candidate_push_can_refresh_pinned_platform_tool_caches(self):
        text = (WORKFLOWS / 'nightly.yml').read_text()
        self.assertEqual(text.count("if: github.event_name == 'push' && steps.tool-cache.outputs.cache-hit != 'true'"), 2)
        self.assertNotIn('github.event.repository.default_branch', text)

    def test_main_fast_gate_has_no_rust_compilation_or_path_skip(self):
        text = (WORKFLOWS / 'development.yml').read_text()
        self.assertIn('  push:\n    branches: [main]', text)
        self.assertIn('  pull_request:\n    branches: [main]', text)
        self.assertIn('  development-checks:', text)
        self.assertNotIn('paths:', text)
        for command in ('npm run test:tools', 'cargo fmt --all --check',
                        'node tools/generate_sdk.mjs --check',
                        'npm run build --workspace editor/ui', 'npm run test --workspace editor/ui'):
            self.assertIn(command, text)
        for command in ('cargo build', 'cargo test', 'cargo clippy', 'tools/shipping'):
            self.assertNotIn(command, text)

    def test_website_deployment_remains_main_only(self):
        caller = (WORKFLOWS / 'website-publish.yml').read_text()
        self.assertIn('branches: [main]', caller)
        self.assertIn("- 'website/**'", caller)
        self.assertIn('uses: ./.github/workflows/website.yml', caller)
        self.assertIn('pages: write', caller)
        self.assertIn('id-token: write', caller)
        workflow = (WORKFLOWS / 'website.yml').read_text()
        self.assertEqual(workflow.count("if: github.ref == 'refs/heads/main' && github.event_name != 'pull_request'"), 2)
        self.assertIn('  workflow_call:', workflow)

    def test_full_context_contract_is_unique_and_excludes_development_and_deploy(self):
        contexts = json.loads((ROOT / '.github/release-checks.json').read_text())['required_contexts']
        self.assertEqual(len(contexts), len(set(contexts)))
        self.assertEqual(len(contexts), 23)
        for required in ('checks', 'classify', 'build', 'compare', 'android', 'ios', 'web', 'portability', 'local-service-protocols'):
            self.assertIn(required, contexts)
        self.assertIn('  local-service-protocols:', (WORKFLOWS / 'local-services.yml').read_text())
        for os in ('macos-latest', 'ubuntu-latest', 'windows-latest'):
            for job in ('storage', 'native', 'conformance'):
                self.assertIn(f'{job} ({os})', contexts)
        self.assertNotIn('deploy', contexts)
        self.assertNotIn('development-checks', contexts)


if __name__ == '__main__':
    unittest.main()
