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
        for (const instance of instances){
            const row = animations.ids.indexOf(instance.entity);
            if (row < 0 || animations.transitionFlags.length !== animations.ids.length) throw Error('alignment');
            const duration = instance.props.duration;
            if (typeof duration !== 'number') throw Error('duration');
            if (clock.tick === 1) {
                commands.controlAnimation(instance.entity, 'crossfade', 1, 0.5, duration);
                if (animations.transitionFlags[row] !== 0) throw Error('queued fade');
            } else {
                if (animations.transitionFlags[row] !== 3 || animations.sourceClips[row] !== 0 || animations.sourceSeconds[row] !== 0.125 || animations.sourceSpeeds[row] !== 1 || animations.transitionElapsed[row] !== 0.125 || animations.transitionDuration[row] !== duration) throw Error('captured fade');
                commands.controlAnimation(instance.entity, 'select', null, 0);
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
