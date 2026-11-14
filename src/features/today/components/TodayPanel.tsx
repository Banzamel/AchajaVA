// The panel on the right of the conversation: the plan's limits, today's commands, estimated cost
// and response time, what is left in the plan today, and the phone alerts. Hidden in a narrow
// window (styles/shell.css); the Claude tab has the full statistics.
import { secondsText, usdText } from '../../../core/claude/stats.ts';
import { todayPlan } from '../../../core/today/today.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { usePoll } from '../../../shared/hooks/usePoll.ts';
import { useT, useTranslator } from '../../../shared/i18n/useT.ts';
import { UsageBars } from '../../claudeAccount/index.ts';
import { TodayService } from '../TodayService.ts';
import { PhoneCard } from './PhoneCard.tsx';

// the per-model windows ("7 days · Opus") stay in the Claude tab
const MAIN_WINDOWS = ['five_hour', 'seven_day'];

export function TodayPanel() {
  const t = useT();
  const { lang } = useTranslator();
  const svc = useService(TodayService);
  const s = useStore(svc.store);
  usePoll(() => {
    if (Date.now() - svc.store.get().loadedAt > 60_000) void svc.refresh();
  }, 5000);
  const today = s.stats?.today;
  const plan = todayPlan(s.plan, new Date());
  const tile = (label: string, value: string) => (
    <div className="today-tile">
      <span className="muted small">{label}</span>
      <b>{value}</b>
    </div>
  );
  return (
    <aside className="shell-today" aria-label={t('today.title')}>
      <section className="today-card">
        <span className="today-title">{t('today.limits')}</span>
        {s.usage.length ? <UsageBars windows={s.usage.filter((w) => MAIN_WINDOWS.includes(w.key))} lang={lang} /> : <span className="muted small">{t('claude.loading')}</span>}
      </section>
      <div className="today-tiles">
        {tile(t('today.runs'), String(today?.count ?? 0))}
        {tile(t('today.cost'), usdText(today?.cost ?? 0))}
        {tile(t('today.time'), secondsText(today?.avg_ms ?? 0))}
      </div>
      <section className="today-card">
        <span className="today-title">{t('today.plan')}</span>
        {plan.length ? (
          plan.map((i) => (
            <div key={i.id} className="today-plan">
              <span className="today-time">{i.time}</span>
              <span className="grow ellipsis">{i.label}</span>
            </div>
          ))
        ) : (
          <span className="muted small">{t('today.plan_empty')}</span>
        )}
      </section>
      <PhoneCard phone={s.phone} />
    </aside>
  );
}
