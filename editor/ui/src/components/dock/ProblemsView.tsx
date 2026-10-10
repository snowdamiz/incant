import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { Diagnostic } from '../../bridge/contract';
import { Icon, iconForKind } from '../../icons/Icon';
import { projectState } from '../../shell/projectState';
import { useShell } from '../../shell/ShellContext';
import { useAssets } from '../../assets/AssetsContext';
import { StateView } from '../StateView';
import { breakAfterSlashes } from '../breakAfterSlashes';
import { Pager, usePage } from './paging';

const SEVERITY_RANK = { error: 0, warning: 1, info: 2 } as const;
const SEVERITY_ICON = { error: 'error', warning: 'warning', info: 'info' } as const;
const SEVERITIES = ['error', 'warning', 'info'] as const;
const SEVERITY_LABEL = { error: 'Errors', warning: 'Warnings', info: 'Info' } as const;

interface ProblemGroup {
  /** The entity's ID, or null for project-level problems. */
  readonly entity: string | null;
  readonly name: string;
  readonly kind: string;
  readonly problems: readonly Diagnostic[];
  readonly errors: number;
  readonly warnings: number;
}

/** A page's run of one group's problems. */
interface PageRun {
  readonly group: ProblemGroup;
  readonly items: Diagnostic[];
  /** The group began on an earlier page. */
  readonly continued: boolean;
  /** Problems of this group on later pages. */
  readonly later: number;
}

/**
 * Problems grouped by the entity they belong to, worst first, with project-level
 * problems in their own group. Severity filters only change what is listed here.
 * Each entity problem is a button that reveals the entity; the entity name is part
 * of its accessible name even though it is shown once, as the group heading.
 *
 * At most one page of problems is mounted. A group that spans pages repeats its
 * heading, marked "continued", and says how many of its problems follow.
 */
