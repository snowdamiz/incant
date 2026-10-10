#!/usr/bin/env python3
"""Verify localization using shared RPC, strict TypeScript and process save/reload."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/release/incant_headless')
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = args.binary.resolve()

    def run(*args):
        return json.loads(subprocess.check_output([str(binary), *map(str, args)], text=True, cwd=ROOT))

    project, journal = out / 'game.incant.json', out / 'history.jsonl'
    run('init', project, '--name', 'Runtime localization', '--entities', 0)
    table_id = '00000000000000000000000010'
    table = {'id': table_id, 'name': 'Game UI', 'source_locale': 'en', 'messages': {
        'coins': {'en': '{n, plural, one {# coin} other {# coins}}',
                  'ru': '{n, plural, one {# монета} few {# монеты} many {# монет} other {# монеты}}',
                  'ja': '{n}枚', 'ar': '{n} عملات'},
        'hello': {'en': 'Hello {name}', 'ja': 'こんにちは {name}', 'ar': 'مرحبا {name}'},
        'gender': {'en': '{gender, select, female {She} male {He} other {They}} won.'},
    }}

    def rpc(requests):
        result = subprocess.check_output([str(binary), 'rpc', str(project), '--journal', str(journal)],
                    input=''.join(json.dumps(r) + '\n' for r in requests), text=True, cwd=ROOT)
        replies = [json.loads(line) for line in result.splitlines()]
        assert len(replies) == len(requests) and all('error' not in r for r in replies), replies
        return replies

    rpc([{'id': 1, 'method': 'command.execute', 'params': {'commands': [{'op': 'upsert_string_table', 'table': table}],
         'expected_revision': 0, 'description': 'Author localized UI'}}, {'id': 2, 'method': 'project.save'}])
    authored = project.read_bytes()
    rpc([{'id': 1, 'method': 'history.undo'}, {'id': 2, 'method': 'history.redo'}, {'id': 3, 'method': 'project.save'}])
    assert project.read_bytes() == authored
    assert run('localization-check', project)['complete'] is True
    missing = subprocess.run([str(binary), 'localization-check', str(project), '--locale', 'ja'], capture_output=True, text=True)
    assert missing.returncode != 0
    assert [item['key'] for item in json.loads(missing.stdout)['missing']] == ['gender']
    assert run('localization-check', project, '--locale', 'ja', '--allow-fallback')['complete'] is False
    source = out / 'behavior.ts'
    source.write_text('import type { ScriptApi } from ' + json.dumps((ROOT / 'sdk/ts/src/index').as_posix()) + ';\n' +
                      'const table_id=' + json.dumps(table_id) + ';\n' + '''
export default defineBehavior<{texts:string[],locales:string[],resolved:string[],gender:string[]}>({
  initialState:{texts:[],locales:[],resolved:[],gender:[]},
  update(api:ScriptApi,_dt,s){
    const locale=api.locale();
    const value=api.localize({table_id,key:'coins',arguments:{n:2}});
    s.locales.push(locale.locale);s.texts.push(value.text);s.resolved.push(value.resolved_locale??'');
    s.gender.push(api.localize({table_id,key:'gender',arguments:{gender:'female'}}).text);
    if(api.clock().tick===1)api.setTimer({id:'switch',delay_ticks:1,interval_ticks:2});
    api.log(api.formatNumber(1234.5));
  },
  onTimer(api){
    const choices:Record<number,string>={2:'ru',4:'ja',6:'ar',8:'fr-CA'};
    const locale=choices[api.clock().tick];
    if(locale)api.command({op:'set_locale',settings:{locale,fallbacks:['en']}});
    if(api.clock().tick===8)api.cancelTimer('switch');
  }
});
''')
    subprocess.run(['node', str(ROOT / 'node_modules/typescript/bin/tsc'), '--strict', '--noEmit', '--target', 'ES2022',
                    '--module', 'ESNext', '--moduleResolution', 'bundler', str(source)], check=True, cwd=ROOT)
    compiled = out / 'behavior.js'
    subprocess.run(['node', str(ROOT / 'tools/build_script.mjs'), str(source), str(compiled)], check=True, cwd=ROOT)
    before = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project, journal]}
    first = run('play', project, '--ticks', 5, '--compiled-script', compiled, '--save-output', out / 'one.save.json')
    resumed = run('play', project, '--ticks', 5, '--compiled-script', compiled, '--load-save', out / 'one.save.json',
                  '--save-output', out / 'two.save.json')
    whole = run('play', project, '--ticks', 10, '--compiled-script', compiled)
    reopened = run('play', project, '--ticks', 0, '--compiled-script', compiled, '--load-save', out / 'two.save.json')
    assert resumed['script_state'] == whole['script_state'] == reopened['script_state']
    assert resumed['state'] == whole['state'] == reopened['state']
    state = whole['script_state']
    assert state['texts'] == ['2 coins'] * 2 + ['2 монеты'] * 2 + ['2枚'] * 2 + ['2 عملات'] * 2 + ['2 coins'] * 2
    assert state['locales'] == ['en'] * 2 + ['ru'] * 2 + ['ja'] * 2 + ['ar'] * 2 + ['fr-CA'] * 2
    assert state['resolved'][-2:] == ['en','en']
    assert state['gender'] == ['She won.'] * 10
    assert resumed['logs'] == [log for log in whole['logs'] if log['tick'] > 5]
    assert json.loads((out / 'two.save.json').read_text())['project']['settings']['localization']['locale'] == 'fr-CA'
    assert before == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [project,journal]}
    result = {'passed': True, 'strict_typescript': True, 'rpc_journal_undo_redo': True,
              'runtime_locales': ['en','ru','ja','ar','fr-CA'], 'timer_switches_saved': True,
              'restored_state_and_log_suffix_equal': True, 'missing_translation_exit_status': True,
              'author_files_unchanged': before, 'shaping_or_visual_acceptance': False,
              'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    (out / 'runs.json').write_text(json.dumps({'first':first,'resumed':resumed,'whole':whole,'reopened':reopened},indent=2)+'\n')
    (out / 'result.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
