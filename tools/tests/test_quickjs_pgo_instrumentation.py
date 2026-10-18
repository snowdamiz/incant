"""Inert Mach-O bytes prove audit stages; no compiler, tool or player execution."""
import copy
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import quickjs_pgo as p
import quickjs_pgo_build as build
from quickjs_pgo_image import PROFILE_APIS, PROFILE_SECTIONS


class Instrumentation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name).resolve()
        self.before, self.packaged = root/'unstripped', root/'packaged'
        names = ('__mod_init_func', *PROFILE_SECTIONS)
        sizes = [152, 72+80*len(names)]
        commands_size = 24+sum(sizes)
        data_offset = 32+commands_size
        uuid = struct.pack('<II', 0x1b, 24)+bytes(range(16))
        text = struct.pack('<II16sQQQQiiII', 0x19, sizes[0], b'__TEXT', 0x1000, 64, data_offset, 64, 7, 5, 1, 0)
        text += struct.pack('<16s16sQQIIIIIIII', b'__text', b'__TEXT', 0x1000, 64, data_offset, 0, 0, 0, 0, 0, 0, 0)
        data = struct.pack('<II16sQQQQiiII', 0x19, sizes[1], b'__DATA', 0x2000, len(names)*16,
                           data_offset+64, len(names)*16, 7, 3, len(names), 0)
        for index, name in enumerate(names):
            data += struct.pack('<16s16sQQIIIIIIII', name.encode(), b'__DATA', 0x2000+index*16, 16,
                                data_offset+64+index*16, 0, 0, 0, 0, 0, 0, 0)
        header = struct.pack('<8I', 0xfeedfacf, 0x100000c, 0, 2, 3, commands_size, 0, 0)
        self.bytes = header+uuid+text+data+bytes(range(64))+b'count-data-block'*len(names)+bytes(len(names))
        self.before.write_bytes(self.bytes+b'local-symbol-table')
        self.packaged.write_bytes(self.bytes)
        self.symbols = '\n'.join(f'{0x1000+index*16:016x} t {name}' for index, name in enumerate(PROFILE_APIS))
        self.symbols += '\n0000000000002000 s ___llvm_profile_runtime\n'
        self.section_output = '\n'.join('Name: '+name+' (synthetic)' for name in PROFILE_SECTIONS)

    def admit(self, proof=None, sections=None):
        return p.instrumentation('generate', 'public_symbols_only',
            self.section_output if sections is None else sections,
            before=p.before_strip(self.before,self.symbols) if proof is None else proof, packaged=self.packaged)

    def test_local_symbol_stripping_preserves_exact_code_initializer_counters_and_stages(self):
        proof = self.admit()
        self.assertTrue(proof['identical_code_initializer_and_profile_sections'])
        self.assertNotEqual(proof['symbol_artifact']['sha256'],proof['packaged_artifact']['sha256'])
        self.assertEqual(proof['symbol_stage'],'unstripped-generation')
        self.assertEqual(proof['section_stage'],'packaged-generation')
        self.assertNotEqual(proof['symbols_sha256'],proof['packaged_symbols_sha256'])

    def test_Clang21_empty_optional_sections_remain_exact_but_empty_counters_refuse(self):
        for path in (self.before,self.packaged):
            data=bytearray(path.read_bytes())
            for name in ('__llvm_prf_bits','__llvm_prf_vns','__llvm_prf_vtab'):
                struct.pack_into('<Q',data,data.index(name.encode())+40,0)
            path.write_bytes(data)
        self.admit()
        for path in (self.before,self.packaged):
            data=bytearray(path.read_bytes())
            struct.pack_into('<Q',data,data.index(b'__llvm_prf_cnts')+40,0)
            path.write_bytes(data)
        with self.assertRaises(ValueError):self.admit()

    def test_missing_renamed_duplicate_or_out_of_code_profile_API_refuses(self):
        for bad in (self.symbols.replace(PROFILE_APIS[0],'unrelated_API'),
                    self.symbols+'0000000000001000 t '+PROFILE_APIS[0],
                    self.symbols.replace('0000000000001000','0000000000009000'),
                    self.symbols.replace('___llvm_profile_runtime','unrelated_runtime')):
            with self.subTest(symbols=bad),self.assertRaises(ValueError):p.before_strip(self.before,bad)

    def test_wrong_stage_changed_source_artifact_or_identity_refuses(self):
        proof=p.before_strip(self.before,self.symbols)
        for change in (lambda x:x.update(stage='packaged-generation'),
                       lambda x:x['image'].update(sha256='0'*64),
                       lambda x:x['profile_API_addresses'].update({PROFILE_APIS[0]:0})):
            bad=copy.deepcopy(proof);change(bad)
            with self.assertRaises(ValueError):self.admit(bad)
        self.before.write_bytes(self.before.read_bytes()+b'changed after audit')
        with self.assertRaises(ValueError):self.admit(proof)

    def test_removed_or_changed_packaged_code_counter_UUID_or_section_inspection_refuses(self):
        original=self.packaged.read_bytes()
        counter=original.index(b'count-data-block')+16
        text=original.index(bytes(range(64)))
        for bad in (original.replace(b'__llvm_prf_cnts',b'__missing_cnts_'),
                    original[:text]+bytes([original[text]^1])+original[text+1:],
                    original[:counter]+bytes([original[counter]^1])+original[counter+1:],
                    original[:40]+bytes([original[40]^1])+original[41:]):
            self.packaged.write_bytes(bad)
            with self.assertRaises(ValueError):self.admit()
        self.packaged.write_bytes(original)
        with self.assertRaises(ValueError):self.admit(sections=self.section_output.replace('__llvm_prf_cnts','wrong'))
        with self.assertRaises(ValueError):p.instrumentation('generate',self.symbols,self.section_output)

    def test_ordinary_use_still_refuse_profile_symbols_or_sections_and_generation_proof(self):
        for mode in ('ordinary','use'):
            p.instrumentation(mode,'plain symbols','plain sections')
            for symbols, sections in ((self.symbols,'plain'),('plain',self.section_output)):
                with self.assertRaises(ValueError):p.instrumentation(mode,symbols,sections)
            with self.assertRaises(ValueError):
                p.instrumentation(mode,'plain','plain',before=p.before_strip(self.before,self.symbols))

    def test_real_generation_hook_captures_before_strip_and_restores_ordinary_hook(self):
        symbols=self.symbols+'\n'+'\n'.join('0000000000001000 T '+name for name in
            (*p.RETAINED,'_JS_ReadObject','_JS_EvalFunction','___JS_EvalInternal'))
        prior=build.export_native.runtime_symbol_audit
        with patch.object(build.export_native,'execute',return_value=subprocess.CompletedProcess([],0,symbols,'')) as call:
            with build.phase('generate') as audit:
                result=build.export_native.runtime_symbol_audit(self.before,'dsym',self.before.parent,None)
                self.assertFalse(result['source_free_claim'])
                self.assertEqual(audit['before_strip']['image']['path'],str(self.before))
                self.assertEqual(audit['before_strip']['symbols'],symbols)
            self.assertIs(build.export_native.runtime_symbol_audit,prior)
            call.assert_called_once()


if __name__ == '__main__':
    unittest.main()
