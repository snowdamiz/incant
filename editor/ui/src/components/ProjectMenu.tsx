import { useRef, useState } from 'react';
import { Icon } from '../icons/Icon';
import { useProjectFile } from '../projectFile/ProjectFileContext';
import { BUSY_REASON } from '../projectFile/model';
import type { FileOperation } from '../projectFile/model';
import { useShell } from '../shell/ShellContext';
import type { MenuItem } from './Menu';
import { ContextMenu } from './Menu';
import { comboText, usePlatform } from './ShortcutsDialog';

/** Spoken after the project's name while a file request runs. */
const BUSY_WORDS: Readonly<Record<FileOperation, string>> = {
  save: 'saving',
  review: 'preparing an engine pin review',
  apply: 'pinning the engine version',
};

/**
 * The project's name in the titlebar: the page's heading until a project is open, then
 * also the button for the project file's menu (Save, and Pin engine version, which
 * reviews first). Items the host does not offer stay listed, marked unavailable, and say
 * why when chosen. The menu sits outside the window's drag area.
 */
export function ProjectTitle({ name, ready }: { name: string; ready: boolean }) {
  if (!ready) {
    return (
      <h1 className="titlebar__project is-pending" title={name}>
        {name}
      </h1>
    );
  }
  return <ProjectMenu name={name} />;
}

function ProjectMenu({ name }: { name: string }) {
  const file = useProjectFile();
  const { announce } = useShell();
  const platform = usePlatform();
  const button = useRef<HTMLButtonElement>(null);
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const blocked = file.unavailable ?? (file.busy ? BUSY_REASON[file.busy] : undefined);
  const items: MenuItem[] = [
    {
      id: 'save',
      label: 'Save',
      icon: 'save',
      shortcut: comboText(['Mod', 'S'], platform),
      unavailable: blocked,
      onSelect: file.save,
    },
    {
      id: 'pin',
      label: 'Pin engine version…',
      icon: 'pin',
      unavailable: blocked,
      onSelect: () => file.openPin(button.current),
    },
  ];
  const open = () => {
    const rect = button.current?.getBoundingClientRect();
    if (rect) setAt({ x: rect.left, y: rect.bottom + 4 });
  };
  return (
    <>
      <h1 className="titlebar__project titlebar__project--menu" title={name}>
        <button
          ref={button}
          type="button"
          className="titlebar__project-button"
          aria-haspopup="menu"
          aria-expanded={at !== null}
          aria-label={`Project: ${name}${file.busy ? `, ${BUSY_WORDS[file.busy]}` : ''}`}
          onClick={() => (at ? setAt(null) : open())}
        >
          <span className="titlebar__project-name">{name}</span>
          {file.busy ? (
            <span className="spinner spinner--small titlebar__project-mark" aria-hidden="true" />
          ) : (
            <Icon name="chevron" size={12} className="titlebar__project-mark titlebar__project-chevron" />
          )}
        </button>
      </h1>
      {at ? (
        // Not a drag area: a press on an item must not move the window.
        <div className="titlebar__menu" data-no-drag="">
          <ContextMenu
            label="Project file"
            items={items}
            at={at}
            onClose={(restore) => {
              setAt(null);
              if (restore) button.current?.focus();
            }}
            onUnavailable={announce}
            anchor={button}
          />
        </div>
      ) : null}
    </>
  );
}
