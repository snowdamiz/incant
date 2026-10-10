#!/usr/bin/env python3
"""Exercise XLIFF through public CLI/RPC, durable history and strict TypeScript gameplay."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
NS = 'urn:oasis:names:tc:xliff:document:2.0'
ET.register_namespace('', NS)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()
    binary_hash = hashlib.sha256(binary.read_bytes()).hexdigest()

    def run(*args):
        return json.loads(subprocess.check_output([str(binary), *map(str, args)], text=True, encoding='utf-8', cwd=ROOT))

    project, journal = out / 'game.incant.json', out / 'game.incant.journal.jsonl'
    run('init', project, '--name', 'Translator exchange', '--entities', 0)
    ids = ['00000000000000000000000010', '00000000000000000000000011']
    tables = [
        {'id': ids[0], 'name': 'Game UI', 'source_locale': 'en', 'messages': {
            'coins': {'en': '{n, plural, one {# coin} other {# coins}}'},
            'hello': {'en': ' Hello {name} & <world>!\n\t🙂 '},
            'empty': {'en': 'Hide this label'},
            'missing': {'en': 'Fallback remains'}}},
        {'id': ids[1], 'name': 'Menu', 'source_locale': 'en', 'messages': {'start': {'en': 'Start'}}},
    ]

    def rpc(requests):
        output = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
                  input=''.join(json.dumps(r) + '\n' for r in requests), text=True, encoding='utf-8', cwd=ROOT)
        replies = [json.loads(line) for line in output.splitlines()]
        assert len(replies) == len(requests) and all('error' not in r for r in replies), replies
        return replies

    rpc([{'id': 1, 'method': 'command.execute', 'params': {
        'commands': [{'op': 'upsert_string_table', 'table': table} for table in tables],
        'expected_revision': 0, 'description': 'Author translator source tables'}},
        {'id': 2, 'method': 'project.save'}])
    authored = project.read_bytes()
    originals = []
    for i, table_id in enumerate(ids):
        destination = out / f'source-{i}.xlf'
        run('localization-export', project, table_id, 'ja', destination)
        originals.append(ET.parse(destination).getroot())
    assert project.read_bytes() == authored
    translations = {'coins': '{n}枚', 'hello': ' こんにちは {name} & <世界>!\n\t🙂 ', 'empty': '', 'start': '開始'}
    combined = originals[0]
    combined.append(originals[1].find(f'{{{NS}}}file'))
    # A CAT tool may preserve arbitrary source-name metadata. It never becomes a host path.
    for file in combined.findall(f'{{{NS}}}file'):
        file.set('original', '../../ignored-by-import.incant.json')
    for unit in combined.iter(f'{{{NS}}}unit'):
        if unit.get('id') in translations:
            segment = unit.find(f'{{{NS}}}segment')
            segment.set('state', 'final')
            target = ET.SubElement(segment, f'{{{NS}}}target')
            target.set('{http://www.w3.org/XML/1998/namespace}space', 'preserve')
            target.text = translations[unit.get('id')]
    translated = out / 'translated.xlf'
    translated.write_bytes(ET.tostring(combined, encoding='utf-8', xml_declaration=True))

    def unchanged_after_failure(arguments):
        before = {p: p.read_bytes() for p in [project, journal]}
        result = subprocess.run([str(binary), *map(str, arguments)], capture_output=True, text=True, encoding='utf-8', cwd=ROOT)
        assert result.returncode != 0 and result.stderr, result
        assert all(p.read_bytes() == data for p, data in before.items())

    unchanged_after_failure(['localization-export', project, ids[0], 'ja', project])
    invalid = out / 'invalid.xlf'
    invalid.write_text(translated.read_text(encoding='utf-8').replace('開始', '{unknown}'), encoding='utf-8')
    unchanged_after_failure(['localization-import', project, invalid])
    invalid.write_text(translated.read_text(encoding='utf-8').replace('Start', 'Changed source'), encoding='utf-8')
    unchanged_after_failure(['localization-import', project, invalid])
    imported = run('localization-import', project, translated)
    assert imported['changed_messages'] == 4 and imported['changed_tables'] == 2
    assert imported['transaction_id'] and imported['revision'] == 2
    translated_project = project.read_bytes()
    data = json.loads(translated_project)
    assert data['string_tables'][ids[0]]['messages']['empty']['ja'] == ''
    assert 'ja' not in data['string_tables'][ids[0]]['messages']['missing']
    before_journal = journal.read_bytes()
    repeated = run('localization-import', project, translated)
    assert repeated['transaction_id'] is None and repeated['changed_tables'] == 0
    assert journal.read_bytes() == before_journal and project.read_bytes() == translated_project
    rpc([{'id': 1, 'method': 'history.undo'}, {'id': 2, 'method': 'project.save'}])
    assert project.read_bytes() == authored
    rpc([{'id': 1, 'method': 'history.redo'}, {'id': 2, 'method': 'project.save'}])
    assert project.read_bytes() == translated_project
    source = out / 'behavior.ts'
    source.write_text('import type {ScriptApi} from ' + json.dumps((ROOT / 'sdk/ts/src/index').as_posix()) + ';\n'
        + 'const table_id=' + json.dumps(ids[0]) + ';\n' + '''
export default defineBehavior<{texts:string[]}>({
  initialState:{texts:[]},
  update(api:ScriptApi,_dt,s){
    if(api.clock().tick===1)api.command({op:'set_locale',settings:{locale:'ja'}});
    if(api.clock().tick===2){
      s.texts.push(api.localize({table_id,key:'coins',arguments:{n:2}}).text);
      s.texts.push(api.localize({table_id,key:'hello',arguments:{name:'海'}}).text);
      s.texts.push(api.localize({table_id,key:'empty'}).text);
      s.texts.push(api.localize({table_id,key:'missing'}).text);
    }
  }
});
''', encoding='utf-8')
    subprocess.run(['node', str(ROOT / 'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
                    '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], check=True, cwd=ROOT)
    compiled = out / 'behavior.js'
    subprocess.run(['node', str(ROOT / 'tools/build_script.mjs'), str(source), str(compiled)], check=True, cwd=ROOT)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    played = run('play', project, '--ticks', 2, '--compiled-script', compiled)
    assert played['script_state']['texts'] == ['2枚', ' こんにちは 海 & <世界>!\n\t🙂 ', '', 'Fallback remains']
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    assert binary_hash == hashlib.sha256(binary.read_bytes()).hexdigest(), 'binary changed during probe'
    result = {'passed': True, 'binary_sha256': binary_hash, 'changed_messages': 4, 'changed_tables': 2,
              'one_atomic_transaction': True, 'invalid_suffix_and_stale_source_rollback': True,
              'no_clobber_export': True, 'no_op_history_unchanged': True, 'durable_undo_redo_exact': True,
              'empty_and_missing_targets_distinguished': True, 'strict_typescript': True,
              'runtime_texts': played['script_state']['texts'], 'author_files_unchanged': before,
              'rendered_text_review': False}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
