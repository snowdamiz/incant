#!/usr/bin/env python3
"""Public localization authoring, native Play and three source-free shipping replays."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import sys

SCRIPT, TABLE, ENTITY = [f'{i:026}' for i in (1, 2, 3)]
SOURCE = r"""
import type {NativeBehavior} from '@incant/runtime/native';
export default {update({chunks,localization:l,commands,clock,log}) {
  const table_id='00000000000000000000000002', tick=clock.tick;
  const locale=tick===1?'en':tick<=4?'fr':tick<=7?'ru':tick<=10?'de':'en';
  if(l.locale().locale!==locale)throw Error('locale publication');
  const text=l.localize({table_id,key:'hello',arguments:{name:'Ada'}}).text;
  const expected=tick>10?'[!! Ĥééľľóó Ada !!]':locale==='fr'||locale==='de'?'Bonjour Ada':'Hello Ada';
  if(text!==expected)throw Error('localized text');
  if(tick===1){
    if(l.localize({table_id,key:'party',arguments:{gender:'female',n:3}}).text!=='She and 2 guests')throw Error('select/plural');
    if(l.localize({table_id,key:'rank',arguments:{n:22}}).text!=='22nd')throw Error('ordinal');
    if(l.formatDate({year:2024,month:1,day:2})!=='Jan 2, 2024')throw Error('ICU date');
  }
  if(locale==='ru'&&l.localize({table_id,key:'items',arguments:{n:22}}).text!=='22 предмета')throw Error('Russian plural');
  if(locale==='de'&&l.formatNumber(1234.5)!=='1.234,5')throw Error('ICU number');
  if(l.localize({table_id,key:'absent'}).missing?.kind!=='key')throw Error('missing key');
  const report=l.report({limit:1});if(report.entries.length>1||(locale!=='en'&&report.total!==4-(locale==='fr'||locale==='ru'?1:0)))throw Error('missing report');
  if(tick===1)commands.setLocale({locale:'fr'});
  if(tick===4)commands.setLocale({locale:'ru'});
  if(tick===7)commands.setLocale({locale:'de',fallbacks:['fr']});
  if(tick===10)commands.setLocale({locale:'en',pseudo:true});
  if(l.locale().locale!==locale)throw Error('uncommitted locale leaked');
  for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=text.length;
  log(text);
}} satisfies NativeBehavior;
"""
MESSAGES = {
    'hello': {'en': 'Hello {name}', 'fr': 'Bonjour {name}'},
    'items': {'en': '{n, plural, one{# item} other{# items}}',
              'ru': '{n, plural, one{# предмет} few{# предмета} many{# предметов} other{# предмета}}'},
    'party': {'en': '{gender, select, female{{n, plural, offset:1 =1{She came alone} one{She and # guest} other{She and # guests}}} other{They came}}'},
    'rank': {'en': '{n, selectordinal, one{#st} two{#nd} few{#rd} other{#th}}'},
}
EXPECTED = ['Hello Ada'] + ['Bonjour Ada'] * 3 + ['Hello Ada'] * 3 + ['Bonjour Ada'] * 3 + ['[!! Ĥééľľóó Ada !!]'] * 2


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--author', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    root, output = Path(__file__).resolve().parents[2], args.output.resolve()
    sys.path.insert(0, str(root / 'tools'))
    import native_process
    import export_native
    module = runpy.run_path(str(root / 'tools/probes/script-source-transactions.py'))
    require, Rpc = module['require'], module['Rpc']
    binary = args.binary.resolve(strict=True)
    inputs = output / 'authoring'
    project, journal = inputs / 'project.json', inputs / 'journal.jsonl'

    def execute(argv, **kwargs):
        result = native_process.run(argv, **kwargs)
        require(result.returncode == 0, result.stderr[-8000:])
        return result

    def write(path, value):
        path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')

    def digest(path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    if args.author:
        document = json.loads(project.read_text(encoding='utf-8'))
        scene, rate = next(iter(document['scenes'])), document['settings']['tick_rate']
        rpc = Rpc(binary, project, journal)
        try:
            created = rpc.call('command.execute', {'expected_revision': 0,
                'description': 'Author native localization replay', 'commands': [
                    {'op': 'write_script', 'script_id': SCRIPT, 'expected_source': {'state': 'missing'}, 'source': SOURCE},
                    {'op': 'upsert_script', 'script': {'id': SCRIPT, 'name': 'Localization', 'path': 'localization.ts'}},
                    {'op': 'upsert_string_table', 'table': {'id': TABLE, 'name': 'Game', 'source_locale': 'en', 'messages': MESSAGES}},
                    {'op': 'set_locale', 'settings': {'locale': 'en'}},
                    {'op': 'create_entity', 'scene_id': scene, 'entity': {'id': ENTITY, 'name': 'Localized actor',
                        'parent': None, 'provenance': None, 'components': {
                            'Transform': {'translation': [0, 0, 0], 'rotation': [0, 0, 0, 1], 'scale': [1, 1, 1]},
                            'Script': {'source': SCRIPT, 'props': {}}}}}]})
            require(created['revision'] == 1, 'Creation was not one transaction')
            authored = rpc.call('project.read')
            result = rpc.call('play.run', {'seconds': 12 / rate})
            require(result['passed'] and result['state']['tick'] == 12, 'Native localization Play failed')
            value = rpc.call('play.state_query', {'path': f'/entities/{ENTITY}/transform/translation'})
            require(value['untrusted_runtime_data'][0] == sum(map(len, EXPECTED)), 'Wrong localized motion')
            require(rpc.call('project.read') == authored, 'Locale commands changed authoring')
            require(rpc.call('project.save')['saved'], 'Checkpoint was not saved')
        finally:
            rpc.close()
        rpc = Rpc(binary, project, journal)
        try:
            require(rpc.call('project.read') == authored, 'Reopened checkpoint/journal differs')
            require(rpc.call('script.read', {'script_id': SCRIPT})['untrusted_source_data']['source'] == SOURCE, 'Reopened source differs')
        finally:
            rpc.close()
        write(output / 'authoring-result.json', {'native_play_x': value['untrusted_runtime_data'][0],
            'binary_sha256': digest(binary), 'project_sha256': digest(project), 'script_sha256': digest(inputs / 'localization.ts'),
            'authoring_unchanged': True, 'checkpoint_and_journal_reopened': True})
        return
    output.mkdir(parents=True, exist_ok=False)
    inputs.mkdir()
    execute([str(binary), 'init', str(project), '--entities', '0'])
    execute([sys.executable, str(Path(__file__).resolve()), str(output), '--binary', str(binary), '--author'], timeout=180)
    authored = json.loads((output / 'authoring-result.json').read_text(encoding='utf-8'))
    report = export_native.export(project, output / 'package')
    require(report['project_sha256'] == authored['project_sha256'], 'Export used a different checkpoint')
    shutil.rmtree(inputs)  # Only the newly created private fixture, after packaging.
    player = output / 'package/player' / ('incant_game_player.exe' if os.name == 'nt' else 'incant_game_player')
    plan = output / 'observe.json'
    write(plan, {'format': 'incant-native-observation', 'version': 1, 'entities': [ENTITY],
        'checks': [{'name': 'localized-motion', 'tick': 12, 'expect': {'type': 'vector', 'entity': ENTITY,
            'property': 'translation', 'value': [sum(map(len, EXPECTED)), 0, 0], 'tolerance': 1e-9}}]})
    env = {'PATH': str(player.parent), 'HOME': str(output / 'absent-home'),
           'TMPDIR': str(output), 'TMP': str(output), 'TEMP': str(output)}
    env.update({k: os.environ[k] for k in ('SystemRoot', 'SystemDrive', 'WINDIR') if k in os.environ})
    runs = []
    for index in range(3):
        result = execute([str(player), '--package', str(player.parent / 'data'), '--ticks', '12',
            '--output', str(output / f'run-{index}.pcm'), '--observe', str(plan)], cwd=player.parent, env=env, timeout=60)
        value = json.loads(result.stdout)
        require(value['observations']['passed'], 'Exported localization motion differs')
        require([entry['message'] for entry in value['logs']['entries']] == EXPECTED, 'Exported localization messages differ')
        write(output / f'run-{index}.json', value)
        runs.append(value)
    require(runs[0] == runs[1] == runs[2], 'Fresh replay reports differ')
    write(output / 'result.json', {'passed': True, 'authoring': authored, 'fresh_runs': len(runs), 'ticks_each': 12,
        'expected_messages': EXPECTED, 'pcm_sha256': runs[0]['pcm_sha256'], 'authoring_deleted': not inputs.exists(),
        'build_storage_removed': report['build_storage_removed_before_startup'], 'player_sha256': report['player_sha256'],
        'target': report['target'], 'limitations': ['CPU functional proof', 'No GPU, reference phone, performance or phase gate']})
    print((output / 'result.json').read_text(encoding='utf-8'))


if __name__ == '__main__':
    main()
