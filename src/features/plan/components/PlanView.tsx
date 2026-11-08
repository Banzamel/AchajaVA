// The plan: reminders (🔔), timed commands (💬) and background jobs (⚙) with when they run next;
// the selected item's description and its last report; Run now (jobs) and Delete.
import { useState } from 'react';
import { planIcon, planWhen, type PlanRow } from '../../../core/plan/format.ts';
import { commands } from '../../../platform/index.ts';
import { useService } from '../../../shared/hooks/services.tsx';
import { usePoll } from '../../../shared/hooks/usePoll.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { ModalService } from '../../../shared/ui/modal/ModalService.ts';
import { TextPanel } from '../../../shared/ui/TextPanel.tsx';

export function PlanView({ onNote }: { onNote: (text: string) => void }) {
  const t = useT();
  const modal = useService(ModalService);
  const [rows, setRows] = useState<PlanRow[]>([]);
  const [id, setId] = useState<number | null>(null);
  const [detail, setDetail] = useState('');
  const item = rows.find((r) => r.id === id);
  usePoll(() => {
    void commands.planItems().then(setRows);
    if (id === null) return;
    void commands.planDetail(id).then((d) => {
      const [text, report] = d ?? ['', null];
      setDetail(item?.kind === 'job' || report ? `${text}\n\n${report ?? t('plan.no_result')}` : text);
    });
  }, 2000);
  const remove = async () => {
    if (!item || !(await modal.confirm(t('plan.del_ask', { name: item.label }), t('btn.yes'), t('btn.no')))) return;
    await commands.planRemove(item.id);
    setId(null);
  };
  const run = async () => {
    if (!item) return;
    const [name, result] = (await commands.planRun(item.id)).split('|');
    onNote(t('note.plan_started', { name, result }));
  };
  const text = item ? detail : rows.length ? t('plan.pick') : t('plan.empty');
  return (
    <div className="tab-page">
      <div className="card list-card">
        <div className="list-head"><span className="grow">{t('plan.what')}</span><span className="when-col">{t('plan.when')}</span></div>
        {rows.map((r) => (
          <button type="button" key={r.id} className={`list-row ${r.id === id ? 'selected' : ''}`} onClick={() => setId(r.id)}>
            <span className="grow">{planIcon(r.kind)}  {r.label}</span>
            <span className="when-col">{planWhen(r)}</span>
          </button>
        ))}
      </div>
      <div className="key-row">
        <Button variant="key" disabled={item?.kind !== 'job'} onClick={() => void run()}>{t('btn.run_now')}</Button>
        <Button variant="key" disabled={!item} onClick={() => void remove()}>{t('btn.delete')}</Button>
      </div>
      <div className="card grow-card"><TextPanel text={text} /></div>
    </div>
  );
}
