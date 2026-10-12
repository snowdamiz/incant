import type { NativeBehavior } from '@incant/runtime/native';
const id=(n:number)=>String(n).padStart(26,'0');
const parameters=[id(10),id(11),id(12)];
const states=[id(20),id(21)];
export default {
  persistence: { version: 1 },
  update(t) {
    const a=t.animations,g=a.graphs,c=t.commands,tick=t.clock.tick;
    if(a.ids.length!==2||g.ids.length!==2||a.poseOffsets.length!==3)throw Error('animation membership');
    const vectors=new Map<string,number[]>();
    for(const instance of t.instances){
      const entity=instance.entity,i=a.ids.indexOf(entity),j=g.ids.indexOf(entity);
      if(i<0||j<0||g.weighted[j]!==1||g.blending[j]!==0||a.transitionFlags[i]!==0)throw Error('weighted tree row');
      const begin=a.poseOffsets[i],end=a.poseOffsets[i+1],poses=[];
      let sum=0;
      for(let k=begin;k<end;k++){
        poses.push([a.poseClips[k],a.poseSeconds[k],a.poseWeights[k]]);
        sum+=a.poseWeights[k];
      }
      if(Math.abs(sum-1)>1e-12||a.clips[i]!==poses[0][0]||a.seconds[i]!==poses[0][1])throw Error('representative or normalized pose');
      const count=Number(t.state.get(entity)??0)+1;
      if(count!==tick)throw Error('persistent script count');t.state.set(count,entity);
      if(instance.props.rest===true){
        if(poses.length!==1||poses[0][0]!==-1||poses[0][1]!==0||poses[0][2]!==1)throw Error('rest tree');
        vectors.set(id(230),[g.seconds[j],a.flags[i],g.paused[j]]);
        if(tick===5)c.controlAnimation(entity,'playing',true);
        if(tick===8)c.controlAnimation(entity,'playing',false);
        if(tick===11)c.controlAnimation(entity,'playing',true);
      }else{
        const p=g.parameterOffsets[j],values=[...g.parameterValues.slice(p,g.parameterOffsets[j+1])];
        if(values.length!==3||parameters.some((v,k)=>g.parameterIds[p+k]!==v))throw Error('parameter alignment');
        for(let k=0;k<8;k++)vectors.set(id(200+k),poses[k]??[-2,0,0]);
        vectors.set(id(220),[poses.length,g.seconds[j],a.flags[i]]);
        vectors.set(id(221),values);
        vectors.set(id(222),[g.elapsed[j],states.indexOf(g.states[j]),g.paused[j]]);
        if(tick===4)c.setAnimationParameter(entity,parameters[0],1);
        if(tick===8)c.controlAnimation(entity,'playing',false);
        if(tick===10){c.setAnimationParameter(entity,parameters[1],0.25);c.setAnimationParameter(entity,parameters[2],0);}
        if(tick===12)c.controlAnimation(entity,'playing',true);
        if(tick===16)c.controlAnimation(entity,'speed',-15);
        if(tick===20){c.setAnimationParameter(entity,parameters[0],0.5);c.setAnimationParameter(entity,parameters[2],0.5);}
        if(tick===24)c.selectAnimationState(entity,states[1]);
        if(tick===28)c.selectAnimationState(entity,states[0]);
        if(tick===32)c.controlAnimation(entity,'speed',0);
        if(tick===36)c.controlAnimation(entity,'speed',15);
        if(tick===40)parameters.forEach((p,k)=>c.setAnimationParameter(entity,p,k===2?1:0));
        if(tick===44)[0.25,0.5,0.75].forEach((v,k)=>c.setAnimationParameter(entity,parameters[k],v));
        if(g.parameterValues[p]!==values[0])throw Error('queued parameter mutated observation');
      }
      t.log(JSON.stringify([tick,entity,g.seconds[j],g.elapsed[j],a.flags[i],g.paused[j],poses]));
    }
    for(const chunk of t.chunks)for(let n=0;n<chunk.ids.length;n++){
      const v=vectors.get(chunk.ids[n]);if(v){const k=n*chunk.transformStride;chunk.transforms[k]=v[0];chunk.transforms[k+1]=v[1];chunk.transforms[k+2]=v[2];}
    }
    // Both current columns and retained JS values are copies, never native loans.
    a.poseWeights.fill(99);a.poseClips.fill(99);a.poseSeconds.fill(99);
  },
} satisfies NativeBehavior;
