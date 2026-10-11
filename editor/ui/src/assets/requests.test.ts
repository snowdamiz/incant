import { describe, expect, it } from 'vitest';
import type { ProjectAsset, Ulid } from '../bridge/contract';
import { reimportRequest, requestFor } from './AssetsContext';

const asset = (path: string, kind: string): ProjectAsset => ({
  id: '01J9ZF1XTR0000000000000001' as Ulid,
  name: 'Imported source', path, kind, fingerprint: 'fixture',
});

describe('typed import interpretation', () => {
  it.each(['sprites/ui.atlas.json', 'audio/theme.wav', 'audio/hit.OGG', 'models/scene.glb'])(
    'omits a stale draft interpretation for %s', (source) => {
      expect(requestFor({ key: 1, source, usage: 'normal' }, [])).toEqual({ source });
    },
  );

  it.each([
    ['sprites/ui.atlas.json', 'texture'],
    ['sprites/UI.ATLAS.JSON', 'texture'],
    ['audio/theme.wav', 'audio'],
    ['audio/hit.ogg', 'audio'],
    ['models/scene.glb', 'model'],
  ])('does not reinterpret %s during reimport', (path, kind) => {
    expect(reimportRequest(asset(path, kind), 'linear')).toEqual({ source: path });
  });

  it('retains ordinary image reinterpretation and saved defaults', () => {
    const image = { ...asset('textures/normal.png', 'texture'), textureUsage: 'normal' as const };
    expect(reimportRequest(image)).toEqual({ source: image.path });
    expect(reimportRequest(image, 'normal')).toEqual({ source: image.path });
    expect(reimportRequest(image, 'linear')).toEqual({ source: image.path, textureUsage: 'linear' });
    expect(requestFor({ key: 1, source: image.path, usage: 'normal' }, [image])).toEqual({ source: image.path });
    expect(requestFor({ key: 1, source: image.path, usage: 'linear' }, [image])).toEqual({ source: image.path, textureUsage: 'linear' });
  });
});