export function ProblemsList() {
  const { snapshot, select, announce, showPanel } = useShell();
  // A problem names an entity; showing it means showing the Hierarchy, not the asset list.
  const { setView } = useAssets();
  const [hidden, setHidden] = useState<ReadonlySet<string>>(() => new Set());
  const diagnostics = useMemo(() => snapshot?.diagnostics ?? [], [snapshot?.diagnostics]);
  const nodes = snapshot?.hierarchy.status === 'ready' ? snapshot.hierarchy.value.nodes : {};
  const groups = useMemo(() => {
    const byEntity = new Map<string, Diagnostic[]>();
    for (const d of [...diagnostics].sort((a, b) => SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity])) {
      if (hidden.has(d.severity)) continue;
      const key = d.entity ?? '';
      const list = byEntity.get(key);
      if (list) list.push(d);
      else byEntity.set(key, [d]);
    }
    const list: ProblemGroup[] = [...byEntity].map(([key, problems]) => {
      const node = key ? nodes[key] : undefined;
      return {
        entity: key || null,
        name: key ? (node?.name ?? 'Unknown entity') : 'Project',
        kind: key ? (node?.kind ?? 'entity') : 'project',
        problems,
        errors: problems.filter((d) => d.severity === 'error').length,
        warnings: problems.filter((d) => d.severity === 'warning').length,
      };
    });
    // Worst group first; project-level problems after entities of the same severity.
    return list.sort((a, b) => SEVERITY_RANK[a.problems[0]!.severity] - SEVERITY_RANK[b.problems[0]!.severity] || Number(a.entity === null) - Number(b.entity === null));
  }, [diagnostics, hidden, nodes]);
  const total = useMemo(() => groups.reduce((sum, group) => sum + group.problems.length, 0), [groups]);
  const page = usePage(total);
  const runs = useMemo(() => pageRuns(groups, page.start, page.end), [groups, page.start, page.end]);
  const listRef = useRef<HTMLUListElement>(null);
  // A new page starts at its top.
  useLayoutEffect(() => {
    if (listRef.current) listRef.current.scrollTop = 0;
  }, [page.index]);

  if (!snapshot) return <StateView icon="plug" title="No engine connected" compact />;
  const project = projectState(snapshot);
  // Nothing has been validated while the project opens; never claim "No problems" then.
  if (project.kind === 'loading') {
    return (
      <StateView icon="problems" spinner title="Opening project…" compact>
        <p>Problems appear when validation finishes.</p>
      </StateView>
    );
  }
  if (diagnostics.length === 0 && project.kind === 'failed') {
    return (
      <StateView icon="problems" title="Not validated" compact>
        <p>The project did not open, so it has not been checked.</p>
      </StateView>
    );
  }
  if (diagnostics.length === 0) {
    return (
      <StateView icon="check" tone="success" title="No problems" compact>
        <p>Validation errors and warnings from the engine appear here.</p>
      </StateView>
    );
  }
  const reveal = (d: Diagnostic) => {
    if (d.entity === null) return;
    showPanel('hierarchy');
    showPanel('inspector');
    setView('hierarchy');
    select(d.entity);
    announce(`Selected ${nodes[d.entity]?.name ?? 'entity'}`);
  };
  return (
    <div className="dock-view">
      <div className="dock-toolbar">
        <SeverityFilters
          counts={Object.fromEntries(SEVERITIES.map((severity) => [severity, diagnostics.filter((d) => d.severity === severity).length]))}
          hidden={hidden}
          onToggle={(severity) => {
            setHidden((current) => {
              const next = new Set(current);
              if (next.has(severity)) next.delete(severity);
              else next.add(severity);
              return next;
            });
            // A different set of problems starts at its first page.
            page.go(null);
          }}
        />
        <Pager page={page} noun="problems" announce={announce} />
      </div>
      {groups.length === 0 ? (
        <StateView icon="problems" title="All problems are filtered out" compact>
          <p>Turn a severity back on above to list its problems.</p>
        </StateView>
      ) : (
        <ul ref={listRef} className="list problems" aria-label="Problems">
          {runs.map((run) => {
            const { group } = run;
            const headingId = `problems-group-${group.entity ?? 'project'}`;
            return (
              <li key={group.entity ?? 'project'} className="problems__group">
                <p className="problems__heading" id={headingId}>
                  <Icon name={group.entity ? iconForKind(group.kind) : 'scene'} size={14} className={`kind--${group.kind}`} />
                  <span className="problems__heading-name">{group.name}</span>
                  {run.continued ? <span className="problems__continued">continued</span> : null}
                  <span className="problems__heading-counts">
                    {group.errors ? (
                      <span className="sev--error">
                        <Icon name="error" size={12} />
                        {group.errors}
                        <span className="visually-hidden"> {group.errors === 1 ? 'error' : 'errors'}</span>
                      </span>
                    ) : null}
                    {group.warnings ? (
                      <span className="sev--warning">
                        <Icon name="warning" size={12} />
                        {group.warnings}
                        <span className="visually-hidden"> {group.warnings === 1 ? 'warning' : 'warnings'}</span>
                      </span>
                    ) : null}
                  </span>
                </p>
                <ul className="problems__items" aria-labelledby={headingId}>
                  {run.items.map((d) => {
                    const place = [d.component?.replace(/^incant\./, ''), d.path].filter(Boolean).join(' · ');
                    const content = (
                      <>
                        <Icon name={SEVERITY_ICON[d.severity]} size={14} className={`sev sev--${d.severity}`} />
                        <span className="visually-hidden">{d.severity}: </span>
                        <span className="problems__text">
                          <span className="list__main">{breakAfterSlashes(d.message)}</span>
                          {group.entity ? <span className="visually-hidden">, {group.name}</span> : null}
                          {place ? (
                            <span className="list__meta problems__where">{breakAfterSlashes(place)}</span>
                          ) : group.entity ? null : (
                            <span className="list__meta">Project</span>
                          )}
                        </span>
                      </>
                    );
                    return (
                      <li key={d.id} className="problems__item">
                        {d.entity !== null ? (
                          <button type="button" className="list__row list__row--action problems__row" onClick={() => reveal(d)}>
                            {content}
                          </button>
                        ) : (
                          <div className="list__row problems__row">{content}</div>
                        )}
                      </li>
                    );
                  })}
                  {run.later > 0 ? (
                    <li className="problems__later">
                      {run.later.toLocaleString()} more {run.later === 1 ? 'problem' : 'problems'} on {group.name} on the next {page.index + 2 < page.count ? 'pages' : 'page'}
                    </li>
                  ) : null}
                </ul>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

/** The groups' problems between two flat indices, as runs per group. */
function pageRuns(groups: readonly ProblemGroup[], start: number, end: number): PageRun[] {
  const runs: PageRun[] = [];
  let offset = 0;
  for (const group of groups) {
    const size = group.problems.length;
    const from = Math.max(start, offset);
    const to = Math.min(end, offset + size);
    if (from < to) {
      runs.push({
        group,
        items: group.problems.slice(from - offset, to - offset),
        continued: from > offset,
        later: offset + size - to,
      });
    }
    offset += size;
    if (offset >= end) break;
  }
  return runs;
}

/** Toggle chips that show or hide each severity; each carries its count. */
function SeverityFilters({
  counts,
  hidden,
  onToggle,
}: {
  counts: Record<string, number>;
  hidden: ReadonlySet<string>;
  onToggle: (severity: (typeof SEVERITIES)[number]) => void;
}) {
  return (
    <div className="dock-toolbar__group" role="group" aria-label="Show problem severities">
      {SEVERITIES.map((severity) => (
        <button
          key={severity}
          type="button"
          className={`filter-chip filter-chip--${severity}`}
          aria-pressed={!hidden.has(severity)}
          onClick={() => onToggle(severity)}
        >
          <Icon name={SEVERITY_ICON[severity]} size={12} />
          {SEVERITY_LABEL[severity]}
          <span className="filter-chip__count">{(counts[severity] ?? 0).toLocaleString()}</span>
        </button>
      ))}
    </div>
  );
}
