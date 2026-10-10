// Runs once in a fresh context before any project code. Fixed, non-owning
// external buffers must not transfer their backing storage to another object.
(() => {
    'use strict';
    const deny = () => { throw new TypeError('ArrayBuffer transfer is unavailable in the native script host'); };
    for (const name of ['transfer', 'transferToFixedLength', 'transferToImmutable']) {
        if (typeof ArrayBuffer.prototype[name] !== 'function') throw new Error('unexpected pinned ArrayBuffer API');
        Object.defineProperty(ArrayBuffer.prototype, name, {
            value: deny, writable: false, configurable: false, enumerable: false
        });
    }
    Object.freeze(ArrayBuffer.prototype);
    Object.freeze(ArrayBuffer);
    Object.defineProperty(globalThis, 'ArrayBuffer', {
        value: ArrayBuffer, writable: false, configurable: false, enumerable: false
    });
    for (const name of ['structuredClone', 'Worker', 'ShadowRealm']) {
        if (typeof globalThis[name] !== 'undefined') throw new Error('unexpected native script realm API');
        Object.defineProperty(globalThis, name, {
            value: undefined, writable: false, configurable: false, enumerable: false
        });
    }
})();
