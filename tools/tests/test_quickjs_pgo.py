"""Portable protocol controls; no compiler, native program or GPU execution."""
import copy
import json
from pathlib import Path
import shlex
import sys
import tempfile
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import quickjs_pgo as p
import quickjs_pgo_build as build


class Protocol(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.compiler = self.root / 'clang'
        self.compiler.write_bytes(b'fake compiler, never executed')
        self.vendor = self.root / 'vendor'
        (self.vendor / 'quickjs').mkdir(parents=True)
        for name in (*p.HEADERS, *p.UNITS):
            (self.vendor / 'quickjs' / name).write_bytes(name.encode())
        self.target = self.root / 'target'
        self.target.mkdir()
        self.base = ['-O3', '-g', '-D_GNU_SOURCE']

    def desc(self, mode='use'):
        runtime = self.root / 'libclang_rt.profile_osx.a'; runtime.write_bytes(b'archive')
        profdata = self.root / 'file.profdata'; profdata.write_bytes(b'profile')
        raw = self.root / 'raw'; raw.mkdir(exist_ok=True)
        tool = p.file_identity(self.compiler)
        tools = {'compiler': tool, 'archiver': tool, 'profdata_tool': tool,
                 'profile_runtime': p.file_identity(runtime), 'sdk': str(self.root),
                 'resource_directory': str(self.root), 'tool_inputs': [tool]}
        return p.descriptor(mode, tools, self.base, self.vendor,
                            profile=profdata if mode == 'use' else None,
                            raw_directory=raw if mode == 'generate' else None)

    def commands(self, desc=None):
        lines = []
        for role in ('cooker', 'runtime'):
            out = self.target / (p.TARGET if role == 'runtime' else '') / 'shipping/build/sys/out'
            out.mkdir(parents=True, exist_ok=True)
            for unit in p.UNITS:
                source = Path('quickjs') / unit if desc else out / unit
                if not desc:
                    source.write_bytes(unit.encode())
                args = self.base if desc is None else p.insert_flags(self.base, p.additions(desc))
                tokens = ['LC_ALL=C', str(self.compiler), *args, '-o', str(out / (unit + '.o')), '-c', str(source)]
                lines.append('[rquickjs-sys 0.14.0] running: ' + shlex.join(tokens))
        return '\n'.join(lines)

    def test_actual_role_parser_requires_both_complete_ordered_four_TU_recipes(self):
        self.assertEqual(len(p.c_commands(self.commands(), self.target, self.vendor)), 8)
        for mode in ('generate', 'use'):
            d = self.desc(mode)
            raw = self.commands(d)
            self.assertEqual(len(p.c_commands(raw, self.target, self.vendor, expected=d)), 8)
            for bad in [raw.rsplit('\n', 1)[0], raw + '\n' + raw.splitlines()[0],
                        raw.replace('-O3 -g', '-g -O3'), raw.replace('quickjs/quickjs.c', 'other/quickjs.c')]:
                with self.subTest(mode=mode), self.assertRaises(ValueError):
                    p.c_commands(bad, self.target, self.vendor, expected=d)

    def test_changed_copied_input_is_refused(self):
        raw = self.commands()
        source = self.target / p.TARGET / 'shipping/build/sys/out/quickjs.c'
        source.write_bytes(b'altered copied unit')
        with self.assertRaisesRegex(ValueError, 'translation unit changed'):
            p.c_commands(raw, self.target, self.vendor)

    def test_descriptor_identity_binds_mode_tool_and_actual_profile_content(self):
        a = self.desc()
        b = copy.deepcopy(a); b['profile']['path'] = '/relocated/same/profile'
        self.assertEqual(p.identity(a), p.identity(b))
        for change in [lambda x: x['profile'].update(sha256='0'*64),
                       lambda x: x['compiler'].update(sha256='0'*64),
                       lambda x: x.update(mode='generate'),
                       lambda x: x['recipe']['base_args'].insert(0, '-DNDEBUG')]:
            b = copy.deepcopy(a); change(b)
            self.assertNotEqual(p.identity(a), p.identity(b))

    def test_fixed_canonical_identity_matches_Rust_build_support(self):
        d = self.desc()
        d.update(sdk='/pinned/sdk', resource_directory='/pinned/resource')
        for key in ('compiler', 'archiver', 'profdata_tool', 'profile_runtime'):
            d[key].update(bytes=5, sha256='1'*64)
        d['profile'].update(bytes=7, sha256='2'*64)
        d['tool_inputs'] = [{'path':'/pinned/sdk/SDKSettings.json','bytes':3,'sha256':'3'*64},
                            {'path':'/pinned/resource/metadata','bytes':8,'sha256':'4'*64}]
        self.assertEqual(p.identity(d), 'f38a676e755b1f1cde6fc258e1d622304e2f68689642a0cbea11064d7fd58c63')

    def test_only_known_cc_insertion_is_admitted_no_warning_suppression(self):
        d = self.desc(); extra = p.additions(d)
        self.assertIn('-Werror=backend-plugin', extra)
        self.assertIn('-Werror=unknown-warning-option', extra)
        self.assertEqual(p.insert_flags(self.base, extra), self.base[:-1]+extra+self.base[-1:])
        for invalid in [[], ['-g'], ['-w', '-D_GNU_SOURCE'], ['-fprofile-use=x', '-D_GNU_SOURCE']]:
            with self.assertRaises(ValueError): p.insert_flags(invalid, extra)
        with self.assertRaisesRegex(ValueError, 'No custom measured baseline'):
            p.descriptor('ordinary', {}, [], self.vendor)

    def test_exact_names_refuse_near_match_foreign_prefix_missing_and_duplicates(self):
        names = 'quickjs/quickjs.c;JS_CallInternal\nJS_ReadObject\nJS_EvalFunction\n'
        self.assertEqual(len(p.covered_names(names)), 3)
        for bad in [names.replace('quickjs/quickjs.c', '/foreign/quickjs.c'),
                    names.replace('JS_CallInternal', 'JS_CallInternalExtra'),
                    names.replace('JS_ReadObject\n', ''), names + 'JS_ReadObject\n']:
            with self.assertRaises(ValueError): p.covered_names(bad)

    def test_actual_final_link_requires_owned_output_and_training_only_runtime(self):
        runtime = '/pinned/libclang_rt.profile_osx.a'
        for mode in p.MODES:
            tokens = [str(self.compiler), '-o', str(self.target / 'binary')]
            tokens += ['-Wl,-force_load', '-Wl,'+runtime] if mode == 'generate' else ['-Wl,-dead_strip']
            raw = shlex.join(tokens)
            p.final_link(raw, mode, runtime, compiler=str(self.compiler), target=self.target)
            for bad in [raw.replace(str(self.target / 'binary'), '/foreign/binary'), raw + '\n' + raw,
                        raw + (' -Wl,-dead_strip' if mode == 'generate' else ' -Wl,'+runtime)]:
                with self.assertRaises(ValueError):
                    p.final_link(bad, mode, runtime, compiler=str(self.compiler), target=self.target)

    def test_training_instrumentation_is_not_source_free_or_a_default_use_feature(self):
        for mode in ('ordinary', 'use'):
            p.instrumentation(mode, 'plain symbols', 'plain sections')
            with self.assertRaises(ValueError): p.instrumentation(mode, '__llvm_profile', '__llvm_prf')
        p.instrumentation('generate', '__llvm_profile', '__llvm_prf')
        with self.assertRaises(ValueError): p.instrumentation('generate', 'plain', 'plain')
        symbols = '\n'.join('000abc T ' + n for n in (*p.RETAINED, '_JS_ReadObject', '_JS_EvalFunction', '___JS_EvalInternal'))
        self.assertFalse(p.training_symbols(symbols)['source_free_claim'])
        with self.assertRaises(ValueError): p.training_symbols(symbols.replace('_JS_NewNumber', '_JS_NewNumberExtra'))

    def test_nonshipping_exception_is_scoped_and_always_restored(self):
        before = build.export_native.runtime_symbol_audit
        features = build.native_profiling.features
        with self.assertRaisesRegex(RuntimeError, 'synthetic'):
            with build.phase('generate'):
                self.assertIsNot(build.export_native.runtime_symbol_audit, before)
                self.assertEqual(build.native_profiling.features('summary', 'headless'), ['profile', 'diagnostic-quickjs-generate'])
                raise RuntimeError('synthetic')
        self.assertIs(build.export_native.runtime_symbol_audit, before)
        self.assertIs(build.native_profiling.features, features)
        with build.phase('ordinary'):
            self.assertIs(build.export_native.runtime_symbol_audit, before)
            self.assertEqual(build.native_profiling.features('summary', 'headless'), ['profile'])

    def test_session_reports_training_exception_in_both_deletion_claims(self):
        session = build.Session.__new__(build.Session); session.training = [{'actual_training_receipt': True}]
        original = {'build_storage_removed_before_any_player': True,
                    'authoring_inputs_removed_before_any_player': True, 'preserved_origins': ['immutable']}
        with patch.object(build.FreshSession, 'report', return_value=original):
            report = session.report()
        self.assertNotIn('build_storage_removed_before_any_player', report)
        self.assertNotIn('authoring_inputs_removed_before_any_player', report)
        self.assertTrue(report['build_storage_removed_before_baseline_and_use_validation'])
        self.assertTrue(report['authoring_inputs_removed_before_baseline_and_use_validation'])
        self.assertEqual(report['preserved_origins'], ['immutable'])

    def test_three_cargo_vectors_change_only_training_retention_and_print_request(self):
        s = build.Session.__new__(build.Session); s.directory = self.root
        result = type('Result', (), {'stdout': '', 'stderr': ''})()
        for mode in p.MODES:
            s.mode = mode
            with patch.object(build.FreshSession, 'build', return_value=(result, 'proof')) as call:
                s.build(['cargo', 'build', '--bin', 'incant_game_player'], cwd=self.root, env={})
            command = call.call_args.args[0]
            self.assertEqual(command[:2], ['cargo', 'rustc'])
            self.assertEqual('link-dead-code=yes' in command, mode == 'generate')
            self.assertIn('--print=link-args='+str(self.root/'link-args.txt'), command)

    def test_ambient_overrides_are_not_experiment_selection(self):
        build.environment({'PATH':'/pinned', 'SDKROOT':'/sdk'})
        for key in ['CFLAGS', 'TARGET_CFLAGS', 'CC', 'AR', 'LLVM_PROFILE_FILE',
                    p.SELECTOR, 'CARGO_BUILD_RUSTC', 'RUSTFLAGS']:
            with self.subTest(key=key), self.assertRaises(ValueError): build.environment({key:'untrusted'})

    def test_raw_merge_inventory_never_accepts_a_wildcard_pool_or_heldout_contamination(self):
        raw = self.root / 'training-raw'; raw.mkdir(); (raw/'training-123.profraw').write_bytes(b'raw')
        rows = build.raw_training(raw); self.assertEqual(len(rows), 1)
        (raw/'heldout.profraw').write_bytes(b'raw')
        with self.assertRaises(ValueError): build.raw_training(raw)

    def role_fixture(self):
        d = self.desc()
        directory = self.root / 'receipts'; directory.mkdir()
        p.create_json(directory / 'descriptor.json', d)
        messages = []
        for package, prefix, version in (('incant_script', 'crates', '0.0.1'),
                                         ('rquickjs-sys', 'third_party', '0.14.0')):
            for role in ('cooker', 'runtime'):
                out = self.target / (p.TARGET if role == 'runtime' else '') / ('shipping/build/'+package+'/out')
                out.mkdir(parents=True)
                data = {'format': 'incant-quickjs-pgo-build-v1', 'package': package,
                        'mode': d['mode'], 'identity': p.identity(d), 'target': p.TARGET,
                        'default_c_flags': False, 'source_free_claim': False, 'out_dir': str(out),
                        'compiler': d['compiler'], 'descriptor_sha256': p.digest(directory/'descriptor.json'),
                        'recipe': d['recipe'], 'profile': d['profile'],
                        'actual_args': None if package == 'incant_script' else p.insert_flags(self.base, p.additions(d))}
                p.create_json(out / 'quickjs-pgo.json', data)
                messages.append({'reason': 'build-script-executed',
                    'package_id': 'path+'+(build.ROOT/prefix/package).as_uri()+'#'+version,
                    'out_dir': str(out), 'env': [['INCANT_SCRIPT_DEFAULT_C_FLAGS', '0'],
                                               ['INCANT_SCRIPT_BUILD_ID', 'a'*64]]})
        producer = {'script': {'build_id': 'a'*64, 'bytecode_abi': {'default_c_flags': False}}}
        return d, directory, messages, producer

    def test_actual_receipts_require_host_and_runtime_with_exact_cooked_identity(self):
        d, directory, messages, producer = self.role_fixture()
        result = build.role_receipts(messages, self.target, d, directory, producer)
        self.assertEqual(len(result['roles']), 4)
        self.assertEqual(result['actual_build_id'], 'a'*64)

    def test_role_membership_foreign_paths_profile_and_runtime_build_mismatch_refuse(self):
        d, directory, messages, producer = self.role_fixture()
        checks = [lambda m: m.pop(),
                  lambda m: m.append(copy.deepcopy(m[0])),
                  lambda m: m[0].update(package_id=m[0]['package_id'].replace('/crates/', '/foreign/')),
                  lambda m: m[0]['env'][1].__setitem__(1, 'b'*64)]
        for index, mutate in enumerate(checks):
            m = copy.deepcopy(messages); mutate(m)
            attempt = self.root / ('attempt-'+str(index)); attempt.mkdir()
            (attempt/'descriptor.json').write_bytes((directory/'descriptor.json').read_bytes())
            with self.subTest(index=index), self.assertRaises(ValueError):
                build.role_receipts(m, self.target, d, attempt, producer)
        # Unchanged claimed identity cannot cover a changed actual profile receipt.
        path = Path(messages[0]['out_dir'])/'quickjs-pgo.json'
        data = json.loads(path.read_text()); data['profile']['sha256'] = '0'*64
        path.write_text(json.dumps(data))
        attempt = self.root/'changed-receipt'; attempt.mkdir()
        (attempt/'descriptor.json').write_bytes((directory/'descriptor.json').read_bytes())
        with self.assertRaisesRegex(ValueError, 'Actual role/descriptor identity mismatch'):
            build.role_receipts(messages, self.target, d, attempt, producer)

    def test_bound_route_and_tool_bytes_are_rechecked(self):
        path = self.root/'bound-input'; path.write_bytes(b'original')
        records = [p.file_identity(path)]
        build.check_inputs(records)
        path.write_bytes(b'modified')
        with self.assertRaisesRegex(ValueError, 'Experiment input changed'): build.check_inputs(records)

    def test_actual_feature_closure_binds_both_roles_and_excludes_mixed_modes(self):
        for mode in p.MODES:
            rows = []
            for name, folder, version, roles in (
                ('incant_script','crates','0.0.1',('cooker','runtime')),
                ('rquickjs-sys','third_party','0.14.0',('cooker','runtime')),
                ('incant_cook','crates','0.0.1',('cooker',)),
                ('incant_game_player','crates','0.0.1',('runtime',))):
                for role in roles:
                    for kind in (('lib','bin') if name == 'incant_game_player' else ('lib',)):
                        directory = self.target / (p.TARGET if role == 'runtime' else '') / 'shipping'
                        rows.append({'reason':'compiler-artifact',
                            'package_id':'path+'+(build.ROOT/folder/name).as_uri()+'#'+version,
                            'target':{'name':name.replace('-','_'),'kind':[kind]},
                            'features':[] if mode == 'ordinary' else [p.FEATURES[mode]],
                            'filenames':[str(directory/(name+'.'+kind))]})
            self.assertEqual(len(p.artifact_features(rows,mode,self.target,build.ROOT)),7)
            for mutation in ('missing','extra','foreign','mode'):
                bad = copy.deepcopy(rows)
                if mutation == 'missing':bad.pop()
                elif mutation == 'extra':bad.append(bad[0])
                elif mutation == 'foreign':bad[0]['filenames']=['/foreign/lib.rlib']
                else:bad[0]['features']=list(p.FEATURES.values())
                with self.subTest(mode=mode,mutation=mutation), self.assertRaises(ValueError):
                    p.artifact_features(bad,mode,self.target,build.ROOT)


if __name__ == '__main__': unittest.main()
