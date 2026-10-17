"""Portable protocol controls; no compiler, native program or GPU execution."""
import copy
import hashlib
import json
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import quickjs_pgo as p
import quickjs_pgo_build as build
from native_package import data_package


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
        self.recipes = {role: {'optimization': opt, 'debug': True,
                              'base_args': ['-O'+str(opt), '-g', '-D_GNU_SOURCE']}
                        for role, opt in (('cooker', 0), ('runtime', 3))}

    def desc(self, mode='use'):
        runtime = self.root / 'libclang_rt.profile_osx.a'; runtime.write_bytes(b'archive')
        profdata = self.root / 'file.profdata'; profdata.write_bytes(b'profile')
        raw = self.root / 'raw'; raw.mkdir(exist_ok=True)
        tool = p.file_identity(self.compiler)
        tools = {'compiler': tool, 'archiver': tool, 'profdata_tool': tool,
                 'profile_runtime': p.file_identity(runtime), 'sdk': str(self.root),
                 'resource_directory': str(self.root), 'tool_inputs': [tool]}
        return p.descriptor(mode, tools, self.recipes, self.vendor,
                            profile=profdata if mode == 'use' else None,
                            raw_directory=raw if mode == 'generate' else None)

    def commands(self, desc=None):
        lines = []
        for role in ('cooker', 'runtime'):
            out = self.target / (p.TARGET if role == 'runtime' else '') / 'shipping/build/sys/out'
            out.mkdir(parents=True, exist_ok=True)
            for unit in p.UNITS:
                source = Path('quickjs') / unit if desc and role == 'runtime' else out / unit
                if source.is_absolute():
                    source.write_bytes(unit.encode())
                base = self.recipes[role]['base_args']
                args = base if desc is None else p.insert_flags(base, p.additions(desc, role))
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
                       lambda x: x['recipes']['runtime']['base_args'].insert(0, '-DNDEBUG')]:
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
        self.assertEqual(p.identity(d), 'e1798a9d06009c0ad83b28306705be67b7b0b9acbf02618172b2c95443502d8f')

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

    def link_wrapper(self, mode='ordinary'):
        self.compiler.chmod(0o700)
        cc = self.root / 'cc'
        if not cc.exists():
            cc.symlink_to(self.compiler)
        environment = {'LC_ALL': 'C', 'PATH': str(self.root), 'SDKROOT': str(self.root),
                       'VSLANG': '1033', 'ZERO_AR_DATE': '1'}
        prefix = ['env', '-u', 'IPHONEOS_DEPLOYMENT_TARGET', '-u', 'TVOS_DEPLOYMENT_TARGET',
                  '-u', 'XROS_DEPLOYMENT_TARGET', *[k+'='+v for k, v in environment.items()]]
        arguments = ['cc', '-o', str(self.target / 'binary')]
        arguments += (['-Wl,-force_load', '-Wl,/pinned/libclang_rt.profile_osx.a'] if mode == 'generate'
                      else ['-Wl,-dead_strip'])
        return prefix, arguments, environment

    def admit_link(self, tokens, environment, mode='ordinary'):
        return p.final_link(shlex.join(tokens), mode, '/pinned/libclang_rt.profile_osx.a',
                            compiler=str(self.compiler), target=self.target, environment=environment)

    def test_rustc_env_wrapper_uses_exact_reported_path_not_ambient_lookup(self):
        for mode in p.MODES:
            prefix, arguments, environment = self.link_wrapper(mode)
            with patch.dict(p.os.environ, {'PATH': '/nonexistent'}):
                receipt = self.admit_link(prefix + arguments, environment, mode)
            self.assertEqual(receipt['driver'], str(self.compiler))
            self.assertTrue(receipt['env_wrapper'])
            self.assertEqual(receipt['dead_strip'], mode != 'generate')
            self.assertEqual(receipt['profile_runtime'], mode == 'generate')
            # The only alternate wrapper spelling admitted is the same system
            # env, not an arbitrary executable with a matching basename.
            self.admit_link(['/usr/bin/env', *prefix[1:], *arguments], environment, mode)

    def test_env_removals_require_explicit_exact_unique_operands(self):
        prefix, arguments, environment = self.link_wrapper()
        for tokens in [prefix[:1] + prefix[3:] + arguments,
                       prefix[:2] + prefix[3:] + arguments,
                       prefix[:1] + ['-u'] + prefix[1:] + arguments,
                       prefix[:1] + ['--unset=IPHONEOS_DEPLOYMENT_TARGET'] + prefix[3:] + arguments,
                       prefix[:1] + ['-i'] + prefix[1:] + arguments,
                       [v.replace('XROS_DEPLOYMENT_TARGET', 'TVOS_DEPLOYMENT_TARGET') for v in prefix] + arguments,
                       [v.replace('XROS_DEPLOYMENT_TARGET', 'UNTRUSTED') for v in prefix] + arguments,
                       ['env', '-u']]:
            with self.subTest(tokens=tokens), self.assertRaises(ValueError):
                self.admit_link(tokens, environment)

    def test_env_assignments_refuse_missing_duplicate_foreign_or_changed_values(self):
        prefix, arguments, environment = self.link_wrapper()
        for key in environment:
            value = key + '=' + environment[key]
            for changed in [[v for v in prefix if v != value] + arguments,
                            prefix + [value] + arguments,
                            [key+'=changed' if v == value else v for v in prefix] + arguments]:
                with self.subTest(key=key), self.assertRaises(ValueError):
                    self.admit_link(changed, environment)
        for changed in [prefix + ['UNTRUSTED=1'] + arguments,
                        prefix + ['PATH_EXTRA='+environment['PATH']] + arguments,
                        prefix + arguments + ['LC_ALL=C']]:
            with self.assertRaises(ValueError): self.admit_link(changed, environment)

    def test_link_wrapper_refuses_missing_unknown_nested_and_ambiguous_drivers(self):
        prefix, arguments, environment = self.link_wrapper()
        for tokens in [[], prefix, arguments, prefix[1:] + arguments,
                       ['/untrusted/env', *prefix[1:], *arguments],
                       prefix + ['xcrun', *arguments], prefix + ['env', *arguments],
                       prefix + ['sh', *arguments], prefix + ['unknown-wrapper', *arguments],
                       prefix + [str(self.compiler), *arguments],
                       prefix + arguments + ['clang'], prefix + arguments + [str(self.compiler)],
                       prefix + arguments + ['@uninspected-response-file'], prefix + ['cc', '-o']]:
            with self.subTest(tokens=tokens), self.assertRaises(ValueError):
                self.admit_link(tokens, environment)

    def test_wrapped_link_preserves_actual_driver_and_all_existing_flag_refusals(self):
        prefix, arguments, environment = self.link_wrapper()
        other = self.root / 'other-clang'; other.write_bytes(b'other never executed'); other.chmod(0o700)
        (self.root / 'cc').unlink(); (self.root / 'cc').symlink_to(other)
        with self.assertRaisesRegex(ValueError, 'differs from admitted'):
            self.admit_link(prefix + arguments, environment)
        (self.root / 'cc').unlink(); (self.root / 'cc').symlink_to(self.compiler)
        for mode in p.MODES:
            prefix, arguments, environment = self.link_wrapper(mode)
            for changed in [prefix + [v for v in arguments if v != '-Wl,-dead_strip'] if mode != 'generate'
                            else prefix + [v for v in arguments if v != '-Wl,-force_load'],
                            prefix + arguments + ['-Wl,-dead_strip'],
                            prefix + [v.replace(str(self.target / 'binary'), '/foreign/binary') for v in arguments],
                            prefix + arguments + ['-Wl,/pinned/libclang_rt.profile_osx.a'] if mode != 'generate'
                            else prefix + [v for v in arguments if v != '-Wl,/pinned/libclang_rt.profile_osx.a']]:
                with self.subTest(mode=mode), self.assertRaises(ValueError):
                    self.admit_link(changed, environment, mode)

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

    def test_real_session_injects_only_exact_cc_evidence_after_ambient_admission(self):
        # Exercise the real FreshSession transport up to an intentional compiler
        # refusal. No native command runs; the actual cc-only diagnostic setting
        # must survive the PGO wrapper and must not mutate caller authority.
        s = build.Session.__new__(build.Session)
        s.root, s.path = self.root, self.target
        s.target, s.split, s.deleted = p.TARGET, 'packed', False
        s.builds, s.inputs = [], []
        s.directory, s.mode = self.root, 'generate'
        argv = ['cargo', 'build', '--profile', 'shipping', '--target', p.TARGET,
                '--target-dir', str(self.target), '--config', 'profile.shipping.split-debuginfo="packed"',
                '--locked', '--message-format=json', '-vv']
        caller_env = {'PATH': '/pinned'}
        seen = []
        def execute(command, **options):
            seen.append((command, options['env']))
            return subprocess.CompletedProcess(command, 1, '', 'intentional no-compiler control')
        build.environment(caller_env)
        with patch.object(build.FreshSession, 'check_frozen'), self.assertRaisesRegex(ValueError, 'intentional no-compiler'):
            s.build(argv, cwd=self.root, env=caller_env, execute=execute)
        self.assertEqual(caller_env, {'PATH': '/pinned'})
        self.assertEqual(seen[0][1], {'PATH': '/pinned', 'CC_ENABLE_DEBUG_OUTPUT': '1'})
        with self.assertRaises(ValueError):
            build.environment(seen[0][1])  # Still forbidden as ambient authority.

    def test_ambient_overrides_are_not_experiment_selection(self):
        build.environment({'PATH':'/pinned', 'SDKROOT':'/sdk'})
        for key in ['CFLAGS', 'TARGET_CFLAGS', 'CC', 'AR', 'LLVM_PROFILE_FILE',
                    p.SELECTOR, 'CARGO_BUILD_RUSTC', 'RUSTFLAGS']:
            with self.subTest(key=key), self.assertRaises(ValueError): build.environment({key:'untrusted'})

    def test_PGO_build_dependencies_do_not_alias_existing_runtime_or_dev_packages(self):
        manifest = tomllib.loads((build.ROOT / 'crates/incant_script/Cargo.toml').read_text())
        def admit(value):
            names = {}
            for role in ('dependencies', 'build-dependencies', 'dev-dependencies'):
                for name, spec in value[role].items():
                    package = spec.get('package', name) if isinstance(spec, dict) else name
                    names.setdefault(package, set()).add(name)
            for name in ('serde', 'serde_json'):
                self.assertEqual(names[name], {name})
                self.assertTrue(value['build-dependencies'][name]['optional'])
                for mode in ('generate', 'use'):
                    self.assertIn('dep:'+name, value['features']['diagnostic-quickjs-'+mode])
        admit(manifest)
        # The actual failed Cargo shape must be rejected before another cold
        # shipping build; normal/dev and optional-build edges share one name.
        for canonical, alias in [('serde', 'pgo-serde'), ('serde_json', 'pgo-json')]:
            broken = copy.deepcopy(manifest)
            broken['build-dependencies'][alias] = dict(broken['build-dependencies'].pop(canonical), package=canonical)
            with self.subTest(package=canonical), self.assertRaises(AssertionError): admit(broken)

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
                data = {'format': 'incant-quickjs-pgo-build-v2', 'package': package,
                        'mode': d['mode'], 'identity': p.identity(d), 'target': p.TARGET,
                        'default_c_flags': False, 'source_free_claim': False, 'out_dir': str(out),
                        'compiler': d['compiler'], 'descriptor_sha256': p.digest(directory/'descriptor.json'),
                        'role':role, 'profile_scope':'runtime-only',
                        'recipe':d['recipes'][role], 'profile':d['profile'] if role == 'runtime' else None,
                        'compiler_invocation':None if package == 'incant_script' else 'cc',
                        'c_unit_profiled':None if package == 'incant_script' else role == 'runtime',
                        'actual_args': None if package == 'incant_script' else p.insert_flags(d['recipes'][role]['base_args'], p.additions(d, role))}
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
        path = Path(messages[-1]['out_dir'])/'quickjs-pgo.json'
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

    def test_real_data_packaging_excludes_mode_headers_but_binds_every_asset_byte(self):
        producer = self.root/'producer'; producer.mkdir()
        (producer/'world.scene').write_bytes(b'unchanged scene data')
        (producer/'input-actions.json').write_text('{}')
        models = self.root/'.incant/cache/models'; models.mkdir(parents=True)
        model = models/('a'*64+'.incmodel'); model.write_bytes(b'unchanged raw model payload')
        receipt = {'format':'incant-native-project-build','version':1,'tick_rate':60,
                   'scene_sha256':p.digest(producer/'world.scene'), 'audio_clips':{}, 'font_assets':[],
                   'visual_assets':[{'kind':'model','fingerprint':'a'*64}]}
        packages = []
        for mode in p.MODES:
            # These are real build outputs beside world.scene, embedded only in
            # the player/symbol artifacts. They are not data_package inputs.
            receipt['script'] = {'build_id':mode,'bytecode_abi':{'default_c_flags':mode=='ordinary'}}
            (producer/'behavior.rs').write_text('const MODE_HEADER: &str = '+repr(mode)+';')
            (producer/'script-symbols.json').write_text(json.dumps(receipt['script']))
            packages.append(data_package(self.root,producer,receipt,self.root/('data-'+mode),None))
        self.assertEqual(packages[0],packages[1]);self.assertEqual(packages[0],packages[2])
        self.assertEqual(packages[0]['files']['models/'+model.name],hashlib.sha256(model.read_bytes()).hexdigest())
        model.write_bytes(b'changed raw model payload')
        changed = data_package(self.root,producer,receipt,self.root/'data-changed',None)
        self.assertNotEqual(changed,packages[0], 'The strict cross-mode data equality must detect changed assets')


