"""Bounded runtime-log controls on the existing fresh source-free game player."""
import json

ACTIONS = [('log_count', 'KeyL'), ('log_bytes', 'KeyB'), ('log_invalid', 'KeyI'),
           ('log_message', 'KeyM'), ('log_calls', 'KeyN'), ('log_total', 'KeyV')]
BEHAVIOR = '''
const beforeLogs = exports.default.update;
exports.default.update = function(ctx) {
    beforeLogs(ctx);
    const {input, clock, log} = ctx;
    if (input.actions.log_count?.active) {
        for(let i=0;i<64;i++)log('bounded');
    } else if (input.actions.log_bytes?.active) {
        for(let i=0;i<16;i++)log('x'.repeat(1024));
    } else {
        if(clock.tick===1)log('Ready 雪🙂');
        if(clock.tick===120)log('Finished','warn');
    }
    // Catching guest errors cannot commit the invalid transaction or its logs.
    try {
        if(input.actions.log_invalid?.pressed)log({toString(){return 'invalid'}});
        if(input.actions.log_message?.pressed)log('x'.repeat(1025));
        if(input.actions.log_calls?.pressed)for(let i=0;i<65;i++)log('');
        if(input.actions.log_total?.pressed)for(let i=0;i<17;i++)log('x'.repeat(1024));
    } catch (_) {}
};
'''
BASELINE = {'entries': [{'tick': 1, 'level': 'info', 'message': 'Ready 雪🙂'},
                        {'tick': 120, 'level': 'warn', 'message': 'Finished'}], 'dropped': 0}


def verify(out, command, baseline, replay, run, reject, require):
    reports = []
    for name, code, retained, calls, message in [
            ('log_count', 'KeyL', 1024, 64, 'bounded'),
            ('log_bytes', 'KeyB', 256, 16, 'x'*1024)]:
        path = out/f'{name}.json'
        path.write_text(json.dumps(dict(replay, frames=[{'tick': 1, 'events': [
            {'type': 'key', 'code': 'KeyD', 'down': True},
            {'type': 'key', 'code': code, 'down': True}]}])))
        dest = out/f'{name}.pcm'
        report = json.loads(run(command(path,dest)))
        logs = report['logs']
        require(len(logs['entries']) == retained and logs['dropped'] == 120*calls-retained,
                'Log history cap or drop count incorrect')
        require(all(row == {'tick': i//calls+1, 'level': 'info', 'message': message}
                    for i,row in enumerate(logs['entries'])), 'Retained prefix changed')
        require(all(report[k] == baseline[k] for k in ('tick','frames','sources','pcm_sha256')),
                'History cap changed simulation/audio')
        reports.append({'mode': name, 'retained': retained, 'dropped': logs['dropped'],
                        'pcm_sha256': report['pcm_sha256']})
    for name, code in ACTIONS[2:]:
        path = out/f'{name}.json'
        path.write_text(json.dumps(dict(replay, frames=[{'tick': 2, 'events': [
            {'type': 'key', 'code': code, 'down': True}]}])))
        dest = out/f'{name}.pcm'
        reject(name,command(path,dest),dest)
    return {'passed': True, 'retention_controls': reports, 'invalid_update_controls': 4,
            'json_serialization': 'once at completed game report, never per tick'}
