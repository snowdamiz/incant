// Sandboxed synchronous callbacks. Host Rust code validates and commits all output.
globalThis.__tick = (worldJson, dt, stateJson, eventsJson, inputJson, clockJson, timerEventsJson) => {
  const world = JSON.parse(worldJson);
  const state = JSON.parse(stateJson);
  const commands = [];
  const logs = [];
  const timers = [];
  const replaced = new Set();
  let asynchronous = false;
  const api = Object.freeze({
    input: () => JSON.parse(inputJson),
    clock: () => JSON.parse(clockJson),
    setTimer: timer => {
      if (typeof __behavior.onTimer !== 'function') throw Error('onTimer required');
      if (timers.length >= 256) throw Error('timer action limit');
      const copy = JSON.parse(JSON.stringify(timer));
      timers.push({op:'set', timer:copy});
      replaced.add(copy.id);
    },
    cancelTimer: id => {
      if (timers.length >= 256) throw Error('timer action limit');
      timers.push({op:'cancel', id});
      replaced.add(id);
    },
    raycast: (query) => {
      const result = JSON.parse(globalThis.__incantRaycast(JSON.stringify(query)));
      if (result.error) throw new Error(result.error);
      return result.hit;
    },
    computeCharacterMotion: (query) => {
      const result = JSON.parse(globalThis.__incantCharacterMotion(JSON.stringify(query)));
      if (result.error) throw new Error(result.error);
      return result.movement;
    },
    triggerEvents: () => JSON.parse(eventsJson),
    query: (component) => Object.values(world.scenes).flatMap(scene => Object.values(scene.entities)
      .filter(entity => !component || Object.hasOwn(entity.components, component))
      .map(entity => ({...entity, scene_id: scene.id}))),
    command: (command) => { if (commands.length >= 10000) throw new Error('command limit'); commands.push(command); },
    log: (message, level = 'info') => {
      if (typeof message !== 'string' || message.length > 4096 || logs.length >= 64 ||
          !['debug', 'info', 'warn', 'error'].includes(level)) throw new Error('invalid script log');
      logs.push({level, message});
    }
  });
  const invoke = (fn, ...args) => {
    if (asynchronous) return;
    const returned = fn.apply(__behavior, args);
    if (returned && (typeof returned.then === 'function' || typeof returned.next === 'function'))
      asynchronous = true;
  };
  for (const event of JSON.parse(timerEventsJson)) {
    if (replaced.has(event.id)) continue;
    if (typeof __behavior.onTimer !== 'function') throw Error('onTimer required');
    invoke(__behavior.onTimer, api, event, state);
  }
  invoke(__behavior.update, api, dt, state);
  return JSON.stringify({state, commands, logs, timers, asynchronous});
};
