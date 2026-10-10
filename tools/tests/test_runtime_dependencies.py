import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    'runtime_dependencies', Path(__file__).parents[1] / 'check_runtime_dependencies.py',
)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


def graph():
    names = ['incant_runtime', 'adapter', 'incant_doc', 'loro', 'serde']
    packages = [{'id': f'pkg:{name}', 'name': name} for name in names]
    nodes = [{'id': p['id'], 'deps': []} for p in packages]
    return {'packages': packages, 'resolve': {'nodes': nodes}}


def edge(data, source, dest, kind=None, target=None):
    node = next(n for n in data['resolve']['nodes'] if n['id'] == f'pkg:{source}')
    node['deps'].append({
        'name': 'renamed_dependency', 'pkg': f'pkg:{dest}',
        'dep_kinds': [{'kind': kind, 'target': target}],
    })


class RuntimeDependencies(unittest.TestCase):
    def test_rejects_transitive_renamed_and_target_specific_authoring_edge(self):
        data = graph()
        edge(data, 'incant_runtime', 'adapter')
        edge(data, 'adapter', 'incant_doc', target='cfg(target_os = "android")')
        self.assertEqual(guard.violations(data, ['incant_runtime']), [
            ['incant_runtime', 'adapter', 'incant_doc'],
        ])

    def test_checks_normal_edge_even_when_same_dependency_is_also_for_tests(self):
        data = graph()
        edge(data, 'incant_runtime', 'loro', 'dev')
        edge(data, 'incant_runtime', 'loro')
        self.assertEqual(guard.violations(data, ['incant_runtime']), [
            ['incant_runtime', 'loro'],
        ])

    def test_dev_and_build_only_dependencies_do_not_enter_runtime(self):
        data = graph()
        edge(data, 'incant_runtime', 'serde')
        edge(data, 'incant_runtime', 'incant_doc', 'dev')
        edge(data, 'serde', 'loro', 'build')
        self.assertEqual(guard.violations(data, ['incant_runtime']), [])

    def test_missing_runtime_cannot_pass_vacuously(self):
        data = graph()
        with self.assertRaises(ValueError):
            guard.violations(data, ['not_implemented'])
        data['resolve']['nodes'] = []
        with self.assertRaises(ValueError):
            guard.violations(data, ['incant_runtime'])

    def test_checks_all_roots_and_stops_dependency_cycles(self):
        data = graph()
        edge(data, 'adapter', 'serde')
        edge(data, 'serde', 'adapter')
        edge(data, 'adapter', 'loro')
        self.assertEqual(guard.violations(data, ['incant_runtime', 'adapter']), [
            ['adapter', 'loro'],
        ])
