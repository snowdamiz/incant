import { Fragment, useState } from 'react';
import type { ComponentSchema, ComponentValue, Diagnostic, EntityDetail } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import { projectState } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { StateView } from '../StateView';
import { AssetInspector } from '../assets/AssetInspector';
import { useAssets } from '../../assets/AssetsContext';
import { FieldView, fieldDomId, safeJson } from './FieldView';
import { sectionKeys } from './presentation';

export function InspectorPanel() {
  const { snapshot, selection } = useShell();
  // The Inspector follows the left column: entities for Hierarchy, assets for Assets.
  const { view } = useAssets();
  const assetsView = view === 'assets';
  const editable = false; // No component-edit command exists on the bridge yet (see NATIVE_VIEWPORT.md).
  const entity = selection !== null ? snapshot?.entities[selection] : undefined;
  const project = projectState(snapshot);
  let body;
  if (!snapshot) {
    body = (
      <StateView icon="plug" title="No engine connected" compact>
        <p>Component values appear here once a project is attached.</p>
      </StateView>
    );
  } else if (project.kind === 'loading') {
    body = <StateView icon="entity" spinner title="Opening project…" compact />;
  } else if (project.kind === 'failed') {
    body = (
      <StateView icon="entity" title="No project loaded" compact>
        <p>Component values appear here once the project opens.</p>
      </StateView>
    );
  } else if (assetsView) {
    body = <AssetInspector />;
  } else if (selection === null) {
    body = (
      <StateView icon="entity" title="Nothing selected" compact>
        <p>Select an entity in the hierarchy to inspect its components.</p>
      </StateView>
    );
  } else if (!entity) {
    body = (
      <StateView icon="error" tone="error" title="Entity details unavailable" compact>
        <p>The bridge listed this entity but sent no component data for it.</p>
        <p className="mono subtle">{selection}</p>
      </StateView>
    );
  } else {
    body = <EntityInspector entity={entity} schemas={snapshot.schemas} diagnostics={snapshot.diagnostics} />;
  }
  return (
    <section className="panel" data-region="inspector" aria-labelledby="inspector-title" tabIndex={-1}>
      <header className="panel__header">
        <h2 id="inspector-title" className="panel__title">
          Inspector
        </h2>
        {entity && !editable && !assetsView ? (
          <span
            className="quiet-chip"
            tabIndex={0}
            title="Read-only: field edits need a component command on the engine bus, which the bridge does not provide yet."
          >
            <Icon name="lock" size={12} />
            Read-only
            <span className="visually-hidden">
              : field edits need a component command on the engine bus, which the bridge does not provide yet.
            </span>
          </span>
        ) : null}
      </header>
      <div className="panel__scroll">{body}</div>
    </section>
  );
}

