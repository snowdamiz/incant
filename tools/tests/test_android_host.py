import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import android_archive
import android_host
import export_android_host


class AndroidHost(unittest.TestCase):
    def test_api26_is_distinct_and_fixed(self):
        self.assertEqual(android_archive.API, 24)
        self.assertEqual(android_host.API, 26)
        tools = {k: Path('/ndk')/k for k in ('clang','clangxx','ar','llvm','sysroot','resource')}
        env, chosen = android_host.environment({}, Path('/rust/bin/rustc'), Path('/project'), None, tools)
        self.assertIn('--target=aarch64-linux-android26', chosen['BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android'])
        self.assertNotIn('android24', chosen['BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android'])
        self.assertEqual(env['CARGO_BUILD_JOBS'], '2')
        for key in ('CARGO_FEATURE_COMPILER', 'CXXFLAGS', 'CPPFLAGS'):
            with self.assertRaises(ValueError):
                android_host.environment({key:''}, Path('/rust/bin/rustc'), Path('/project'), None, tools)

    def test_static_wasm_refuses_before_build_or_output(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve(); project = root/'project'; output = root/'output'
            project.write_text(json.dumps({'scenes':{'s':{'entities':{},'program':{'kind':'wasm','module':'m'}}},
                                           'assets':{'m':{'kind':'wasm','sha256':'a'*64}}}))
            with patch.object(export_android_host, 'build') as build, self.assertRaisesRegex(ValueError, 'TypeScript only'):
                export_android_host.export(project, output, root, root, root)
            build.assert_not_called(); self.assertFalse(output.exists())

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); project = root/'p'; project.write_text('{}'); output = root/'out'; output.mkdir()
            (output/'keep').write_text('old')
            with self.assertRaisesRegex(ValueError,'new output'):
                export_android_host.export(project, output, root, root, root)
            self.assertEqual((output/'keep').read_text(),'old')

    def test_index_accepts_cooked_names_and_refuses_links_traversal_oversize(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve(); (root/'package.json').write_text('{}'); (root/'fonts').mkdir()
            (root/'fonts'/('a'*64+'.ifont')).write_bytes(b'font')
            rows = android_host.index_files(root)
            self.assertEqual(len(rows),2)
            (root/'alias').symlink_to(root/'package.json')
            with self.assertRaisesRegex(ValueError,'Linked'): android_host.index_files(root)
            (root/'alias').unlink(); (root/'wrong.name.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'path'): android_host.index_files(root)
            (root/'wrong.name.json').unlink()
            with patch.object(android_host,'MAX_PACKAGE',1), self.assertRaisesRegex(ValueError,'bounds'):
                android_host.index_files(root)

    def test_catalog_line_matches_java_exact_boundary_and_long_path_refusal(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve(); (root/'package.json').write_text('{}')
            parent = root / ('a' * 220); parent.mkdir()
            boundary = parent / ('b' * 224); boundary.write_bytes(b'x')
            rows = android_host.index_files(root)
            name, payload = next(row for row in rows if row[0] != 'package.json')
            self.assertEqual(64 + 2 + len(str(len(payload))) + len(name), 512)
            boundary.rename(parent / ('b' * 225))
            with self.assertRaisesRegex(ValueError, 'catalog line exceeds 512'):
                android_host.index_files(root)

    def test_selected_graph_refuses_context_competitors_and_missing_real_backend(self):
        names=['incant_game_player','incant_script','ndk','ndk-context','jni','cpal','wgpu','incant_render_gpu_host']
        graph='\n'.join(name+' v1.0.0|' for name in names)+'\n'
        self.assertIn('cpal',android_host.graph(graph))
        for name in ('ndk-glue','android-activity','tao','gilrs'):
            with self.assertRaisesRegex(ValueError,'Conflicting'):
                android_host.graph(graph+name+' v1.0.0|\n')
        with self.assertRaisesRegex(ValueError,'Missing real'):
            android_host.graph(graph.replace('cpal v1.0.0|\n',''))

    def test_compiler_macros_must_describe_api26_arm64(self):
        class Output:
            stdout='#define __ANDROID_MIN_SDK_VERSION__ 26\n#define __aarch64__ 1\n'
        android_host.compiler({'clang':Path('/clang')},lambda _:Output())
        for value in ('24','27'):
            class Wrong: stdout=Output.stdout.replace('26',value)
            with self.assertRaisesRegex(ValueError,'API or architecture'):
                android_host.compiler({'clang':Path('/clang')},lambda _:Wrong())
