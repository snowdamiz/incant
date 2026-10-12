import type { NativeBehavior } from '@incant/runtime/native';
export default {
  persistence: { version: 1 },
  update(t) {
    const rows=t.animations.ik,tick=t.clock.tick;
    if(rows.ids.length!==2||rows.chains.length!==6||rows.weights.length!==2)throw Error('IK row shape');
    if(tick===1)t.timers.set('pulse',3,3);
    for(const instance of t.instances){
      const id=instance.entity,i=rows.ids.indexOf(id),first=instance.props.first===true;
      if(i<0||rows.constraintIds[i]!=='00000000000000000000000009')throw Error('IK identity');
      if(rows.chains[3*i]!==0||rows.chains[3*i+1]!==1||rows.chains[3*i+2]!==2)throw Error('chain identity');
      const count=Number(t.state.get(id)??0)+1;if(count!==tick)throw Error('persistent count');t.state.set(count,id);
      const expected=tick===1?1:(first?1:-1)*(1+(tick-1)/64),weight=tick===1?1:((tick-1)%3)/2;
      if(rows.targets[3*i]!==expected||rows.weights[i]!==weight)throw Error('committed IK continuation');
      for(const chunk of t.chunks)for(let n=0;n<chunk.ids.length;n++)if(chunk.ids[n]===id){
        const offset=n*chunk.transformStride;chunk.transforms[offset]=expected;chunk.transforms[offset+1]=weight;chunk.transforms[offset+2]=count;
      }
      t.commands.setAnimationIk(id,rows.constraintIds[i],0,1,0,0,1,0,0);
      t.commands.setAnimationIk(id,rows.constraintIds[i],(first?1:-1)*(1+tick/64),1,0,0,1,0,(tick%3)/2);
      if(rows.targets[3*i]!==expected||rows.weights[i]!==weight)throw Error('queued control leaked');
      t.log(JSON.stringify([tick,id,expected,weight,count,t.timers.events.length]));
    }
  },
} satisfies NativeBehavior;
