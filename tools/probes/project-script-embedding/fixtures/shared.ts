// INCANT_EMBEDDING_TS_SOURCE_SENTINEL_67d2c3
import type { NativeContext } from '@incant/runtime/native';
let counter = 0;
let current: NativeContext['chunks'] | undefined;
let retained: Float64Array | undefined;
export function next() { return ++counter; }
export function update(context: NativeContext, group: number, count: number, initialized: number) {
  const { chunks, instances, clock, input, log } = context;
  if (initialized !== group || next() !== clock.tick * 2 + group) throw Error('shared initialization/source order');
  if (instances.length !== count || !Object.isFrozen(instances)) throw Error('group membership');
  if (group === 1) {
    if (retained && retained.byteLength !== 0) throw Error('old loan remains live');
    current = chunks; retained = chunks[0].transforms;
  } else if (chunks !== current) throw Error('not one aggregate loan');
  for (const compile of [() => eval('1'), () => (0,eval)('1'),
    () => Function('return 1')(), () => (function(){}).constructor('return 1')(),
    () => (async function(){}).constructor('return 1')(),
    () => (function*(){}).constructor('yield 1')()]) {
    let denied = false; try { compile(); } catch (_) { denied = true; }
    if (!denied) throw Error('runtime source evaluator active');
  }
  for (const instance of instances) {
    const props = instance.props;
    if (!Object.isFrozen(instance) || !Object.isFrozen(props) || Object.getPrototypeOf(props) !== null) throw Error('props ownership');
    const key = 'quote"\\\n\0🚀';
    if (props[key] !== '"}); panic!("not Rust"); //\n\\\0🚀') throw Error('escaped property lost');
    const nested = props.__proto__;
    if (!nested || typeof nested !== 'object' || Array.isArray(nested) || Object.getPrototypeOf(nested) !== null || !Object.isFrozen(nested)) throw Error('prototype key changed');
    const values = (nested as {readonly [key:string]: unknown}).constructor;
    if (!Array.isArray(values) || !Object.isFrozen(values) || values[0] !== 'own-data' || values[1] !== true || values[2] !== null) throw Error('nested constructor');
    let refused = 0;
    for (const write of [() => Object.defineProperty(props, 'speed', {value:99}),
      () => Object.defineProperty(values, '0', {value:'bad'}), () => Object.defineProperty(instances,'0',{value:null})]) {
      try { write(); } catch (_) { ++refused; }
    }
    if (refused !== 3) throw Error('frozen data modified');
    const speed = props.speed;
    if (typeof speed !== 'number') throw Error('invalid speed');
    for (const chunk of chunks) for (let row=0;row<chunk.ids.length;row++)
      if (chunk.ids[row] === instance.entity) chunk.transforms[row*10] += speed*context.dt;
  }
  log('group '+group+' tick '+clock.tick+' 🚀');
  if (group === 2) {
    if (input.keyboard.held.includes('KeyE')) eval('1');
    if (input.keyboard.held.includes('KeyF')) Function('return 1')();
    if (input.keyboard.held.includes('KeyX')) throw Error('late bound-source failure');
  }
}
