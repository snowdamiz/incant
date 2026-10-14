/**
 * DETERMINISTIC PROJECT FILE FIXTURE DATA for handoff 0077. Invented content, not engine
 * output. The project is handoff 0045's harbor fixture (0033's invented scene with the
 * real component schemas), renamed; the host offers file access exactly as the bridge
 * requires (`project_file: { available: true }`), except on the older host.
 */
import { fixtureEngine } from '../../0045-native-agent-chat/tools/scenarios.mjs';

function harbor(name = 'Harbor', overrides = {}) {
  const engine = fixtureEngine();
  return {
    ...engine,
    project: { ...engine.project, name },
    viewport_error: 'Project file fixture: a browser page has no native surface.',
    project_file: { available: true },
    ...overrides,
  };
}

const lighthouse = () => {
  const engine = harbor('Lighthouse');
  return { ...engine, project: { ...engine.project, id: '01JA45FX00000000000000NGHT' }, revision: 12 };
};

const base = (extra = {}) => ({ engine: harbor(), other: lighthouse(), requirement: '=0.1.0', ...extra });
const SLOW = 600_000;

export const SCENARIOS = {
  saved: base(),
  unsaved: base({ unsaved: true }),
  pinned: base({ pinned: true }),
  'older-host': base({ engine: harbor('Harbor', { project_file: { available: false } }) }),
  'long-name': base({ engine: harbor('Harbor district night market, revision with a very long working title') }),
  'slow-save': base({ unsaved: true, delays: { save: SLOW } }),
  'slow-review': base({ delays: { review_pin: SLOW } }),
  'slow-apply': base({ delays: { apply_pin: SLOW } }),
  // Copied from the backend's error Display text: a project file changed outside the editor
  // is reported with the shared source error's words.
  'save-fails': base({ unsaved: true, errors: { save: 'script source changed; read it again before editing' } }),
  'apply-fails': base({ errors: { apply_pin: 'document revision conflict: expected 7, current 8' } }),
  // A transport that loses the answer: capture.mjs makes the bridge's request promise reject
  // with this structured error, so the host never receives the request.
  'save-rejected': base({ unsaved: true, rejectRequests: { 'project.save': { code: 'host.closed', message: 'The editor process exited.' } } }),
  'apply-rejected': base({ rejectRequests: { 'project.pin.apply': { code: 'host.closed', message: 'The editor process exited.' } } }),
};
