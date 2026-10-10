import { Fragment, useMemo, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import type { BridgeSnapshot, ComponentSchema, ComponentValue, Diagnostic, EntityDetail } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import { kindLabel } from '../../assets/paths';
import { projectState } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { COPY_ID_UNAVAILABLE, canWriteClipboard, copyText } from '../../shell/clipboard';
import { StateView } from '../StateView';
import { IdentityHeader } from './IdentityHeader';
import { AssetInspector, useAssetEscape, useAssetInspectorHeader } from '../assets/AssetInspector';
import { useAssets } from '../../assets/AssetsContext';
import type { FieldContext, ReferenceResolver } from './FieldView';
import { FieldView, fieldDomId, severityOf } from './FieldView';
import { breakAfterSlashes } from '../breakAfterSlashes';
import { parsePointer, safeJson } from './format';
import { componentFamily, componentIcon, componentTitle, sectionKeys } from './presentation';
import { FieldRevealProvider, useFieldReveal, useRevealField } from './reveal';

/**
 * The right column's top panel. Its header line names what is inspected (an entity
 * or an asset) instead of repeating "Inspector", so the selection's identity costs
 * no extra height; the panel keeps "Inspector" as its accessible name.
 */
export function InspectorPanel() {
  const { snapshot, selection } = useShell();
  // The Inspector follows the left column: entities for Hierarchy, assets for Assets.
  const { view } = useAssets();
  const assetsView = view === 'assets';
  const onAssetKey = useAssetEscape();
  const assetHeader = useAssetInspectorHeader();
  const entity = selection !== null ? snapshot?.entities[selection] : undefined;
  const project = projectState(snapshot);
  const refs = useReferences(snapshot);
  let header: ReactNode = null;
  let body;
  if (!snapshot) {
    body = (
      <StateView icon="plug" title="No engine connected" compact>
        <p>Component values appear here once a project is attached.</p>
      </StateView>
    );
  } else if (project.kind === 'loading') {
    body = <StateView icon="scene" spinner title="Opening project…" compact />;
  } else if (project.kind === 'failed') {
    body = (
      <StateView icon="scene" title="No project loaded" compact>
        <p>Component values appear here once the project opens.</p>
      </StateView>
    );
  } else if (assetsView) {
    header = assetHeader;
    body = <AssetInspector />;
  } else if (selection === null) {
    body = (
      <StateView icon="pointer" title="Nothing selected" compact>
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
    header = <IdentityHeader icon={iconForKind(entity.kind)} kind={entity.kind} name={entity.name} actions={<CopyIdButton id={entity.id} />} />;
    body = <FieldRevealProvider><EntityInspector entity={entity} schemas={snapshot.schemas} diagnostics={snapshot.diagnostics} refs={refs} /></FieldRevealProvider>;
  }
  return (
    <section
      className="panel panel--inspector"
      data-region="inspector"
      aria-labelledby="inspector-title"
      tabIndex={-1}
      onKeyDown={assetsView ? (event: KeyboardEvent<HTMLElement>) => onAssetKey(event) : undefined}
    >
      <header className={`panel__header inspector-header${header ? ' has-identity' : ''}`}>
        <h2 id="inspector-title" className={header ? 'visually-hidden' : 'panel__title'}>
          Inspector
        </h2>
        {header}
      </header>
      <div className="panel__scroll inspector-scroll">{body}</div>
    </section>
  );
}

/** Exact IDs that name a project asset or entity, for reference fields. Built once per snapshot. */
function useReferences(snapshot: BridgeSnapshot | null): ReferenceResolver {
  const assets = snapshot?.assets;
  const hierarchy = snapshot?.hierarchy;
  return useMemo(() => {
    const byId = new Map((assets?.status === 'ready' ? assets.value : []).map((asset) => [asset.id as string, asset]));
    const nodes = hierarchy?.status === 'ready' ? hierarchy.value.nodes : {};
    return (id: string) => {
      const asset = byId.get(id);
      if (asset) return { name: asset.name, kind: asset.kind, spoken: `${kindLabel(asset.kind)} asset ${asset.name}` };
      const node = Object.hasOwn(nodes, id) ? nodes[id] : undefined;
      if (node) return { name: node.name, kind: node.kind, spoken: `Entity ${node.name}` };
      return null;
    };
  }, [assets, hierarchy]);
}

function EntityInspector({
  entity,
  schemas,
  diagnostics,
  refs,
}: {
  entity: EntityDetail;
  schemas: Readonly<Record<string, ComponentSchema>>;
  diagnostics: readonly Diagnostic[];
  refs: ReferenceResolver;
}) {
  const mine = diagnostics.filter((d) => d.entity === entity.id);
  const editable = false; // No component-edit command exists on the bridge yet (see NATIVE_VIEWPORT.md).
  return (
    <div className="inspector">
      <div className="inspector__meta">
        <span className="inspector__kind">{entityKindLabel(entity.kind)}</span>
        <span className="inspector__id mono" title={entity.id}>
          <span className="visually-hidden">ID </span>
          <MiddleTruncated text={entity.id} />
        </span>
        {!editable ? (
          <span
            className="quiet-chip inspector__readonly"
            tabIndex={0}
            title="Read-only: component values can be inspected here but not edited yet."
          >
            <Icon name="lock" size={12} />
            Read-only
            <span className="visually-hidden">: component values can be inspected here but not edited yet.</span>
          </span>
        ) : null}
      </div>
      {mine.length > 0 ? <EntityProblems entity={entity} problems={mine} /> : null}
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
            refs={refs}
          />
        ))
      )}
    </div>
  );
}

