/**
 * Source-path rules for asset imports. These mirror crates/incant_import validate_path()
 * and its extension table so the form can explain a problem before anything is sent.
 * The engine stays the authority: a path that passes here can still be rejected there
 * (for example, when the file does not exist), and that exact error is shown.
 */
import type { TextureUsage } from '../bridge/contract';

/** Mirrors incant_import::MAX_BATCH_IMPORTS. */
export const MAX_BATCH = 64;

export type SourceKind = 'model' | 'image';

const EXTENSIONS: Readonly<Record<string, SourceKind>> = {
  gltf: 'model',
  glb: 'model',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  exr: 'image',
};

export const ACCEPTED_FORMATS = 'glTF, GLB, PNG, JPEG or EXR';

/** The import kind implied by the path's extension, or null when it is not importable. */
export function sourceKind(path: string): SourceKind | null {
  const file = path.slice(path.lastIndexOf('/') + 1);
  const dot = file.lastIndexOf('.');
  if (dot <= 0) return null;
  return EXTENSIONS[file.slice(dot + 1).toLowerCase()] ?? null;
}

/** A short, specific reason the path cannot be sent, or null when it is well formed. */
export function pathProblem(path: string): string | null {
  if (path.length === 0) return 'Enter a path.';
  if (path.trim() !== path) return 'Remove the spaces at the start or end.';
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(path)) return 'Use a file in the project folder, not a URL.';
  if (path.includes('\\')) return 'Use forward slashes (/) between folders.';
  if (path.startsWith('/') || /^[a-z]:/i.test(path)) return 'Use a path relative to the project file, not an absolute path.';
  if (path.includes(':') || path.includes('\0')) return 'Paths cannot contain “:”.';
  const parts = path.split('/');
  if (parts.includes('..')) return 'Paths cannot leave the project folder (“..”).';
  if (parts.includes('.')) return 'Remove “./” from the path.';
  if (parts.includes('')) return path.endsWith('/') ? 'Enter a file, not a folder.' : 'Remove the repeated “/”.';
  if (!sourceKind(path)) return `Unsupported file type. Use ${ACCEPTED_FORMATS}.`;
  return null;
}

/** Splits pasted text into candidate paths: one per line, blank lines dropped. */
export function splitPaste(text: string): string[] {
  return text
    .split(/\r\n|\r|\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/** Splits a source path for display so the file name stays visible when the folder is truncated. */
export function splitPath(path: string): { folder: string; file: string } {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? { folder: '', file: path } : { folder: path.slice(0, slash + 1), file: path.slice(slash + 1) };
}

export const USAGE_LABEL: Readonly<Record<TextureUsage, string>> = {
  color: 'Color',
  linear: 'Linear',
  normal: 'Normal map',
};

export const USAGE_HINT: Readonly<Record<TextureUsage, string>> = {
  color: 'sRGB color, such as albedo or UI art.',
  linear: 'Non-color data, such as roughness or masks.',
  normal: 'Tangent-space normal map.',
};

export const KIND_LABEL: Readonly<Record<string, string>> = {
  model: 'Model',
  texture: 'Texture',
};

/** Display label for an engine asset kind. Unknown kinds are shown as reported. */
export function kindLabel(kind: string): string {
  return KIND_LABEL[kind] ?? kind;
}

/** Engine errors that mean the project changed underneath the import; retrying is safe. */
export function isConflict(message: string): boolean {
  return /revision conflict|project changed since/i.test(message);
}
