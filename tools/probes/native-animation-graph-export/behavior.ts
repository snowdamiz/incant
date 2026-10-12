import type { NativeBehavior } from '@incant/runtime/native';
const parameter = '00000000000000000000000010';
const states = ['00000000000000000000000020', '00000000000000000000000021', '00000000000000000000000022'];
export default {
  persistence: { version: 1 },
  update(t) {
    const a=t.animations,g=a.graphs,c=t.commands,tick=t.clock.tick;
    if(g.ids.length!==3||g.parameterOffsets.length!==4||g.parameterOffsets[3]!==3)throw Error('graph alignment');
    for(const instance of t.instances){
      const id=instance.entity,first=instance.props.first===true,i=g.ids.indexOf(id),j=a.ids.indexOf(id),p=g.parameterOffsets[i];
      if(i<0||j<0||g.parameterIds[p]!==parameter)throw Error('graph membership');
      const count=Number(t.state.get(id)??0)+1;if(count!==tick)throw Error('persistent clock');t.state.set(count,id);
      const before=[states.indexOf(g.states[i]),g.seconds[i],g.elapsed[i],g.parameterValues[p],a.clips[j],a.seconds[j],a.flags[j],g.sourceSeconds[i],g.weights[i]];
      for(const chunk of t.chunks)for(let n=0;n<chunk.ids.length;n++)if(chunk.ids[n]===id){const k=n*chunk.transformStride;chunk.transforms[k]=before[0]+before[3];chunk.transforms[k+1]=before[1];chunk.transforms[k+2]=tick+before[8];}
      if(instance.props.completed===true){
        if(g.paused[i]!==Number(tick>=10&&tick<=12))throw Error('master pause');
        if(tick===9)c.controlAnimation(id,'playing',false);
        if(tick===10)c.setAnimationParameter(id,parameter,0.5);
        if(tick===12)c.controlAnimation(id,'playing',true);
        if(tick===13)c.controlAnimation(id,'speed',1);
        if(tick===50)c.setAnimationParameter(id,parameter,0.25);
        if(tick===60)c.selectAnimationState(id,states[2]);
        if(tick===70)c.setAnimationParameter(id,parameter,0.75);
        t.log(JSON.stringify([tick,id,...before]));continue;
      }
      if(tick===1)c.setAnimationParameter(id,parameter,0.5);
      if(tick===3&&!first)c.controlAnimation(id,'playing',true);
      if((tick===6||tick===50)&&first)c.controlAnimation(id,'playing',false);
      if((tick===9||tick===53)&&first)c.controlAnimation(id,'playing',true);
      if(tick===40)c.setAnimationParameter(id,parameter,first?0.25:0.75);
      if(tick===45)c.controlAnimation(id,'speed',first?-1:2);
      if(tick===60)c.selectAnimationState(id,states[2]);
      if(tick===70)c.setAnimationParameter(id,parameter,first?-1:2);
      if(tick===75){c.setAnimationParameter(id,parameter,0);c.selectAnimationState(id,states[0]);}
      if(g.seconds[i]!==before[1]||g.parameterValues[p]!==before[3])throw Error('queued control leaked');
      t.log(JSON.stringify([tick,id,...before]));
    }
  },
} satisfies NativeBehavior;
