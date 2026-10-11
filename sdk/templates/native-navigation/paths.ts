import type { NativeNavigation, NativeId, NativePathRequest, NativeGridPathRequest } from '../../ts/src/native';

/** The caller applies movement through existing numeric/command APIs and must
 * implement every off-mesh traversal explicitly. Requery when generation changes. */
export function meshRoute(navigation: NativeNavigation, owner: NativeId, start: NativePathRequest['start'], end: NativePathRequest['end']) {
  return navigation.findPath(owner, { start, end, snap_distance: 1, max_visited: 2048 });
}

/** Cell indices stay in grid space; the game supplies its own world mapping. */
export function gridRoute(navigation: NativeNavigation, owner: NativeId, start: NativeGridPathRequest['start'], end: NativeGridPathRequest['end']) {
  return navigation.findGridPath(owner, { start, end, diagonal: false, max_expansions: 2048 });
}
