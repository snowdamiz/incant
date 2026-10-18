// Independently authored TS contract reference, executed by ordinary native Play.
import type { NativeBehavior } from '@incant/runtime/native';

const locales = ['en', 'fr-CA', 'de', 'zh-Hant'] as const;
const tables = ['00000000000000000000000004', '00000000000000000000000005'];
export default {
  update({ clock, chunks, localization, commands, log }) {
    const count = clock.tick;
    const phase = (count - 1) % 4;
    const locale = locales[phase];
    if (locale === undefined || count < 1 || count > 12) throw Error('clock');
    const current = localization.locale();
    if (current.locale !== locale || current.pseudo !== (phase === 2)
        || JSON.stringify(current.fallbacks) !== JSON.stringify(phase === 2 ? ['fr'] : [])) throw Error('settings');
    const next = (phase + 1) % 4;
    commands.setLocale({ locale: locales[next]!, fallbacks: next === 2 ? ['fr'] : [], pseudo: next === 2 });
    const expected: Array<{ table_id: string; key: string; requested_locale: string;
      resolved_locale: string; kind: string }> = [];
    for (const table_id of tables) {
      if (phase === 2) expected.push({ table_id, key: 'empty', requested_locale: locale, resolved_locale: 'fr', kind: 'translation' });
      for (let n = 0; n < 65; n++) {
        if (phase === 0 || (phase === 3 && n % 2 === 0)) continue;
        expected.push({ table_id, key: `k${String(n).padStart(3, '0')}`, requested_locale: locale,
          resolved_locale: phase !== 3 && n % 2 === 0 ? 'fr' : 'en', kind: 'translation' });
      }
    }
    const first = localization.report({ offset: 0, limit: 128 });
    const rest = localization.report({ offset: 128, limit: 128 });
    const entries = [...first.entries, ...rest.entries];
    const fields = ['table_id', 'key', 'requested_locale', 'resolved_locale', 'kind'] as const;
    function check(offset: number, limit: number) {
      const page = localization.report({ offset, limit });
      const wanted = expected.slice(offset, offset + limit);
      if (page.total !== expected.length || page.entries.length !== wanted.length) throw Error('page size');
      page.entries.forEach((row, i) => {
        for (const field of fields) if (row[field] !== wanted[i]![field]) throw Error('page field');
      });
    }
    if (first.total !== expected.length || rest.total !== expected.length || entries.length !== expected.length) throw Error('total');
    let hash = 2166136261;
    entries.forEach((row, i) => {
      for (const field of fields) {
        if (row[field] !== expected[i]![field] || row[field] === null) throw Error('canonical field');
        const text = row[field]!;
        // These identifiers/locales/kind are deliberately ASCII; catalog text is not hashed.
        for (let j = 0; j < text.length; j++) hash = Math.imul(hash ^ text.charCodeAt(j), 16777619) >>> 0;
        hash = Math.imul(hash, 16777619) >>> 0;
      }
    });
    check(0, 1); check(1, 127); check(4096, 128);
    const absent = localization.localize({ table_id: tables[0]!, key: 'absent' });
    if (absent.requested_locale !== locale || absent.resolved_locale !== null
        || absent.missing?.kind !== 'key' || absent.missing.table_id !== tables[0]
        || absent.missing.key !== 'absent' || absent.missing.requested_locale !== locale
        || absent.missing.resolved_locale !== null) throw Error('unknown key');
    check(0, 128); // Unknown queries never append to the prepared coverage report.
    const after = localization.locale();
    if (after.locale !== current.locale || after.pseudo !== current.pseudo
        || JSON.stringify(after.fallbacks) !== JSON.stringify(current.fallbacks)) throw Error('premature commit');
    for (const chunk of chunks) for (let i = 0; i < chunk.ids.length; i++) {
      if (chunk.ids[i] !== '00000000000000000000000001') continue;
      const at = i * chunk.transformStride;
      chunk.transforms[at] = count;
      chunk.transforms[at + 1] = expected.length;
      chunk.transforms[at + 2] = hash;
    }
    log(locale);
  },
} satisfies NativeBehavior;
