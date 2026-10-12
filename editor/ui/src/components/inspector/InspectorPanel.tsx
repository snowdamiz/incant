import { Fragment, useMemo, useState } from 'react';
import type { KeyboardEvent, ReactNode } from 'react';
import type { BridgeSnapshot, ComponentSchema, ComponentValue, Diagnostic, EntityDetail } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import { kindLabel } from '../../assets/paths';
import { projectState } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { COPY_ID_UNAVAILABLE, canWriteClipboard, copyText } from '../../shell/clipboard';
import { StateView } from '../StateView';
import { RuntimeSection } from '../play/RuntimeSection';
import { IdentityHeader } from './IdentityHeader';
import { AssetInspector, useAssetEscape, useAssetInspectorHeader } from '../assets/AssetInspector';
import { useAssets } from '../../assets/AssetsContext';
import type { FieldContext, ReferenceResolver } from './FieldView';
import { FieldView, fieldDomId } from './FieldView';
import { breakAfterSlashes } from '../breakAfterSlashes';
import { parsePointer, safeJson } from './format';
import { componentIcon, componentTitle, sectionKeys } from './presentation';
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
      <StateView icon="plug" title="No engine connected" compact />
    );
  } else if (project.kind === 'loading') {
    body = <StateView icon="scene" title="Opening project…" quiet />;
  } else if (project.kind === 'failed') {
    body = (
      <StateView icon="scene" title="No project loaded" compact />
    );
  } else if (assetsView) {
    header = assetHeader;
    body = <AssetInspector />;
  } else if (selection === null) {
    body = (
      <StateView icon="pointer" title="Nothing selected" compact />
    );
  } else if (!entity) {
    body = (
      <StateView icon="error" tone="error" title="Entity details unavailable" compact>
        <p>The bridge listed this entity but sent no component data for it.</p>
        <p className="mono subtle">{selection}</p>
      </StateView>
    );
  } else {
    header = (
      <IdentityHeader
        icon={iconForKind(entity.kind)}
        kind={entity.kind}
        name={entity.name}
        description={`${entityKindLabel(entity.kind)}, ID ${entity.id}`}
        actions={
          <>
            <ReadOnlyMark />
            <CopyIdButton id={entity.id} />
          </>
        }
      />
    );
    body = (
      <FieldRevealProvider>
        {/* While Play runs: the entity in the native world, above its authored components. */}
        <RuntimeSection />
        <EntityInspector entity={entity} schemas={snapshot.schemas} diagnostics={snapshot.diagnostics} refs={refs} />
      </FieldRevealProvider>
    );
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
  return (
    <div className="inspector">
      {mine.length > 0 ? <EntityProblems entity={entity} problems={mine} /> : null}
      {entity.components.length === 0 ? (
        <StateView icon="entity" title="No components" compact />
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
 * Component values are read-only: no component-edit command exists on the bridge yet
 * (see NATIVE_VIEWPORT.md). One lock glyph in the header line says so, instead of a
 * row; its tooltip and spoken text carry the sentence.
 */
function ReadOnlyMark() {
  return (
    <span className="inspector-header__state" title="Read-only: values can be inspected but not edited yet.">
      <Icon name="lock" size={14} />
      <span className="visually-hidden">Read-only</span>
    </span>
  );
}

/**
 * The entity's problems, at the top: each line is the severity icon, the engine message,
 * then where it is (the component and exact path) as a link that focuses the field.
 */
function EntityProblems({ entity, problems }: { entity: EntityDetail; problems: readonly Diagnostic[] }) {
  const revealField = useRevealField();
  return (
    <ul className="inspector__problems" aria-label="Problems on this entity">
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
  const typeId = `${bodyId}-type`;
  const ctx: FieldContext = { entity, component: component.type, diagnostics, value: component.value, schema, depth: 0, refs };
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
          <Icon name={schema ? componentIcon(component.type) : 'component'} size={14} className="component__icon" />
          <span className="component__title">{title}</span>
          {errors ? (
            <span className="component__count sev--error">
              <Icon name="error" size={12} />
              {errors}
              <span className="visually-hidden"> {errors === 1 ? 'error' : 'errors'}</span>
            </span>
          ) : null}
          {warnings ? (
            <span className="component__count sev--warning">
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
