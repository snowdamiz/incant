import type { Command } from '../src/index';
import type { Project, EngineCompatibility, EnginePinReview } from '../src/generated';

const legacy: Project = { id: 'project', schema_version: 1, name: 'Legacy', scenes: {}, assets: {}, scripts: {}, settings: { tick_rate: 60 }, memory: {} };
const pinned: Project = { ...legacy, engine_version: '=0.0.1' };
const explicitPin = { op: 'set_engine_version', requirement: '=0.0.1' } satisfies Command;
const range = { op: 'set_engine_version', requirement: '>=0.0.1, <0.1.0' } satisfies Command;
// Syntax and matching are checked by the backend; the generated wire type is a string.
// @ts-expect-error Engine requirements cannot be numeric.
const wrongType = { ...legacy, engine_version: 1 } satisfies Project;
// @ts-expect-error Declaring a range cannot select a filesystem path.
const fileWrite = { op: 'set_engine_version', requirement: '=0.0.1', path: 'other.json' } satisfies Command;
// @ts-expect-error Missing requirements cannot become a command.
const missing = { op: 'set_engine_version' } satisfies Command;
function inspect(report: EngineCompatibility, preview: EnginePinReview): string {
    const revision: number = preview.expected_revision;
    const migration: boolean = preview.file_migration_supported;
    const candidate: string = preview.candidate_text;
    return `${report.current_engine}:${report.declaration}:${revision}:${migration}:${candidate}`;
}
void [legacy, pinned, explicitPin, range, wrongType, fileWrite, missing, inspect];
