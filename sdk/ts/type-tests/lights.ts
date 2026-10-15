import type { NativeCommands } from '../src/native';
declare const commands: NativeCommands;
commands.setLightColor('light', [0.1, 0.2, 0.3]);
commands.setLightColor('light', new Float64Array([1, 0.5, 0]));
commands.setLightIntensity('light', 0);
// @ts-expect-error RGB has exactly three channels.
commands.setLightColor('light', [1, 0]);
// @ts-expect-error Intensity is a number, never an implicit string conversion.
commands.setLightIntensity('light', '10');
// @ts-expect-error Gameplay scalar controls cannot change shadow bindings.
commands.setLightShadows('light', true);
