import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import check_schemas


class PublishedSchemaTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.published = self.root / 'schemas'
        self.published.mkdir()
        (self.published / 'agent-tools.json').write_bytes(b'[{"name":"current"}]\n')
        (self.published / 'Project.schema.json').write_bytes(b'{"type":"object"}\n')
        self.before = {p.name: p.read_bytes() for p in self.published.iterdir()}
        self.marker = self.root / 'generator-output-path'

    def generate(self, files, *, exit_code=0):
        # Exercise a real child that writes only to the supplied destination.
        program = (
            'import json,pathlib,sys\n'
            'assert sys.argv[1] == "schema"\n'
            'destination=pathlib.Path(sys.argv[2])\n'
            f'pathlib.Path({str(self.marker)!r}).write_text(str(destination))\n'
            f'files={files!r}\n'
            'for name, data in files.items(): (destination/name).write_bytes(data)\n'
            f'sys.exit({exit_code})\n'
        )
        return check_schemas.check_schemas([sys.executable, '-c', program], self.published)

    def tearDown(self):
        self.assertEqual({p.name: p.read_bytes() for p in self.published.iterdir()}, self.before)
        if self.marker.exists():
            generated = Path(self.marker.read_text())
            self.assertFalse(generated.is_relative_to(self.root))
            self.assertFalse(generated.exists())

    def test_matching_generation_preserves_published_sources(self):
        self.assertEqual(self.generate(self.before), [])

    def test_stale_tool_contract_is_reported_without_replacing_it(self):
        generated = dict(self.before, **{'agent-tools.json': b'[{"name":"new"}]\n'})
        self.assertEqual(self.generate(generated), ['Stale published schema: agent-tools.json'])

    def test_semantically_equal_but_different_bytes_are_stale(self):
        generated = dict(self.before, **{'Project.schema.json': b'{ "type": "object" }\n'})
        self.assertEqual(json.loads(generated['Project.schema.json']),
                         json.loads(self.before['Project.schema.json']))
        self.assertEqual(self.generate(generated), ['Stale published schema: Project.schema.json'])

    def test_added_and_removed_schemas_both_fail(self):
        generated = {'New.schema.json': b'{}\n', 'agent-tools.json': self.before['agent-tools.json']}
        self.assertEqual(self.generate(generated), [
            'Unpublished generated schema: New.schema.json',
            'Published schema no longer generated: Project.schema.json'])

    def test_failed_partial_generator_propagates_and_cleans_private_output(self):
        with self.assertRaises(subprocess.CalledProcessError) as raised:
            self.generate({'agent-tools.json': b'partial'}, exit_code=9)
        self.assertEqual(raised.exception.returncode, 9)

    def test_cli_exits_unsuccessfully_and_names_drift(self):
        with patch.object(sys, 'argv', ['check_schemas', '--binary', sys.executable]), \
                patch.object(check_schemas, 'check_schemas', return_value=[
                    'Stale published schema: agent-tools.json']), \
                contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(check_schemas.main(), 1)
        self.assertIn('Stale published schema: agent-tools.json', output.getvalue())

    def test_ci_compares_private_generation_before_shipping(self):
        workflow = (check_schemas.ROOT / '.github/workflows/ci.yml').read_text()
        check = 'run: python3 tools/check_schemas.py --binary target/ci/incant_headless'
        self.assertEqual(workflow.count(check), 1)
        self.assertLess(workflow.index(check), workflow.index('run: python3 tools/shipping_suite.py '))
        self.assertNotIn('run: target/ci/incant_headless schema schemas', workflow)
        self.assertIn('run: git diff --exit-code -- schemas', workflow)


if __name__ == '__main__':
    unittest.main()
