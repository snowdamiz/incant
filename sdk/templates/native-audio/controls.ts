import type { NativeCommands, NativeId } from '../../ts/src/native';

/** Music pause/resume keeps its cursor; buses retain their startup routing. */
export function musicSettings(commands: NativeCommands, music: NativeId, bus: NativeId, paused: boolean, gainDb: number): void {
  commands.setAudioPlaying(music, !paused);
  commands.setAudioGain(music, 0);
  commands.setAudioBusGain(bus, gainDb);
}
