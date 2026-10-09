import { describe, expect, it } from 'vitest';
import { isConflict, pathProblem, sourceKind, splitPaste, splitPath } from './paths';

describe('import source paths (mirrors incant_import validate_path)', () => {
  it('accepts canonical project-relative model and image paths', () => {
    for (const path of ['models/crate.glb', 'a.gltf', 'textures/sky/Harbor 4k.EXR', 'x/y.jpeg', 'x/y.JPG', 'x.png']) {
      expect(pathProblem(path)).toBeNull();
    }
  });

  it('rejects paths the engine would reject, with a specific reason', () => {
    expect(pathProblem('')).toMatch(/Enter a path/);
    expect(pathProblem('/Users/me/crate.glb')).toMatch(/absolute/);
    expect(pathProblem('C:/crate.glb')).toMatch(/absolute/);
    expect(pathProblem('../outside/crate.glb')).toMatch(/leave the project/);
    expect(pathProblem('models/../crate.glb')).toMatch(/leave the project/);
    expect(pathProblem('./crate.glb')).toMatch(/\.\//);
    expect(pathProblem('models//crate.glb')).toMatch(/repeated/);
    expect(pathProblem('models/')).toMatch(/folder/);
    expect(pathProblem('models\\crate.glb')).toMatch(/forward slashes/);
    expect(pathProblem('https://example.com/crate.glb')).toMatch(/URL/);
    expect(pathProblem('file:///tmp/crate.glb')).toMatch(/URL/);
    expect(pathProblem('models/a:b.png')).toMatch(/:/);
    expect(pathProblem(' models/crate.glb')).toMatch(/spaces/);
    expect(pathProblem('models/crate.fbx')).toMatch(/Unsupported/);
    expect(pathProblem('models/.png')).toMatch(/Unsupported/);
  });

  it('derives the kind from the extension only', () => {
    expect(sourceKind('a/b.GLB')).toBe('model');
    expect(sourceKind('a/b.exr')).toBe('image');
    expect(sourceKind('a.b/c')).toBeNull();
  });

  it('splits pasted lines and display paths', () => {
    expect(splitPaste(' a.png\r\n\n b.glb \r')).toEqual(['a.png', 'b.glb']);
    expect(splitPath('a/b/c.png')).toEqual({ folder: 'a/b/', file: 'c.png' });
    expect(splitPath('c.png')).toEqual({ folder: '', file: 'c.png' });
  });

  it('recognises engine conflict errors', () => {
    expect(isConflict('document revision conflict: expected 1, current 2')).toBe(true);
    expect(isConflict('the project changed since import preparation; prepare again')).toBe(true);
    expect(isConflict('could not cook a.png')).toBe(false);
  });
});
