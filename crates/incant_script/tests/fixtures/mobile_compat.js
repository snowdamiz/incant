// Trusted cross-runtime numeric fixture. Never a runtime source input.
const doubles = [-0, 5e-324, 1.7976931348623157e308, 9007199254740991,
    1 / 3, (1 + Number.EPSILON) - 1, Math.fround(1 / 3)];
const wide = (1n << 127n) + 9007199254740993n;
const negative = -18446744073709551617n;
const makeClosure = value => offset => value + offset;
const closure = makeClosure(41);
function bits(value) {
    const bytes = new ArrayBuffer(8);
    const view = new DataView(bytes);
    view.setFloat64(0, value, true);
    return view.getUint32(4, true).toString(16).padStart(8, '0') +
        view.getUint32(0, true).toString(16).padStart(8, '0');
}
let updates = 0;
exports.default = { update({chunks, input, clock, dt, log}) {
    ++updates;
    // Runtime source compilation remains absent after trusted bytecode loading.
    for (const compile of [() => eval('1'), () => Function('return 1')]) {
        let denied = false;
        try { compile(); } catch (_) { denied = true; }
        if (!denied) throw Error('cross-runtime source compiler enabled');
    }
    const words = new BigInt64Array(2);
    words[0] = 9223372036854775807n;
    words[1] = -9223372036854775808n;
    const unsigned = new BigUint64Array(words.buffer);
    const bytes = new Uint8Array(new Uint32Array([0x12345678]).buffer);
    const regex = 'Aé中🙂'.match(/\p{Letter}/gu).join('');
    log('mobile.f64 ' + doubles.map(bits).join(','));
    log('mobile.bigint ' + wide + ',' + negative + ',' + (wide >> 64n));
    log('mobile.typed ' + unsigned[0] + ',' + unsigned[1] + ';' + Array.from(bytes).join(','));
    log('mobile.language ' + closure(1) + ';' + regex + ';' + 'e\u0301'.normalize('NFC'));
    log('mobile.tick ' + updates + ';' + clock.tick + ';' + bits(dt) + ';' + (input.actions.right?.active ? 1 : 0));
    for (const chunk of chunks) {
        if (!chunk.velocities) continue;
        for (let i = 0; i < chunk.ids.length; i++) {
            chunk.velocities[i * 3] = input.actions.right?.active ? 3 : 0;
        }
    }
    // A genuine late failure after numeric writes and buffered logs.
    if (input.actions.fail?.pressed) throw Error('accepted game input failure');
}};
