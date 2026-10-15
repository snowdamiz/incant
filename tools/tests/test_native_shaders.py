"""Public selection and strict fresh compiler/output admission; no native tools."""
import copy
import json
import os
import re
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import export_native
import native_shaders as shaders
from native_package import sha256

ROOT = Path(__file__).resolve().parents[2]
TARGET = 'aarch64-apple-darwin'


class NativeShaders(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.target = Path(self.tmp.name).resolve() / 'target'
        self.output = self.target / TARGET / 'shipping/build/incant_render-fixture/out'
        self.output.mkdir(parents=True)
        self.messages = []
        for name, kinds, flags in [('incant_render', ['lib'], shaders.COOKED),
                ('incant_game_player', ['lib', 'bin'], {shaders.FEATURE, 'native-window'})]:
            for kind in kinds:
                self.messages.append({'reason': 'compiler-artifact', 'package_id': name + '#0.0.1',
                    'target': {'name': name, 'kind': [kind]}, 'features': sorted(flags),
                    'filenames': [str(self.target / TARGET / 'shipping' / (name + '-' + kind))]})
        self.messages += [{'reason': 'build-script-executed', 'package_id': 'incant_render#0.0.1',
                           'out_dir': str(self.output)}, {'reason': 'build-finished', 'success': True}]
        self.receipts = {}
        for family, directory in shaders.DIRECTORIES.items():
            directory = self.output / directory
            directory.mkdir()
            stages = ['Compute'] if family == 'cluster' else ['Vertex', 'Fragment']
            artifacts = []
            for backend in ['Metal', 'Vulkan', 'Dx12']:
                for stage in stages:
                    data = struct.pack('<5I', 0x07230203, 0x00010000, 0, 1, 0) if backend == 'Vulkan' else b'fixture native compiler output'
                    file = backend + '-' + stage + '.shader'
                    (directory / file).write_bytes(data)
                    artifacts.append({'backend': backend, 'stage': stage, 'entry': 'build' if stage == 'Compute' else stage.lower(),
                                      'file': file, 'bytes': len(data), 'sha256': sha256(data)})
            if family == 'output':
                paths = ['src/lib.rs', 'src/contract.rs', 'src/emit.rs', 'src/manifest.rs', 'src/audit.rs', 'Cargo.toml']
            elif family.startswith('ssao-'):
                paths = ['src/ambient_occlusion/' + n for n in ('mod.rs', 'contract.rs', 'emit.rs')]
                paths += ['src/geometry/' + n for n in ('mod.rs', 'contract.rs', 'interpolation.rs', 'emit.rs',
                    'sprite.rs', 'ui.rs', 'text.rs', 'material.rs', 'material_emit.rs')]
                paths += ['src/lib.rs', 'src/audit.rs', 'Cargo.toml', '../incant_render/src/ambient_occlusion/source.rs']
            elif family.startswith('bloom-'):
                paths = ['src/bloom/' + n for n in ('mod.rs', 'contract.rs', 'emit.rs')]
                paths += ['src/audit.rs', 'Cargo.toml', '../incant_render/src/bloom/source.rs']
            elif family == 'cluster':
                paths = ['src/compute/' + n for n in ('mod.rs', 'contract.rs', 'emit.rs', 'static_workgroup.rs', 'spirv_workgroup.rs')]
                paths += ['src/audit.rs', 'Cargo.toml']
            else:
                paths = ['src/geometry/' + n for n in ('mod.rs', 'contract.rs', 'interpolation.rs', 'emit.rs', 'sprite.rs',
                                                        'ui.rs', 'text.rs', 'material.rs', 'material_emit.rs')]
                paths += ['src/audit.rs', 'Cargo.toml']
            receipt = {'format': 'incant-fixed-shader-build', 'version': 1, 'contract': shaders.CONTRACTS[family],
                'source_sha256': sha256(shaders.source_bytes(family)),
                'producer_sha256': sha256(b'\n'.join((ROOT / 'crates/incant_shader_cook' / n).read_bytes() for n in paths)),
                'artifacts': artifacts}
            compact = family == 'cluster'
            space = '' if compact else ' '
            text = f'const MANIFEST_CONTRACT: &str = {json.dumps(receipt["contract"])};\n'
            for label in ['SOURCE', 'PRODUCER']:
                values = list(bytes.fromhex(receipt[label.lower() + '_sha256']))
                text += f'const {label}_HASH: [u8;{space}32] = {values};\n'
            text += f'const ARTIFACTS: [Artifact;{space}{len(artifacts)}] = [\n'
            for a in artifacts:
                text += ('Artifact { backend: wgpu::Backend::' + a['backend'] + ', stage: Stage::' + a['stage']
                    + ', entry: ' + json.dumps(a['entry']) + ', hash: ' + str(list(bytes.fromhex(a['sha256'])))
                    + ', bytes: include_bytes!(' + json.dumps(a['file']) + ') },\n')
            text += '];\n'
            (directory / 'manifest.rs').write_text(text)
            receipt['manifest_sha256'] = sha256(text.encode())
            self.receipts[family] = receipt
            self.write(family)

    def write(self, family):
        (self.output / shaders.DIRECTORIES[family] / 'audit.json').write_text(json.dumps(self.receipts[family]))

    def audit(self, **kwargs):
        return shaders.audit(self.messages, 'cooked', target_dir=self.target, target=TARGET,
                             profile='shipping', physics_mode='fast', **kwargs)

    def test_omitted_default_does_not_inspect_compiler_or_files(self):
        self.assertEqual(shaders.features(None, 'headless'), [])
        self.assertEqual(shaders.features(None, 'window'), [])
        self.assertIsNone(shaders.audit(None, None, target_dir=None, target=None, profile=None, physics_mode=None))

    def test_public_cli_keeps_independent_options_with_explicit_cooked_selection(self):
        with mock.patch.object(sys, 'argv', ['export_native.py', 'project', 'output', '--mode', 'window',
                '--shaders', 'cooked', '--profiling', 'summary', '--ui-viewport', '640x480']), \
                mock.patch.object(export_native, 'export', return_value={'target': TARGET, 'player_sha256': 'a' * 64}) as export, \
                mock.patch('builtins.print'):
            export_native.main()
        export.assert_called_once_with(Path('project'), Path('output'), None, None, 'window',
            wabt=None, ui_viewport='640x480', profiling='summary', shaders='cooked')

    def test_audited_contracts_match_each_actual_runtime_selector(self):
        for family, path, index in [('output', 'output/cooked.rs', 0),
                ('diagnostic', 'geometry_shader/cooked.rs', 0), ('shadow', 'geometry_shader/cooked.rs', 1),
                ('material', 'material_shader.rs', 0), ('sprite', 'sprites/shader/cooked.rs', 0),
                ('ui', 'ui/shader/cooked.rs', 0), ('text', 'text/shader/cooked.rs', 0), ('cluster', 'lighting/cooked.rs', 0),
                ('bloom-prefilter', 'bloom/cooked.rs', 0), ('bloom-downsample', 'bloom/cooked.rs', 1),
                ('bloom-upsample', 'bloom/cooked.rs', 2), ('bloom-display', 'bloom/cooked.rs', 3),
                ('ssao-prepass', 'ambient_occlusion/cooked.rs', 0), ('ssao-forward', 'ambient_occlusion/cooked.rs', 1),
                ('ssao-sample', 'ambient_occlusion/cooked.rs', 2), ('ssao-horizontal', 'ambient_occlusion/cooked.rs', 3),
                ('ssao-vertical', 'ambient_occlusion/cooked.rs', 4)]:
            source = (ROOT / 'crates/incant_render/src' / path).read_text()
            contracts = re.findall(r'(?:const CONTRACT: &str = |expected: )"([^"\n]+)"', source)
            self.assertEqual(shaders.CONTRACTS[family], contracts[index])

    def test_malformed_compiler_metadata_refuses_explicitly(self):
        for messages in [None, [None], [dict(self.messages[0], target=None)]]:
            with self.assertRaises(ValueError):
                shaders.audit(messages, 'cooked', target_dir=self.target, target=TARGET,
                    profile='shipping', physics_mode='fast')

    def test_public_unknown_or_headless_selection_refuses_before_any_tool_or_input(self):
        for mode, route in [('cooked', 'headless'), ('unknown', 'window')]:
            with self.subTest(mode=mode), mock.patch.object(export_native, 'execute', side_effect=AssertionError('tool launched')):
                with self.assertRaises(ValueError):
                    export_native.export(Path('/not-read'), Path('/not-created'), mode=route, shaders=mode)
                with self.assertRaises(ValueError):
                    export_native.build(None, None, None, None, None, route, shaders=mode)
        with self.assertRaises(ValueError):
            shaders.target_admission('cooked', 'wasm32-unknown-unknown')

    def test_complete_target_and_all_seventeen_families_are_reported_without_publication(self):
        result = self.audit()
        self.assertEqual(set(result['families']), set(shaders.DIRECTORIES))
        self.assertEqual(sum(len(f['artifacts']) for f in result['families'].values()), 99)
        self.assertFalse(result['runtime_wgsl_in_selected_ordinary_catalog'])
        self.assertEqual(result['compiler_features']['incant_render'], sorted(shaders.COOKED))
        self.assertFalse((self.output / 'export.json').exists())

    def test_each_missing_ssao_program_or_partial_feature_refuses(self):
        for family in ['ssao-prepass', 'ssao-forward', 'ssao-sample', 'ssao-horizontal', 'ssao-vertical']:
            path = self.output / shaders.DIRECTORIES[family] / 'audit.json'
            data = path.read_bytes(); path.unlink()
            with self.assertRaises(OSError): self.audit()
            path.write_bytes(data)
        self.messages[0]['features'].remove('cooked-ssao')
        with self.assertRaisesRegex(ValueError, 'Missing/extra actual cooked'): self.audit()

    def test_ssao_source_assembly_preserves_original_programs_and_rejects_changed_seams(self):
        base = (ROOT / 'crates/incant_render/src/model_material.wgsl').read_bytes()
        self.assertEqual(shaders.source_bytes('ssao-prepass'), base.split(b'@fragment fn fragment', 1)[0] + b'\n'
                         + (ROOT / 'crates/incant_render/src/ambient_occlusion/prepass.wgsl').read_bytes())
        seam = b'(1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance'
        expected = b'\n'.join([(ROOT / 'crates/incant_render/src/shadows/shade.wgsl').read_bytes(),
            (ROOT / 'crates/incant_render/src/lighting/shade.wgsl').read_bytes(),
            b'@group(0) @binding(1) var diffuse_accessibility:texture_2d<f32>;\n' + base.replace(seam,
                b'(' + seam + b')*textureLoad(diffuse_accessibility,vec2i(in.position.xy),0).r')])
        self.assertEqual(shaders.source_bytes('ssao-forward'), expected)
        for family in ['ssao-prepass', 'ssao-forward']:
            with self.subTest(family=family), mock.patch.object(shaders, 'read', side_effect=lambda p, n:
                    b'changed material seam' if p.name == 'model_material.wgsl' else p.read_bytes()):
                with self.assertRaisesRegex(ValueError, 'SSAO .* seam changed'): shaders.source_bytes(family)

    def test_each_missing_bloom_program_refuses_complete_catalog_admission(self):
        for family in ['bloom-prefilter', 'bloom-downsample', 'bloom-upsample', 'bloom-display']:
            path = self.output / shaders.DIRECTORIES[family] / 'audit.json'
            data = path.read_bytes()
            path.unlink()
            with self.assertRaises(OSError):
                self.audit()
            path.write_bytes(data)

    def test_bloom_display_source_uses_exact_single_guarded_composition(self):
        original = (ROOT / 'crates/incant_render/src/output.wgsl').read_bytes()
        seam = b'var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);'
        self.assertEqual(original.count(seam), 1)
        expected = b'\n'.join([original.replace(seam, b'var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }'),
            (ROOT / 'crates/incant_render/src/tone_map.wgsl').read_bytes(),
            (ROOT / 'crates/incant_render/src/bloom/output.wgsl').read_bytes()])
        self.assertEqual(shaders.source_bytes('bloom-display'), expected)
        with mock.patch.object(shaders, 'read', side_effect=lambda path, limit: b'changed output seam' if path.name == 'output.wgsl' and path.parent.name == 'src' else path.read_bytes()):
            with self.assertRaisesRegex(ValueError, 'seam changed'):
                shaders.source_bytes('bloom-display')

    def test_deterministic_physics_flag_preserved(self):
        self.messages[0]['features'].append('physics-deterministic')
        result = shaders.audit(self.messages, 'cooked', target_dir=self.target, target=TARGET,
                               profile='shipping', physics_mode='deterministic')
        self.assertIn('physics-deterministic', result['compiler_features']['incant_render'])
        with self.assertRaises(ValueError):
            self.audit()

    def test_build_host_cannot_substitute_for_target(self):
        host = copy.deepcopy(self.messages[0])
        host['filenames'] = [str(self.target / 'shipping/incant_render-host')]
        self.messages.insert(0, host)
        self.assertEqual(self.audit()['ignored_build_host'], ['incant_render'])
        del self.messages[1]
        with self.assertRaisesRegex(ValueError, 'Missing actual'):
            self.audit()

    def test_missing_or_conflicting_or_unknown_feature_units_refuse(self):
        for change in ['missing', 'extra', 'duplicate', 'conflicting']:
            with self.subTest(change=change):
                original = copy.deepcopy(self.messages)
                if change == 'missing': self.messages[0]['features'].remove('cooked-text')
                elif change == 'extra': self.messages[0]['features'].append('cooked-temporal')
                elif change == 'duplicate': self.messages[0]['features'].append('cooked-text')
                else: self.messages[-2:-2] = [dict(self.messages[0], features=sorted(shaders.COOKED | {'physics-deterministic'}))]
                with self.assertRaises(ValueError): self.audit()
                self.messages = original

    def test_missing_player_library_or_binary_and_failed_build_refuse(self):
        for index in [1, 2, 4]:
            with self.subTest(index=index):
                original = self.messages.pop(index)
                with self.assertRaises(ValueError): self.audit()
                self.messages.insert(index, original)
        self.messages[-1]['success'] = False
        with self.assertRaises(ValueError): self.audit()

    def test_foreign_or_duplicate_build_output_refuses(self):
        original = self.messages[-2]['out_dir']
        self.messages[-2]['out_dir'] = str(self.target.parent / 'project-supplied')
        with self.assertRaises(ValueError): self.audit()
        self.messages[-2]['out_dir'] = original
        self.messages.append(copy.deepcopy(self.messages[-2]))
        with self.assertRaises(ValueError): self.audit()

    def test_changed_contract_source_and_producer_refuse(self):
        for field in ['contract', 'source_sha256', 'producer_sha256']:
            original = self.receipts['text'][field]
            self.receipts['text'][field] = 'changed' if field == 'contract' else 'a' * 64
            self.write('text')
            with self.assertRaises(ValueError): self.audit()
            self.receipts['text'][field] = original
        self.write('text')

    def test_late_bad_family_retains_all_prior_outputs(self):
        prior = {str(p): p.read_bytes() for p in self.output.rglob('*') if p.is_file()}
        file = self.output / 'cluster-shaders/Metal-Compute.shader'
        file.write_bytes(b'bad')
        with self.assertRaises(ValueError): self.audit()
        for name, data in prior.items():
            if name != str(file): self.assertEqual(Path(name).read_bytes(), data)
        self.assertFalse((self.output / 'export.json').exists())

    def test_duplicate_missing_and_foreign_stage_refuse(self):
        saved = copy.deepcopy(self.receipts['output']['artifacts'])
        for artifacts in [saved[:-1], saved[:-1] + saved[:1], saved[:-1] + [dict(saved[-1], backend='Gl')]]:
            self.receipts['output']['artifacts'] = artifacts; self.write('output')
            with self.assertRaises(ValueError): self.audit()

    def test_paths_entry_sizes_and_unknown_fields_refuse(self):
        saved = copy.deepcopy(self.receipts['output']['artifacts'][0])
        for field, value in [('file', '../escape.shader'), ('entry', 'entry;code'), ('bytes', True),
                             ('bytes', 1024**2 + 1), ('bytes', 0), ('extra', 1)]:
            self.receipts['output']['artifacts'][0] = dict(saved, **{field: value}); self.write('output')
            with self.assertRaises(ValueError): self.audit()

    def test_manifest_mismatch_refuses_even_if_receipt_rehashes_it(self):
        file = self.output / 'output-shaders/manifest.rs'
        file.write_bytes(file.read_bytes() + b'// changed executable selection\n')
        self.receipts['output']['manifest_sha256'] = sha256(file.read_bytes()); self.write('output')
        with self.assertRaisesRegex(ValueError, 'differs from embedded'):
            self.audit()

    def test_unknown_duplicate_receipt_keys_and_boolean_version_refuse(self):
        file = self.output / 'output-shaders/audit.json'
        for text in [json.dumps(dict(self.receipts['output'], extra=1)),
                     json.dumps(dict(self.receipts['output'], version=True)),
                     '{"version":1,' + json.dumps(self.receipts['output'])[1:]]:
            file.write_text(text)
            with self.assertRaises(ValueError): self.audit()

    @unittest.skipIf(os.name == 'nt', 'POSIX symlink fixture')
    def test_linked_compiler_artifact_refuses(self):
        file = self.output / 'text-shaders/Metal-Vertex.shader'
        moved = file.with_suffix('.retained'); file.rename(moved); file.symlink_to(moved.name)
        with self.assertRaisesRegex(ValueError, 'Linked package'):
            self.audit()

    def test_native_shader_compiler_cannot_enter_normal_player_graph(self):
        with self.assertRaisesRegex(ValueError, 'Authoring/JIT'):
            export_native.normal_dependencies('incant_game_player v0.0.1|\nincant_shader_cook v0.0.1|')


if __name__ == '__main__':
    unittest.main()
