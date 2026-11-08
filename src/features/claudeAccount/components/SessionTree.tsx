// Projects with Claude sessions (size, last work); opening one lists its sessions with their
// title and flags (active, Achaja's current, an agent's). Ctrl+click selects several.
import type { ReactNode } from 'react';
import { baseName, human, shortDate } from '../../../core/claude/format.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ClaudeService } from '../ClaudeService.ts';

export function SessionTree() {
  const t = useT();
  const svc = useService(ClaudeService);
  const s = useStore(svc.store);
  const row = (key: string, label: string, size: number, when: number, depth: number, project?: string): ReactNode => (
    <div
      key={key}
      className={`tree-row ${s.selected.includes(key) ? 'selected' : ''}`}
      style={{ paddingLeft: 8 + depth * 18 }}
      onClick={(e) => svc.select(key, e.ctrlKey || e.metaKey)}
      onDoubleClick={project ? () => void svc.toggle(project) : undefined}
    >
      {project ? (
        <button type="button" className="tree-toggle" onClick={(e) => { e.stopPropagation(); void svc.toggle(project); }}>
          {s.open.includes(project) ? '▾' : '▸'}
        </button>
      ) : null}
      <span className="grow ellipsis">{label}</span>
      <span className="tree-size">{human(size)}</span>
      <span className="tree-date">{shortDate(when)}</span>
    </div>
  );
  const rows: ReactNode[] = [];
  for (const p of s.projects) {
    rows.push(row(`p:${p.id}`, `${baseName(p.path)}  (${p.sessions})`, p.size, p.last, 0, p.id));
    if (!s.open.includes(p.id)) continue;
    const list = s.sessions[p.id];
    if (!list) rows.push(<div key={`w:${p.id}`} className="tree-row muted" style={{ paddingLeft: 26 }}>{t('claude.loading')}</div>);
    for (const x of list ?? []) {
      const state = x.current ? t('claude.current') : x.active ? t('claude.active') : '';
      const flags = [state, x.agent ? t('claude.agent', { name: x.agent }) : ''].filter(Boolean);
      const label = `${x.active || x.current ? '● ' : ''}${x.title.slice(0, 70)}${flags.length ? `  [${flags.join(', ')}]` : ''}`;
      rows.push(row(`s:${p.id}:${x.id}`, label, x.size, x.mtime, 1));
    }
  }
  return (
    <div className="tree">
      <div className="tree-row head">
        <span className="grow">{t('claude.col_name')}</span>
        <span className="tree-size">{t('claude.col_size')}</span>
        <span className="tree-date">{t('claude.col_date')}</span>
      </div>
      {rows}
    </div>
  );
}