if __name__ == '__main__': unittest.main()


class RoleScope(unittest.TestCase):
    setUp = Protocol.setUp
    desc = Protocol.desc
    commands = Protocol.commands
    role_fixture = Protocol.role_fixture
    def test_profile_scope_preserves_cooker_O0_and_limits_flags_and_profile_to_runtime(self):
        d = self.desc()
        self.assertEqual(d['version'], 2)
        self.assertEqual(d['profile_scope'], 'runtime-only')
        self.assertEqual(p.additions(d, 'cooker'), [])
        self.assertTrue(p.additions(d, 'runtime'))
        rows = p.c_commands(self.commands(), self.target, self.vendor)
        self.assertEqual(p.ordinary_recipes(rows), self.recipes)
        for wrong in ({'runtime':self.recipes['runtime']},
                      dict(self.recipes, cooker=self.recipes['runtime'])):
            with self.assertRaises(ValueError):
                p.descriptor('use', self.desc_tools(d), wrong, self.vendor,
                             profile=self.root/'file.profdata')
        raw = self.commands(d)
        for bad in (raw.replace('-O0 -g', '-O3 -g'),
                    raw.replace('-O3 -g', '-O3 -isysroot /foreign -g')):
            with self.assertRaises(ValueError):
                p.c_commands(bad, self.target, self.vendor, expected=d)

    def desc_tools(self, d):
        return {k:d[k] for k in ('compiler','archiver','profdata_tool','profile_runtime',
                                 'sdk','resource_directory','tool_inputs')}

    def test_role_receipt_cannot_apply_runtime_profile_to_cooker_or_claim_cooker_instrumentation(self):
        d, directory, messages, producer = self.role_fixture()
        path = Path(messages[2]['out_dir'])/'quickjs-pgo.json'
        original = json.loads(path.read_text())
        for key, value in (('profile',d['profile']), ('c_unit_profiled',True),
                           ('recipe',d['recipes']['runtime']), ('profile_scope','all-roles')):
            data = dict(original, **{key:value}); path.write_text(json.dumps(data))
            attempt = self.root/(key+'-refusal'); attempt.mkdir()
            (attempt/'descriptor.json').write_bytes((directory/'descriptor.json').read_bytes())
            with self.assertRaises(ValueError):
                build.role_receipts(messages, self.target, d, attempt, producer)
        path.write_text(json.dumps(original))


