import { useId } from 'react';
import type { FieldSchema, PlayEntity } from '../../bridge/contract';
import { unavailableMessage } from '../../bridge/resolve';
import { Icon } from '../../icons/Icon';
import { useOptionalPlay } from '../../play/PlayContext';
import type { AuthoringSnapshot } from '../../shell/ShellContext';
import { useShell } from '../../shell/ShellContext';
import type { FieldContext } from '../inspector/FieldView';
import { FieldView } from '../inspector/FieldView';
import { tickText } from './PlayControls';

const vector = (length: 3 | 4): FieldSchema => ({ type: 'array', items: { type: 'number' }, minItems: length, maxItems: length });

/** The authored schema's field when the host registers it, so runtime rows read like authored ones. */
function fieldOf(snapshot: AuthoringSnapshot | null, component: string, key: string, fallback: FieldSchema): FieldSchema {
  const schema = snapshot?.schemas[component];
  const field = schema && Object.hasOwn(schema.properties, key) ? schema.properties[key] : undefined;
  return field ?? fallback;
}

/**
 * The selected entity in the running native world, above its authored components:
 * Transform and, when the entity has one, Velocity, labelled with the tick they were
 * read at. Read on selection and after a pause or step, never polled; Refresh reads
 * again. Read-only, and never written to the project. Missing runtime entities and
 * other scenes are stated, never filled in from authored values.
 */
export function RuntimeSection() {
  const { capabilities, snapshot, bridge } = useShell();
  const play = useOptionalPlay();
  const titleId = useId();
  const noteId = useId();
  const session = play?.feed.session;
  if (!play || !session || play.feed.transport || (session.phase !== 'running' && session.phase !== 'paused')) return null;
  const { runtime, inspection } = play;
  if (runtime.kind === 'none' || runtime.kind === 'scene') return null;
  const supported = capabilities.has('play.inspect');
  // Only a read of this selection: another entity's values never show here, even for a frame.
  const read = runtime.kind === 'entity' && inspection?.entity === runtime.id ? inspection : null;
  const value = read?.value ?? null;
  const loading = read?.loading === true;
  const stepping = play.busy === 'step';

  let body;
  if (!supported) {
    body = <p className="runtime__state">{unavailableMessage('play.inspect', bridge)}</p>;
  } else if (runtime.kind === 'elsewhere') {
    body = (
      <p className="runtime__state">
        Not in the running scene. {play.playingScene ? `Play runs ${play.playingScene}.` : null}
      </p>
    );
  } else if (value) {
    body = <RuntimeValues value={value} snapshot={snapshot} />;
  } else if (read?.error) {
    // Usually the game removed the entity: a normal outcome, said plainly and quietly.
    body = (
      <p className="runtime__state">
        <span>
          No runtime state. <span className="runtime__message">{read.error}</span>
        </span>
      </p>
    );
  } else {
    body = (
      <p className="runtime__state">
        <span className="spinner spinner--small" aria-hidden="true" />
        Reading runtime state…
      </p>
    );
  }

  return (
    <section className="runtime" aria-labelledby={titleId} aria-describedby={noteId} aria-busy={loading || undefined}>
      <div className="runtime__header">
        <h3 className="runtime__title" id={titleId}>
          Runtime
        </h3>
        {value ? <span className="runtime__tick">tick {tickText(value.tick)}</span> : null}
        {supported && runtime.kind === 'entity' ? (
          <button
            type="button"
            className="tool-button tool-button--small"
            aria-label="Read runtime state again"
            title={value ? `Read again (these values are from tick ${tickText(value.tick)})` : 'Read again'}
            aria-disabled={loading || stepping || undefined}
            onClick={() => {
              if (!loading && !stepping) play.inspect();
            }}
          >
            <Icon name="refresh" size={14} />
          </button>
        ) : null}
      </div>
      <span id={noteId} className="visually-hidden">
        {value
          ? `Read-only values from the running game at tick ${tickText(value.tick)}. They are not saved to the project.`
          : 'Read-only values from the running game. They are not saved to the project.'}
      </span>
      <div className="runtime__body component__body">{body}</div>
    </section>
  );
}

function RuntimeValues({ value, snapshot }: { value: PlayEntity; snapshot: AuthoringSnapshot | null }) {
  // Field IDs in their own namespace, so they never collide with the authored Transform's.
  const ctx: FieldContext = { entity: `runtime-${value.entity}`, component: 'Transform', diagnostics: [], depth: 0 };
  const { translation, rotation, scale } = value.transform;
  return (
    <>
      <FieldView name="translation" schema={fieldOf(snapshot, 'Transform', 'translation', vector(3))} value={translation} path={['translation']} ctx={ctx} />
      <FieldView name="rotation" schema={fieldOf(snapshot, 'Transform', 'rotation', vector(4))} value={rotation} path={['rotation']} ctx={ctx} />
      <FieldView name="scale" schema={fieldOf(snapshot, 'Transform', 'scale', vector(3))} value={scale} path={['scale']} ctx={ctx} />
      {value.velocity ? (
        <FieldView
          name="linear"
          schema={{ ...fieldOf(snapshot, 'Velocity', 'linear', vector(3)), title: 'Velocity' }}
          value={value.velocity.linear}
          path={['linear']}
          ctx={{ ...ctx, component: 'Velocity' }}
        />
      ) : null}
    </>
  );
}
