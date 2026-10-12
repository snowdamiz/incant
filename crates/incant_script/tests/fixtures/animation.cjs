"use strict";
exports.default = (function () {
const factories = [
function (require, exports) {
"use strict";
Object.defineProperty(exports, "__esModule", {
    value: true
});
Object.defineProperty(exports, "default", {
    enumerable: true,
    get: function() {
        return _default;
    }
});
const _default = {
    update ({ animations, instances, commands, clock }) {
        if (!(animations.clips instanceof Int32Array) || !(animations.seconds instanceof Float64Array) || !(animations.speeds instanceof Float64Array) || !(animations.flags instanceof Uint32Array)) throw Error('numeric arrays');
        for (const instance of instances){
            const row = animations.ids.indexOf(instance.entity);
            if (row < 0) throw Error('missing animation instance');
            if (clock.tick === 1) {
                const clip = instance.props.clip;
                const speed = instance.props.speed;
                const looping = instance.props.looping;
                if (typeof clip !== 'number' || typeof speed !== 'number' || typeof looping !== 'boolean') throw Error('props');
                const old = [
                    animations.clips[row],
                    animations.seconds[row],
                    animations.speeds[row],
                    animations.flags[row]
                ];
                commands.controlAnimation(instance.entity, 'select', clip, 0);
                commands.controlAnimation(instance.entity, 'speed', speed);
                commands.controlAnimation(instance.entity, 'looping', looping);
                commands.controlAnimation(instance.entity, 'playing', false);
                if (old.some((value, index)=>value !== [
                        animations.clips[row],
                        animations.seconds[row],
                        animations.speeds[row],
                        animations.flags[row]
                    ][index])) {
                    throw Error('pending readback');
                }
            } else {
                if (animations.clips[row] !== instance.props.clip || animations.speeds[row] !== instance.props.speed || (animations.flags[row] & 1) !== 0) throw Error('committed control');
                commands.controlAnimation(instance.entity, 'seek', 0.5);
            }
        }
        return undefined;
    }
};
},
];
const dependencies = [[]];
const cache = [];
function load(id) {
  if (cache[id]) return cache[id];
  const values = Object.create(null); cache[id] = values;
  factories[id](name => {
    const edge = dependencies[id].find(item => item[0] === name);
    if (!edge) throw Error('Unresolved native module');
    return load(edge[1]);
  }, values);
  return values;
}
return load(0).default;
})();
