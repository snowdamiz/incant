// Offscreen-harness guest (development probe, not the sample's gameplay): exercises the
// win/loss HUD hook that needs no new engine API. At tick 3 it moves the authored 'Won'
// label (01CSWD...11) from the zero-size stash (01CSWD...10) into 'Result' (01CSWD...09)
// with the existing structural command buffer, then at tick 6 moves it back.
exports.default = {
  update({ commands, clock, log }) {
    if (clock.tick === 3) {
      commands.setParent('01CSWD00000000000000000011', '01CSWD00000000000000000009');
      log('won label -> result');
    }
    if (clock.tick === 6) {
      commands.setParent('01CSWD00000000000000000011', '01CSWD00000000000000000010');
      log('won label -> stash');
    }
    return undefined;
  },
};
