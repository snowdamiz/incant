import type { NativeCommands, HapticTarget } from '../src/native';
declare const commands: NativeCommands;
for (const target of commands.hapticCapabilities()) {
  if (target.kind === 'gamepad') commands.playHaptic(target, {kind:'rumble',strong:0.5,weak:0.25,durationMs:125});
  else commands.playHaptic(target, {kind:'impact',intensity:0.5});
  commands.stopHaptic(target);
}
// @ts-expect-error capabilities require a host generation
const stale: HapticTarget = {kind:'gamepad',id:0};
// @ts-expect-error unknown effects are not capabilities
commands.playHaptic({kind:'device',generation:1}, {kind:'unknown'});
void stale;
