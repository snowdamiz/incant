// Engine wrapper source; builtins were captured before project initialization.
let routineCount = 0;
function captureRoutines(behavior) {
  const out = nativeCreate(null);
  const declaration = nativeDescriptor(behavior, 'coroutines');
  if (declaration === undefined) return out;
  if (!nativeHasOwn(declaration, 'value')) throw new Error('coroutine catalog must be own data');
  const table = declaration.value;
  if (table === undefined) return out;
  if (!table || typeof table !== 'object' || nativeIsArray(table)) throw new Error('coroutine catalog required');
  const names = nativeOwnKeys(table);
  if (names.length > 32 || routineCount + names.length > 128) throw new Error('coroutine catalog limit');
  routineCount += names.length;
  for (let i = 0; i < names.length; i++) {
    const name = names[i];
    if (typeof name !== 'string' || !/^[A-Za-z0-9_.:-]{1,64}$/.test(name)) throw new Error('coroutine routine name');
    const entry = nativeDescriptor(table, name);
    if (!entry || !nativeHasOwn(entry, 'value') || !entry.value || typeof entry.value !== 'object') throw new Error('coroutine declaration required');
    const callback = nativeDescriptor(entry.value, 'resume');
    if (!callback || !nativeHasOwn(callback, 'value') || typeof callback.value !== 'function') throw new Error('coroutine resume function required');
    nativeDefine(out, name, {value: callback.value, enumerable: true});
  }
  return out;
}