/** Entity kinds as words; unknown kinds are shown as authored. */
function entityKindLabel(kind: string): string {
  return kind ? kind[0]!.toUpperCase() + kind.slice(1) : 'Entity';
}

/**
 * A long ID that keeps both ends visible when space runs out: the head ellipsizes and
 * the last six characters (the random part of a ULID, which tells IDs apart) stay.
 * The DOM holds the whole ID, so selecting or copying it gets every character.
 */
function MiddleTruncated({ text }: { text: string }) {
  const cut = Math.max(0, text.length - 6);
  return (
    <span className="middle-truncate">
      <span className="middle-truncate__head">{text.slice(0, cut)}</span>
      <span className="middle-truncate__tail">{text.slice(cut)}</span>
    </span>
  );
}

/**
 * The entity's problems as one tinted card, worst severity setting the tint. Each
 * line is the engine message, then where it is: the component and exact path, a
 * link that focuses the field.
 */
function EntityProblems({ entity, problems }: { entity: EntityDetail; problems: readonly Diagnostic[] }) {
  const revealField = useRevealField();
  return (
    <ul className={`inspector__problems inspector__problems--${severityOf(problems) ?? 'info'}`} aria-label="Problems on this entity">
      {problems.map((d) => (
        <li key={d.id} className={`problem-line problem-line--${d.severity}`}>
          <Icon name={d.severity === 'error' ? 'error' : d.severity === 'warning' ? 'warning' : 'info'} size={12} />
          <span className="problem-line__text">
            <span className="problem-line__message">{d.message}</span>
            {d.component ? (
              <a
                className="problem-line__where"
                href={`#${fieldDomId(entity.id, d.component, parsePointer(d.path) ?? [])}`}
                onClick={(event) => {
                  event.preventDefault();
                  revealField(entity.id, d.component ?? '', d.path);
                }}
              >
                <span className="problem-line__component">{shortType(d.component)}</span>
                {d.path ? <code className="problem-line__path">{breakAfterSlashes(d.path)}</code> : null}
              </a>
            ) : null}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** Copies the entity ID; without a clipboard it stays focusable, marked unavailable, and says why. */
function CopyIdButton({ id }: { id: string }) {
  const { announce } = useShell();
  const available = canWriteClipboard();
  return (
    <button
      type="button"
      className="tool-button tool-button--small"
      aria-label="Copy entity ID"
      aria-disabled={available ? undefined : true}
      aria-describedby={available ? undefined : 'copy-id-unavailable'}
      title={available ? 'Copy entity ID' : COPY_ID_UNAVAILABLE}
      onClick={() => void copyText(id, 'Entity ID', announce)}
    >
      <Icon name="copy" size={14} />
      {available ? null : (
        <span id="copy-id-unavailable" className="visually-hidden">
          {COPY_ID_UNAVAILABLE}
        </span>
      )}
    </button>
  );
}

function shortType(type: string) {
  return type.replace(/^incant\./, '');
}

function ComponentSection({
  entity,
  component,
  schema,
  diagnostics,
  refs,
}: {
  entity: string;
  component: ComponentValue;
  schema: ComponentSchema | undefined;
  diagnostics: readonly Diagnostic[];
  refs: ReferenceResolver;
}) {
  const [open, setOpen] = useState(true);
  const id = fieldDomId(entity, component.type, []);
  useFieldReveal(id, () => setOpen(true));
  const bodyId = `${id}-body`;
  const errors = diagnostics.filter((d) => d.severity === 'error').length;
  const warnings = diagnostics.filter((d) => d.severity === 'warning').length;
  const keys = schema ? orderedKeys(schema) : [];
  const title = componentTitle(component.type, schema?.title);
  const family = schema ? componentFamily(component.type) : null;
  const typeId = `${bodyId}-type`;
  const ctx: FieldContext = { entity, component: component.type, diagnostics, value: component.value, depth: 0, refs };
  return (
    <section className="component" id={id} aria-label={title} aria-describedby={typeId}>
      <h3 className="component__heading">
        <button
          type="button"
          className="component__toggle"
          aria-expanded={open}
          aria-controls={bodyId}
          title={title === component.type ? title : `${title} (${component.type})`}
          onClick={() => setOpen(!open)}
        >
          <Icon name="chevron" size={12} className={`component__chevron${open ? ' is-open' : ''}`} />
          <span className={`component__mark${family ? ` family--${family}` : ''}`} aria-hidden="true">
            <Icon name={schema ? componentIcon(component.type) : 'component'} size={14} />
          </span>
          <span className="component__title">{title}</span>
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
      <span id={typeId} className="visually-hidden">
        Component type {component.type}
      </span>
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
                    <FieldView key={key} name={key} schema={field} value={component.value[key]} path={[key]} ctx={ctx} />
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
