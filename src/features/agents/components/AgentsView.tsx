// Project agents: the list on top (working ones first, green), the selected agent's live progress
// and final report below, Stop for a working one and Remove for a finished one.
import { useState } from 'react';
import { shortDate } from '../../../core/claude/format.ts';
import { commands, type AgentRow } from '../../../platform/index.ts';
import { usePoll } from '../../../shared/hooks/usePoll.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { TextPanel } from '../../../shared/ui/TextPanel.tsx';

export function AgentsView({ onNote }: { onNote: (text: string) => void }) {
  const t = useT();
  const [rows, setRows] = useState<AgentRow[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [log, setLog] = useState('');
  const name = selected ?? rows[0]?.name ?? null;
  const row = rows.find((r) => r.name === name);
  usePoll(() => {
    void commands.agentsList().then(setRows);
    if (name) void commands.agentLog(name).then(([text]) => setLog(text));
  }, 1000);
  const action = async () => {
    if (!row) return;
    if (row.working) {
      onNote(t('note.agent_stopping', { name: row.name }));
      await commands.agentStop(row.name);
    } else {
      await commands.agentRemove(row.name);
      setSelected(null);
    }
  };
  return (
    <div className="tab-page">
      <div className="card list-card">
        {rows.length === 0 ? <div className="muted pad">{t('agents.none')}</div> : null}
        {rows.map((r) => (
          <button type="button" key={r.name} className={`list-row ${r.name === name ? 'selected' : ''} ${r.working ? 'working' : ''}`} onClick={() => setSelected(r.name)}>
            {r.working ? `●  ${r.name}  · ${t('agents.working', { min: r.minutes })}` : `✓  ${r.name}  · ${t('agents.finished', { when: shortDate(r.finished_at) })}`}
          </button>
        ))}
      </div>
      <div className="card grow-card column">
        <div className="card-head">
          <span className="card-title">{name ?? t('agents.pick')}</span>
          <span className={row?.working ? 'green' : 'muted'}>{row ? (row.working ? `● ${t('agents.working', { min: row.minutes })}` : t('agents.done')) : ''}</span>
          <span className="grow" />
          <Button variant="key" disabled={!row} onClick={() => void action()}>{row?.working ? t('agents.stop') : t('agents.remove')}</Button>
        </div>
        <TextPanel text={name ? log : t('agents.none')} mono className="term" />
      </div>
    </div>
  );
}
