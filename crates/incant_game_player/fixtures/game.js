// Trusted build fixture; project behaviors replace this through the vetted cook.
// Source is never loaded or parsed by the standalone runtime executable.
exports.default = { persistence: {version: 1}, update({chunks, input}) {
    if (input.actions.fail?.pressed) throw Error('accepted game input failure');
    for (const chunk of chunks) {
        // Finite source-free material-miss fixture; ordinary actions omit mirror.
        if (input.actions.mirror?.pressed) for (let i = 0; i < chunk.ids.length; i++) {
            if (chunk.ids[i] === '00000000000000000000000001') chunk.transforms[i * 10 + 7] *= -1;
        }
        if (!chunk.velocities) continue;
        for (let i = 0; i < chunk.ids.length; i++) {
            chunk.velocities[i * 3] = input.actions.right?.active ? 3 : 0;
        }
    }
}};
