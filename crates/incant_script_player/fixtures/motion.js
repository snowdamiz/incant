// Default build fixture. Exports replace this at build time with freshly
// transpiled CommonJS source; the player has no runtime script-path argument.
let ticks = 0;
exports.default = { update({chunks, dt, commands}) {
    ticks += 1;
    for (const chunk of chunks) {
        for (let i = 0; i < chunk.transforms.length; i += chunk.transformStride) {
            chunk.transforms[i] += dt;
        }
    }
    const spawned = '0000000000000000000000002A';
    if (ticks === 1) commands.spawn(spawned, [0,0,0,0,0,0,1,1,1,1], null, [2,0,0]);
    if (ticks === 4) commands.despawn(spawned);
}};
