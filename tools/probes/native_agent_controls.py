"""Source-deleted native agent request controls on the real headless RPC host."""
import json


def exercise(run, project, digest, entity, rate, out):
    def require(ok, reason):
        if not ok:
            raise ValueError(reason)
    baseline = None
    for repeat in range(2):
        recording={'format':'incant-input','version':1,'tick_rate':rate,'start_tick':0,'ticks':5,
                   'frames':[{'tick':1,'events':[{'type':'key','code':'KeyD','down':True}]}]}
        observe={'format':'incant-native-observation','version':1,'entities':[entity],
                 'checks':[{'name':'custom motion','tick':5,'expect':{'type':'vector','entity':entity,
                 'property':'translation','value':[14/rate,0,0],'tolerance':1e-10}}]}
        request={'seconds':4.75/rate,'script_sha256':digest,'inputs_script':recording,'observe':observe}
        failed=json.loads(json.dumps(request))
        failed['inputs_script']['frames'][0]['events'].append({'type':'key','code':'Space','down':True})
        false=json.loads(json.dumps(request)); false['observe']['checks'][0]['expect']['value']=[99,0,0]
        query={'path':f'/entities/{entity}/transform/translation'}
        requests=[{'method':'play.run','params':request},{'method':'play.state_query','params':query},
                  {'method':'play.run','params':failed},{'method':'play.state_query','params':query},
                  {'method':'play.run','params':false},{'method':'play.state_query','params':{'path':'/logs'}},
                  {'method':'project.read'}]
        for index,item in enumerate(requests): item['id']=index
        process=run(['rpc',project],input=''.join(json.dumps(r)+'\n' for r in requests),check=True)
        responses=[json.loads(line) for line in process.stdout.splitlines()]
        require(len(responses)==len(requests),'response count')
        require(all(r['id']==i for i,r in enumerate(responses)),'response identity')
        report=responses[0]['result']; state=report['state']
        require(report['passed'] and state['runtime']=='native' and state['tick']==5,'native run result')
        require(state['script_source_sha256']==digest and not state['script_source_parsing'],'embedded identity')
        require(len(state['logs']['entries'])==5 and state['logs']['entries'][-1]['message']=='accepted 5 🚀','custom committed logs')
        require(abs(responses[1]['result']['untrusted_runtime_data'][0]-14/rate)<1e-10,'selected native query')
        require('later failure' in responses[2]['error']['message'],'candidate runtime failure')
        require(responses[1]['result']==responses[3]['result'],'failed candidate replaced previous session')
        require(responses[4]['result']['passed'] is False and responses[4]['result']['state']['tick']==5,'false assertion report')
        require(responses[5]['result']['untrusted_runtime_data']==state['logs'],'queried native logs')
        require(responses[6]['result']['revision']==0,'play modified authoring')
        if baseline is not None: require(responses==baseline,'fresh process divergence')
        baseline=responses
        (out/f'agent-run-{repeat}.json').write_text(json.dumps(responses,indent=2)+'\n')
    return {'processes':2,'source_parsing':False,'full_world_serialization':False,
            'real_provider':False,'candidate_failure_preserves_previous':True,'false_assertion_inspectable':True}
