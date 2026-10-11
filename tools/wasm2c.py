#!/usr/bin/env python3
"""Prepare trusted build inputs for static wasm2c embedding (never runtime code).

Requires the pinned WABT tool and the built incant_wasm_cook validator/meter.
This is not an arbitrary Rust build service or a sandbox for native toolchains.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from wasm_abi import ABI_VERSION, NAMESPACES


def run(args):
    return subprocess.check_output([str(x) for x in args], text=True, encoding='utf-8')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wrapper(module, imports):
    # Parameter order comes from the pinned translator's actual declaration, not
    # import encounter order or an assumed module-name sort.
    if isinstance(imports, bool): imports = ['w2c_incant__v1'] if imports else []
    fields = NAMESPACES
    if any(name not in fields for name in imports): raise ValueError('Unsupported translated import namespace')
    instantiate = f'wasm2c_{module}_instantiate(&p->guest' + ''.join(', &p->'+fields[name] for name in imports) + ');'
    contexts = ' '.join(f'struct {namespace} {field};' for namespace,field in fields.items())
    copies = '\n'.join(f'  p->{field}.core=p->imports;' for field in fields.values() if field!='imports')
    clear = '\n'.join(f'  p->{field}'+('' if field=='imports' else '.core')+'.callback=NULL; p->'+field+('' if field=='imports' else '.core')+'.context=NULL;' for field in fields.values())
    bind = '\n'.join(f'  p->{field}'+('' if field=='imports' else '.core')+'.callback=callback; p->'+field+('' if field=='imports' else '.core')+'.context=context;' for field in fields.values())
    return f'''#include "{module}.h"
#include "imports.h"
#include <stdlib.h>
typedef struct {{ w2c_{module} guest; {contexts} }} Instance;
int32_t incant_{module}_create(incant_import callback,void* context,void** output) {{
  *output=NULL;
  Instance* p=calloc(1,sizeof(Instance));
  if(!p) return -1;
  p->imports=(struct w2c_incant__v1){{callback,context,w2c_{module}_memory(&p->guest)}};
{copies}
  wasm_rt_init();
  /* setjmp target and every potentially trapping guest frame stay in C. */
  int32_t status=0;
  incant_trap_prepare();
  if(incant_try()==0) {{
    {instantiate}
    if(w2c_{module}_incant_abi(&p->guest)!= {ABI_VERSION}) status=-2;
  }} else {{ status=*w2c_{module}_incant_private_fuel(&p->guest)==0 ? -3 : 1; }}
  g_wasm_rt_jmp_buf.initialized=false;
{clear}
  if(status) {{ wasm2c_{module}_free(&p->guest); free(p); return status; }}
  *output=p; return 0;
}}
int32_t incant_{module}_update(void* instance,incant_import callback,void* context,int64_t epoch,double dt) {{
  Instance* p=instance;
{bind}
  *w2c_{module}_incant_private_fuel(&p->guest)=10000000;
  int32_t status=0;
  incant_trap_prepare();
  if(incant_try()==0) w2c_{module}_update(&p->guest,(uint64_t)epoch,dt);
  else status=*w2c_{module}_incant_private_fuel(&p->guest)==0 ? -3 : 1;
  g_wasm_rt_jmp_buf.initialized=false;
{clear}
  return status;
}}
void incant_{module}_free(void* instance) {{ Instance* p=instance; wasm2c_{module}_free(&p->guest); free(p); }}
'''


def prepare(output, modules, wabt, cook):
    translator = wabt / ('bin/wasm2c.exe' if os.name == 'nt' else 'bin/wasm2c')
    if run([translator, '--version']).strip() != '1.0.42':
        raise ValueError('WABT 1.0.42 required')
    output.mkdir(parents=True, exist_ok=False)
    report = []
    bindings = ['// Generated trusted static descriptors. Do not load from project data.',
                'use crate::{Import, ModuleExports};', 'use std::ffi::c_void;']
    names = set()
    for index, (name, source) in enumerate(modules):
        if not re.fullmatch(r'[a-z][a-z0-9-]{0,63}', name) or name in names:
            raise ValueError('Invalid/duplicate static module name')
        names.add(name)
        if source.stat().st_size > 1024*1024:
            raise ValueError('Module input size limit')
        module = f'm{index}'
        (output / f'{module}.original.wasm').write_bytes(source.read_bytes())
        prepared = output / f'{module}.wasm'
        run([cook, source, prepared])
        run([translator, prepared, '--module-name', module, '--no-debug-names',
             '--disable-exceptions', '--disable-tail-call', '--disable-memory64',
             '--disable-multi-memory', '--disable-simd', '--disable-relaxed-simd',
             '-o', output / f'{module}.c'])
        header = (output / f'{module}.h').read_text()
        declaration = re.search(r'void wasm2c_'+module+r'_instantiate\(([^;]+)\);', header, re.DOTALL)
        if not declaration: raise ValueError('Missing pinned instantiate declaration')
        imports = re.findall(r'struct (w2c_\w+)\s*\*', declaration.group(1))
        (output / f'{module}-facade.c').write_text(wrapper(module, imports))
        bindings.append(f'''unsafe extern "C" {{
 fn incant_{module}_create(cb:Import,ctx:*mut c_void,out:*mut *mut c_void)->i32;
 fn incant_{module}_update(p:*mut c_void,cb:Import,ctx:*mut c_void,e:i64,dt:f64)->i32;
 fn incant_{module}_free(p:*mut c_void);
}}''')
        report.append({'name': name, 'prefix': module, 'input_sha256': sha(source),
                       'prepared_sha256': sha(prepared)})
    bindings.append('pub fn linked(name: &str) -> Option<ModuleExports> { match name {')
    for row in report:
        m = row['prefix']
        bindings.append(f'''"{row['name']}" => Some(unsafe {{ ModuleExports::from_linked(incant_{m}_create, incant_{m}_update, incant_{m}_free) }}),''')
    bindings.append('_ => None } }')
    (output / 'linked.rs').write_text('\n'.join(bindings) + '\n')
    report = {'wabt': '1.0.42', 'modules': report,
              'files': {p.name: sha(p) for p in sorted(output.iterdir()) if p.is_file()}}
    (output / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--module', action='append', required=True, help='NAME=trusted WASM file')
    parser.add_argument('--wabt', type=Path, required=True)
    parser.add_argument('--cook', type=Path, required=True)
    args = parser.parse_args()
    modules = [(name, Path(path)) for name, path in (spec.split('=', 1) for spec in args.module)]
    print(json.dumps(prepare(args.output, modules, args.wabt, args.cook), indent=2))


if __name__ == '__main__':
    main()
