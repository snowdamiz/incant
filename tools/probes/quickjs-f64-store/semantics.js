// Differential observable records. No timing and no engine/project mutation.
(() => {
  'use strict';
  const records = [];
  function require(value, message) { if (!value) throw Error(message); }
  function scalar(value) {
    if (typeof value === 'number') return Number.isNaN(value) ? 'NaN' :
      Object.is(value, -0) ? '-0' : value === Infinity ? '+Infinity' : value === -Infinity ? '-Infinity' : value;
    if (typeof value === 'bigint') return `${value}n`;
    if (typeof value === 'symbol') return String(value);
    return value === undefined ? 'undefined' : value;
  }
  function inspect(array) {
    let bytes;
    try { bytes = Array.from(new Uint8Array(array.buffer)); } catch (e) { bytes = e.name; }
    return { length: array.length, values: Array.from(array, scalar), bytes };
  }
  function record(name, body) {
    const log = [];
    records.push({ name, result: body(log), log });
  }
  const numbers = [0, -0, 7, -7, 1.25, Number.MAX_SAFE_INTEGER, Number.MIN_VALUE,
    Number.MAX_VALUE, NaN, Infinity, -Infinity, tagInt, tagFloat];
  for (let n = 0; n < numbers.length; n++) record(`number-${n}`, () => {
    const value = numbers[n], a = new Float64Array([111, 222, 333]);
    const returned = (a[1] = value);
    require(Object.is(a[1], value) && Object.is(returned, value), 'numeric store/return differs');
    require(a[0] === 111 && a[2] === 333, 'numeric store escaped index');
    return inspect(a);
  });
  for (const [name, key] of [['negative', -1], ['fractional', 1.5], ['nan', NaN],
    ['positive-infinity', Infinity], ['negative-infinity', -Infinity], ['large', 2 ** 32],
    ['oob', 3], ['numeric-string', '1'], ['negative-zero-string', '-0'], ['zero-number', -0],
    ['leading-zero', '01'], ['decimal-string', '1.0'], ['exponent-string', '1e0'],
    ['ordinary-property', 'other'], ['symbol', Symbol('field')]]) record(`index-${name}`, () => {
      const a = new Float64Array([111, 222, 333]);
      a[key] = 19.5;
      return { ...inspect(a), own: Reflect.ownKeys(a).map(k => [String(k), scalar(a[k])]) };
    });
  for (const [name, value] of [['boolean', true], ['null', null], ['undefined', undefined],
    ['string', '12.5'], ['bigint', 7n], ['symbol', Symbol('value')]]) record(`value-${name}`, () => {
      const a = new Float64Array([111]); let error = null;
      try { a[0] = value; } catch (e) { error = e.name; }
      if (name === 'bigint' || name === 'symbol') require(error === 'TypeError' && a[0] === 111, 'type error skipped');
      return { error, ...inspect(a) };
    });
  for (const key of [0, -1, 1.5, 999]) record(`coercion-${key}`, log => {
    const a = new Float64Array([111]);
    const value = { [Symbol.toPrimitive](hint) { log.push(`value:${hint}`); return 17.25; } };
    a[{ [Symbol.toPrimitive](hint) { log.push(`key:${hint}`); return key; } }] = value;
    require(log[0] === 'key:string' && log[1] === 'value:number', 'coercion ordering changed');
    return inspect(a);
  });
  record('throwing-coercion', log => {
    const a = new Float64Array([111]); let error;
    try { a[0] = { valueOf() { log.push('valueOf'); throw Error('conversion'); } }; }
    catch (e) { error = [e.name, e.message]; }
    require(a[0] === 111 && log.length === 1, 'throwing conversion changed storage');
    return { error, ...inspect(a) };
  });
  record('ordinary-coercion-fallback', log => {
    const a = new Float64Array([111]);
    a[0] = { valueOf() { log.push('valueOf'); return {}; }, toString() { log.push('toString'); return '17.25'; } };
    require(a[0] === 17.25 && log.join(',') === 'valueOf,toString', 'ordinary conversion order');
    return inspect(a);
  });
  record('throwing-key', log => {
    const a = new Float64Array([111]); let error;
    const key = { [Symbol.toPrimitive]() { log.push('key'); throw RangeError('key error'); } };
    const value = { valueOf() { log.push('value'); return 7.5; } };
    try { a[key] = value; } catch (e) { error = e.name; }
    require(error === 'RangeError' && log.join(',') === 'key' && a[0] === 111, 'key error ordering');
    return { error, ...inspect(a) };
  });
  record('coercion-accessor', log => {
    const a = new Float64Array([111]);
    a[0] = { get [Symbol.toPrimitive]() { log.push('get primitive'); return hint => { log.push(hint); return -0; }; } };
    require(Object.is(a[0], -0) && log.join(',') === 'get primitive,number', 'coercion accessor');
    return inspect(a);
  });
  record('nonprimitive-conversion', log => {
    const a = new Float64Array([111]); let error;
    try { a[0] = { valueOf() { log.push('valueOf'); return {}; }, toString() { log.push('toString'); return {}; } }; }
    catch (e) { error = e.name; }
    require(error === 'TypeError' && a[0] === 111 && log.join(',') === 'valueOf,toString', 'nonprimitive coercion');
    return { error, ...inspect(a) };
  });
  record('detached-before-number', () => {
    const a = new Float64Array([111]); a.buffer.transfer(); a[0] = 7.5;
    require(a.length === 0 && a[0] === undefined, 'detached store revived view');
    return { length: a.length, read: scalar(a[0]) };
  });
  record('detach-during-conversion', log => {
    const a = new Float64Array([111]); let transferred;
    a[0] = { valueOf() { log.push('detach'); transferred = a.buffer.transfer(); return 7.5; } };
    require(a.length === 0 && new Float64Array(transferred)[0] === 111, 'conversion detach wrote backing');
    return { length: a.length, transferred: inspect(new Float64Array(transferred)) };
  });
  for (const coercion of [false, true]) record(`immutable-${coercion}`, log => {
    const a = new Float64Array(new Float64Array([111]).buffer.sliceToImmutable());
    const value = coercion ? { valueOf() { log.push('conversion'); return 7.5; } } : 7.5;
    let error = null; try { a[0] = value; } catch (e) { error = e.name; }
    require(a[0] === 111 && log.length === Number(coercion), 'immutable write/coercion changed');
    return { error, ...inspect(a) };
  });
  record('resizable-shrink', log => {
    const b = new ArrayBuffer(24, { maxByteLength: 32 }), a = new Float64Array(b);
    a[0] = 111;
    a[0] = { valueOf() { log.push('resize'); b.resize(0); return 7.5; } };
    require(a.length === 0, 'shrink lost'); b.resize(24); require(a[0] === 0, 'stale store after shrink');
    return inspect(a);
  });
  record('resizable-grow', log => {
    const b = new ArrayBuffer(8, { maxByteLength: 32 }), a = new Float64Array(b);
    a[1] = { valueOf() { log.push('resize'); b.resize(24); return 7.5; } };
    require(a.length === 3 && a[1] === 7.5, 'conversion growth not respected');
    return inspect(a);
  });
  record('proxy', log => {
    const a = new Float64Array([111]);
    const proxy = new Proxy(a, { set(target, key, value) { log.push([key, scalar(value)]); return Reflect.set(target, key, value, target); } });
    proxy[0] = 7.5;
    require(log.length === 1 && a[0] === 7.5, 'proxy trap bypassed'); return inspect(a);
  });
  record('proxy-refusal', log => {
    const a = new Float64Array([111]); let error;
    const proxy = new Proxy(a, { set() { log.push('refused'); return false; } });
    try { proxy[0] = 7.5; } catch (e) { error = e.name; }
    require(error === 'TypeError' && a[0] === 111 && log.length === 1, 'proxy refusal bypassed'); return inspect(a);
  });
  record('subclass-offset', () => {
    class Derived extends Float64Array {}
    const b = new Float64Array([111, 222, 333, 444]), a = new Derived(b.buffer, 8, 2);
    a[0] = -0; a[1] = 7.5;
    require(a instanceof Derived && Object.is(b[1], -0) && b[2] === 7.5 && b[0] === 111 && b[3] === 444, 'subclass/offset store');
    return inspect(b);
  });
  record('other-classes', () => {
    const arrays = [[111], new Int8Array([111]), new Float32Array([111]), new BigInt64Array([111n])];
    const errors = [];
    for (const a of arrays) { try { a[0] = 7.5; errors.push(null); } catch (e) { errors.push(e.name); } }
    require(arrays[0][0] === 7.5 && arrays[1][0] === 7 && arrays[2][0] === 7.5 && errors[3] === 'TypeError', 'other class changed');
    return { values: arrays.map(a => Array.from(a, scalar)), errors };
  });
  record('reflect-and-expression', () => {
    const a = new Float64Array([111, 222]);
    const reflected = Reflect.set(a, '0', 7.5), expression = (a[1] = a[0] + 2);
    const assigned = a[0] += 1;
    require(reflected && expression === 9.5 && assigned === 8.5, 'assignment result changed');
    return { reflected, expression, assigned, ...inspect(a) };
  });
  return JSON.stringify(records);
})();
