// Capture before any guest code can replace the global property.
export const freeze = Object.freeze;
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

// Fixed persistent-data inspector. Capture builtins before project code runs.
// Native callers reserve their quota and release all Rust borrows before entry.
export const persistentShape = (() => {
    const ownKeys = Reflect.ownKeys;
    const hasOwn = Object.hasOwn;
    const text = String;
    const Invalid = TypeError;
    const descriptor = Object.getOwnPropertyDescriptor;
    const prototype = Object.getPrototypeOf;
    const define = Object.defineProperty;
    const isArray = Array.isArray;
    const objectPrototype = Object.prototype;
    const arrayPrototype = Array.prototype;
    const append = (array, index, value) => define(array, index, { value, enumerable: true });
    return (value, remaining) => {
        const array = isArray(value);
        const proto = prototype(value);
        if (array ? proto !== arrayPrototype : proto !== null && proto !== objectPrototype)
            throw new Invalid('persistent state class/prototype unsupported');
        const keys = ownKeys(value);
        if (keys.length > remaining + (array ? 1 : 0))
            throw new Invalid('persistent state node limit');
        const result = [];
        if (array) {
            const length = descriptor(value, 'length');
            if (!length || !hasOwn(length, 'value') || keys.length !== length.value + 1)
                throw new Invalid('persistent state sparse/extended array');
            for (let i = 0; i < length.value; i++) {
                const d = descriptor(value, text(i));
                if (!d || !hasOwn(d, 'value') || !d.enumerable)
                    throw new Invalid('persistent state accessor/sparse array');
                append(result, i, d.value);
            }
        } else {
            for (let i = 0; i < keys.length; i++) {
                const key = keys[i];
                const d = descriptor(value, key);
                if (typeof key !== 'string' || !d || !hasOwn(d, 'value') || !d.enumerable)
                    throw new Invalid('persistent state requires enumerable string data properties');
                const row = [];
                append(row, 0, key);
                append(row, 1, d.value);
                append(result, i, row);
            }
        }
        return result;
    };
})();

// The private native callback receives these trusted functions as call-local
// arguments. Capturing JS values inside its Rust closure would evade VM tracing.
export const persistentWrapper = (() => {
    const inspect = persistentShape;
    const immutable = freeze;
    return native => function(a, b, c, d) {
        switch (arguments.length) {
            case 0: return native(inspect, immutable);
            case 1: return native(inspect, immutable, a);
            case 2: return native(inspect, immutable, a, b);
            case 3: return native(inspect, immutable, a, b, c);
            case 4: return native(inspect, immutable, a, b, c, d);
            default: return native(inspect, immutable, a, b, c, d, undefined);
        }
    };
})();