function EntityInspector({
  entity,
  schemas,
  diagnostics,
}: {
  entity: EntityDetail;
  schemas: Readonly<Record<string, ComponentSchema>>;
  diagnostics: readonly Diagnostic[];
}) {
  const mine = diagnostics.filter((d) => d.entity === entity.id);
  return (
    <div className="inspector">
      <div className="inspector__identity">
        <span className={`kind-tile kind--${entity.kind}`} aria-hidden="true">
          <Icon name={iconForKind(entity.kind)} />
        </span>
        <div className="inspector__identity-text">
          <p className="inspector__name" title={entity.name}>
            {entity.name}
          </p>
          <p className="inspector__meta">
            <span className="inspector__kind">{entity.kind}</span>
            <span aria-hidden="true">·</span>
            <span className="inspector__id mono" title={entity.id}>
              <span className="visually-hidden">ID </span>
              {entity.id}
            </span>
          </p>
        </div>
        <CopyIdButton id={entity.id} />
      </div>
      {mine.length > 0 ? (
        <ul className="inspector__problems" aria-label="Problems on this entity">
          {mine.map((d) => (
            <li key={d.id} className={`problem-line problem-line--${d.severity}`}>
              <Icon name={d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info'} size={12} />
              <span>
                {d.message}
                {d.component ? (
                  <a
                    className="problem-line__where mono"
                    href={`#${fieldDomId(entity.id, d.component, (d.path ?? '').split('/').filter(Boolean))}`}
                    onClick={(event) => {
                      event.preventDefault();
                      const target = nearestField(entity.id, d.component ?? '', d.path);
                      target?.scrollIntoView({ block: 'center' });
                      (target?.matches('input,[tabindex]') ? target : target?.querySelector<HTMLElement>('input,[tabindex]'))?.focus();
                    }}
                  >
                    {shortType(d.component)}
                    {d.path ?? ''}
                  </a>
                ) : null}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      {entity.components.length === 0 ? (
        <StateView icon="entity" title="No components" compact>
          <p>{entity.kind === 'scene' ? 'Scenes hold entities, not components.' : 'This entity has no components.'}</p>
        </StateView>
      ) : (
        entity.components.map((component, index) => (
          <ComponentSection
            key={`${component.type}-${index}`}
            entity={entity.id}
            component={component}
            schema={schemas[component.type]}
            diagnostics={mine.filter((d) => d.component === component.type)}
          />
        ))
      )}
    </div>
  );
}

function CopyIdButton({ id }: { id: string }) {
  const { announce } = useShell();
  return (
    <button
      type="button"
      className="tool-button"
      aria-label="Copy entity ID"
      title="Copy entity ID"
      onClick={() => {
        navigator.clipboard
          ?.writeText(id)
          .then(() => announce('Entity ID copied'))
          .catch(() => announce('Could not copy: clipboard access was denied'));
      }}
    >
      <Icon name="copy" size={14} />
    </button>
  );
}

/**
 * The field element for a diagnostic path, or the closest rendered ancestor: a
 * path inside a mismatched union or past a vector axis still lands on its row.
 */
function nearestField(entity: string, component: string, path: string | null): HTMLElement | null {
  const parts = (path ?? '').split('/').filter(Boolean).map((part) => part.replace(/~1/g, '/').replace(/~0/g, '~'));
  for (let length = parts.length; length >= 0; length -= 1) {
    const target = document.getElementById(fieldDomId(entity, component, parts.slice(0, length)));
    if (target) return target;
  }
  return null;
}

function shortType(type: string) {
  return type.replace(/^incant\./, '');
}

function ComponentSection({
  entity,
  component,
  schema,
  diagnostics,
}: {
  entity: string;
  component: ComponentValue;
  schema: ComponentSchema | undefined;
  diagnostics: readonly Diagnostic[];
}) {
  const [open, setOpen] = useState(true);
  const bodyId = `component-${entity}-${component.type}`.replace(/[^A-Za-z0-9_-]/g, '_');
  const errors = diagnostics.filter((d) => d.severity === 'error').length;
  const warnings = diagnostics.filter((d) => d.severity === 'warning').length;
  const keys = schema ? orderedKeys(schema) : [];
  return (
    <section className="component" aria-label={schema?.title ?? component.type}>
      <h3 className="component__heading">
        <button
          type="button"
          className="component__toggle"
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={() => setOpen(!open)}
        >
          <Icon name="chevron" size={12} className={`component__chevron${open ? ' is-open' : ''}`} />
          <span className="component__title">{schema?.title ?? shortType(component.type)}</span>
          {(schema?.title ?? shortType(component.type)) !== component.type ? (
            <span className="component__type mono" title={component.type}>
              {component.type}
            </span>
          ) : null}
          {errors ? (
            <span className="badge badge--error">
              <Icon name="error" size={12} />
              {errors}
              <span className="visually-hidden"> {errors === 1 ? 'error' : 'errors'}</span>
            </span>
          ) : null}
          {warnings ? (
            <span className="badge badge--warning">
              <Icon name="warning" size={12} />
              {warnings}
              <span className="visually-hidden"> {warnings === 1 ? 'warning' : 'warnings'}</span>
            </span>
          ) : null}
        </button>
      </h3>
      {open ? (
        <div className="component__body" id={bodyId}>
          {!schema ? (
            <div className="component__notice">
              <p>
                <Icon name="warning" size={12} /> No schema is registered for <code>{component.type}</code>, so its
                fields cannot be presented.
              </p>
              <code className="component__raw">{safeJson(component.value)}</code>
            </div>
          ) : (
            <>
              {schema.version !== component.schemaVersion ? (
                <p className="component__notice">
                  <Icon name="warning" size={12} /> Data is schema version {component.schemaVersion}; the registry has
                  version {schema.version}. Values may be shown incorrectly until migrated.
                </p>
              ) : null}
              {sectionKeys(component.type, keys, (schema.order ?? []).length > 0).map((section, index) => {
                const rows = section.keys.map((key) => {
                  const field = schema.properties[key];
                  return field ? (
                    <FieldView
                      key={key}
                      name={key}
                      schema={field}
                      value={component.value[key]}
                      path={[key]}
                      ctx={{ entity, component: component.type, diagnostics }}
                    />
                  ) : null;
                });
                if (section.title === null) return <Fragment key={`section-${index}`}>{rows}</Fragment>;
                const captionId = `${bodyId}-section-${index}`;
                return (
                  <div key={section.title} className="component__section" role="group" aria-labelledby={captionId}>
                    <p className="component__section-title" id={captionId}>
                      {section.title}
                    </p>
                    {rows}
                  </div>
                );
              })}
              {Object.keys(component.value)
                .filter((key) => !(key in schema.properties))
                .map((key) => (
                  <p key={key} className="component__notice">
                    <Icon name="warning" size={12} /> Field <code>{key}</code> is not in the schema:{' '}
                    <code>{safeJson(component.value[key])}</code>
                  </p>
                ))}
            </>
          )}
        </div>
      ) : null}
    </section>
  );
}

function orderedKeys(schema: ComponentSchema): string[] {
  const all = Object.keys(schema.properties);
  const ordered = (schema.order ?? []).filter((key) => all.includes(key));
  return [...ordered, ...all.filter((key) => !ordered.includes(key)).sort()];
}