class CompilerLookup(unittest.TestCase):
    setUp = Protocol.setUp
    desc = Protocol.desc
    commands = Protocol.commands
    def test_natural_spelling_resolves_only_against_actual_child_PATH(self):
        self.compiler.chmod(0o700)
        (self.root/'cc').symlink_to(self.compiler)
        desc = self.desc('generate')
        raw = self.commands(desc).replace(str(self.compiler), 'cc')
        with patch.dict(p.os.environ, {'PATH':'/wrong/ambient/path'}):
            rows = p.c_commands(raw, self.target, self.vendor, expected=desc, search_path=str(self.root))
        self.assertEqual(len(rows), 8)
        self.assertTrue(all(r['compiler']==str(self.compiler) for r in rows))
        for search in (None, '/wrong/child/path'):
            with self.assertRaises(ValueError):
                p.c_commands(raw, self.target, self.vendor, expected=desc, search_path=search)


class ObservedOrdering(unittest.TestCase):
    def test_exact_retained_cc_vector_places_additions_before_supported_warning(self):
        # CPU006 actual strict mismatch, retained unchanged outside the repo.
        actual = ['-O3',
         '-ffunction-sections',
         '-fdata-sections',
         '-fPIC',
         '-g',
         '-gdwarf-2',
         '-fno-omit-frame-pointer',
         '-mno-omit-leaf-frame-pointer',
         '--target=arm64-apple-macosx',
         '-mmacosx-version-min=27.0',
         '-Wall',
         '-Werror=unknown-warning-option',
         '-fprofile-generate=/Users/andreyyurlov/.codex/incant-evidence/quickjs-c-pgo-feasibility-001/core-cpu-006/raw',
         '-Wno-implicit-const-int-float-conversion',
         '-D_GNU_SOURCE']
        old_expected = ['-O3',
         '-ffunction-sections',
         '-fdata-sections',
         '-fPIC',
         '-g',
         '-gdwarf-2',
         '-fno-omit-frame-pointer',
         '-mno-omit-leaf-frame-pointer',
         '--target=arm64-apple-macosx',
         '-mmacosx-version-min=27.0',
         '-Wall',
         '-Wno-implicit-const-int-float-conversion',
         '-Werror=unknown-warning-option',
         '-fprofile-generate=/Users/andreyyurlov/.codex/incant-evidence/quickjs-c-pgo-feasibility-001/core-cpu-006/raw',
         '-D_GNU_SOURCE']
        base = [arg for arg in actual if arg not in ('-Werror=unknown-warning-option',)
                and not arg.startswith('-fprofile-generate=')]
        descriptor = {'mode':'generate', 'profile_scope':'runtime-only',
                      'raw_directory':actual[12].split('=',1)[1]}
        extra = p.additions(descriptor)
        self.assertNotEqual(actual, old_expected)
        self.assertEqual(p.insert_flags(base, extra), actual)
        self.assertEqual(p.insert_flags(base, []), base)
        self.assertEqual(p.insert_flags(base, p.additions(descriptor, 'cooker')), base)
        for bad in (old_expected, [*actual[:-1], '-DNDEBUG', actual[-1]],
                    [*actual[:11], *reversed(extra), *actual[13:]]):
            self.assertNotEqual(p.insert_flags(base, extra), bad)
