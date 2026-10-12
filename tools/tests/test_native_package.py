import io
import json
import os
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import native_package as package
import export_native


class NativePackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.producer = self.root / 'producer'
        self.producer.mkdir()
        (self.producer / 'world.scene').write_bytes(b'engine-cooked-scene')
        (self.producer / 'input-actions.json').write_text('{}')
        self.receipt = {'format': 'incant-native-project-build', 'version': 1, 'tick_rate': 60,
                        'scene_sha256': package.sha256(b'engine-cooked-scene'),
                        'visual_assets': [], 'audio_clips': {}, 'cameras': [], 'has_spatial_content': False}

    def data(self):
        return package.data_package(self.root, self.producer, self.receipt, self.root / 'data', None)

    def test_cargo_utf8_paths_survive_a_legacy_windows_text_encoding(self):
        path = self.root / 'cargo.jsonl'
        message = {'reason': 'compiler-artifact', 'executable': 'C:/Новый/世界/player.exe'}
        path.write_bytes(('Compiling engine\n' + json.dumps(message, ensure_ascii=False) + '\n').encode('utf-8'))
        original_open = io.open
        def legacy_open(file, mode='r', buffering=-1, encoding=None, *args, **kwargs):
            encoding = 'cp1252' if encoding in (None, 'locale') else encoding
            return original_open(file, mode, buffering, encoding, *args, **kwargs)
        # Use actual file bytes and decoding, with the Windows locale default.
        with patch('io.open', side_effect=legacy_open):
            with self.assertRaises(UnicodeDecodeError):
                path.read_text(encoding='locale')
            self.assertEqual(export_native.cargo_messages(path), [message])

    def test_msvc_runtime_audit_requires_the_map_and_retains_its_exact_identity(self):
        executable, link_map = self.root / 'player.exe', self.root / 'player-link.map'
        executable.write_bytes(b'PE has no COFF symbol names')
        with patch.object(export_native, 'execute', side_effect=AssertionError('PE must not use llvm-nm')):
            with self.assertRaises(FileNotFoundError):
                export_native.runtime_symbol_audit(executable, 'pdb', self.root, link_map)
            payload = b' Publics by Value\n 0001:00000010 JS_ReadObject 0000000140001010 f quickjs.obj\n 0001:00000020 JS_EvalFunction 0000000140001020 f quickjs.obj\n'
            link_map.write_bytes(payload)
            audit = export_native.runtime_symbol_audit(executable, 'pdb', self.root, link_map)
            self.assertEqual(audit, {'source': 'fresh MSVC linker map', 'file': link_map.name,
                                     'sha256': package.sha256(payload)})
            for missing in (b'JS_ReadObject', b'JS_EvalFunction'):
                with self.subTest(missing=missing):
                    link_map.write_bytes(payload.replace(missing, b'absent'))
                    with self.assertRaisesRegex(ValueError, 'Missing bytecode runtime'):
                        export_native.runtime_symbol_audit(executable, 'pdb', self.root, link_map)

    def test_msvc_runtime_audit_rejects_each_parser_and_mangled_compiler(self):
        link_map = self.root / 'player-link.map'
        for forbidden in ('JS_AddIntrinsicEval', '__JS_EvalInternal', 'js_parse_program',
                          'js_parse_function_decl', 'incant_script::compiler',
                          '_ZN13incant_script8compiler7compile17h123E',
                          '_RNvNtCs123_13incant_script8compiler7compile'):
            with self.subTest(forbidden=forbidden):
                link_map.write_text('JS_ReadObject JS_EvalFunction ' + forbidden, encoding='utf-8')
                with self.assertRaisesRegex(ValueError, 'Source compiler linked'):
                    export_native.runtime_symbol_audit(self.root / 'player.exe', 'pdb', self.root, link_map)

    def test_minimal_data_is_source_free_and_startup_hashes_are_exact(self):
        (self.root / 'source.ts').write_text('DO NOT EXPORT AUTHORING SOURCE')
        result = self.data()
        data = self.root / 'data'
        self.assertEqual((data / 'clips.bin').read_bytes(), b'NCLIPS01' + struct.pack('<I', 0))
        replay = json.loads((data / 'empty-input.json').read_text())
        self.assertEqual(replay['tick_rate'], 60)
        self.assertEqual(replay['frames'], [])
        manifest = json.loads((data / 'package.json').read_text())
        self.assertEqual(set(manifest['files']), {'world.scene', 'input-actions.json', 'clips.bin', 'empty-input.json'})
        for name, checksum in result['files'].items():
            self.assertEqual(package.sha256((data / name).read_bytes()), checksum)
        self.assertFalse((data / 'source.ts').exists())

    def test_referenced_texture_roles_copied_once_and_tampering_refused(self):
        cache = self.root / '.incant/cache/textures'
        cache.mkdir(parents=True)
        fingerprint = 'a' * 64
        content = package.sha256(b'ktx')
        unweighted = package.sha256(b'other-ktx')
        (cache / (content + '.ktx2')).write_bytes(b'ktx')
        (cache / (unweighted + '.ktx2')).write_bytes(b'other-ktx')
        (cache / (fingerprint + '.json')).write_text(json.dumps({'fingerprint': fingerprint, 'content_sha256': content,
                                                               'unweighted_content_sha256': unweighted}))
        (cache / 'unreferenced.ktx2').write_bytes(b'ignored')
        self.receipt['visual_assets'] = [{'kind': 'texture', 'fingerprint': fingerprint}] * 2
        result = self.data()
        self.assertEqual(len(list((self.root / 'data/textures').iterdir())), 3)
        self.assertEqual(result['bytes'], sum((self.root / 'data' / name).stat().st_size for name in result['files']))
        self.assertIn('textures/' + unweighted + '.ktx2', result['files'])
        (cache / (content + '.ktx2')).write_bytes(b'wrong')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            package.data_package(self.root, self.producer, self.receipt, self.root / 'bad-data', None)

    def test_camera_selection_and_id_boundaries(self):
        one, two = '00000000000000000000000001', '00000000000000000000000002'
        self.receipt.update(cameras=[one, two], has_spatial_content=True)
        with self.assertRaisesRegex(ValueError, 'Choose'):
            package.select_camera(self.receipt, None, False)
        self.assertEqual(package.select_camera(self.receipt, two, True), '0' * 31 + '2')
        with self.assertRaises(ValueError):
            package.ulid_hex('Z' * 26)
        with self.assertRaises(ValueError):
            package.ulid_hex('../outside')

    def test_links_special_entries_and_changed_hash_refused(self):
        target = self.root / 'source'
        target.write_bytes(b'data')
        with self.assertRaises(ValueError):
            package.read(target, 3)
        link = self.root / 'linked'
        try:
            link.symlink_to(target)
        except OSError:
            pass  # Windows may not grant symlink creation; hardlink below is mandatory.
        else:
            with self.assertRaisesRegex(ValueError, 'Linked'):
                package.read(link, 10)
        hard = self.root / 'hard'
        hard.hardlink_to(target)
        with self.assertRaises(ValueError):
            package.read(hard, 10)
        self.receipt['scene_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'receipt'):
            self.data()

    @unittest.skipIf(os.name == 'nt', 'POSIX executable fixture; environment keys are shared with Windows')
    def test_export_startup_omits_home_and_keeps_restricted_runtime_paths(self):
        project = self.root / 'project.json'
        project.write_text('{}')
        parent_home = os.environ.get('HOME')
        def build(_project, staging, storage, *_args):
            self.assertTrue(storage.is_dir())
            player = staging / 'player' / 'fixture'
            player.parent.mkdir()
            player.write_text('#!' + sys.executable + '\n'
                'import json, os, pathlib, sys\n'
                'pathlib.Path(sys.argv[sys.argv.index("--output")+1]).write_bytes(b"pcm")\n'
                'print(json.dumps({"environment": dict(os.environ)}))\n')
            player.chmod(0o755)
            return player, {}
        with patch.object(export_native, 'build', side_effect=build):
            report = export_native.export(project, self.root / 'exported')
        environment = report['startup']['environment']
        self.assertNotIn('HOME', environment)
        self.assertEqual(os.environ.get('HOME'), parent_home)
        self.assertEqual(Path(environment['PATH']).name, 'player')
        self.assertEqual(environment['TMPDIR'], environment['TEMP'])
        self.assertEqual(environment['TMPDIR'], environment['TMP'])
        self.assertEqual(Path(environment['PATH']).parent, Path(environment['TMPDIR']))
        for key in ('SystemRoot', 'SystemDrive', 'WINDIR'):
            self.assertEqual(environment.get(key), os.environ.get(key))
        self.assertTrue(report['build_storage_removed_before_startup'])

    def test_publication_never_replaces_existing_output(self):
        staged = self.root / 'staged'
        staged.mkdir()
        (staged / 'player').mkdir()
        (staged / 'player/game').write_bytes(b'executable')
        (staged / 'export.json').write_text('{}')
        output = self.root / 'published'
        package.publish(staged, output)
        (output / 'player/game').write_bytes(b'preserve')
        with self.assertRaises(FileExistsError):
            package.publish(staged, output)
        self.assertEqual((output / 'player/game').read_bytes(), b'preserve')
        self.assertTrue((output / 'export.json').is_file())


if __name__ == '__main__':
    unittest.main()
