// Token statistics of the Claude tab: today and the last 7 days (runs, tokens, estimated cost,
// response time), the 14-day cost trend, the split by kind and by model, and the most
// expensive sessions, jobs and agents of the week.
import { secondsText, tokensText, usdText } from '../../../core/claude/stats.ts';
import type { StatTotals, TokenStats } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { DayBars } from './DayBars.tsx';
import { StatTable } from './StatTable.tsx';

function Period({ title, s }: { title: string; s: StatTotals }) {
  const t = useT();
  const total = s.tokens.input + s.tokens.output + s.tokens.cache_read + s.tokens.cache_5m + s.tokens.cache_1h;
  return (
    <div className="stat-period">
      <div className="card-title">{title}</div>
      <div className="stat-tiles">
        <div className="stat-tile">
          <span className="muted small">{t('stats.runs')}</span>
          <b>{s.count}</b>
          {s.errors ? <span className="small stat-bad">{t('stats.errors', { count: s.errors })}</span> : null}
        </div>
        <div className="stat-tile" title={`in ${s.tokens.input} · out ${s.tokens.output} · cache read ${s.tokens.cache_read} · cache write ${s.tokens.cache_5m + s.tokens.cache_1h}`}>
          <span className="muted small">{t('stats.tokens')}</span>
          <b>{tokensText(total)}</b>
          <span className="muted small">out {tokensText(s.tokens.output)}</span>
        </div>
        <div className="stat-tile">
          <span className="muted small">{t('stats.cost')}</span>
          <b>{usdText(s.cost)}</b>
        </div>
        <div className="stat-tile">
          <span className="muted small">{t('stats.time')}</span>
          <b>{secondsText(s.avg_ms)}</b>
          <span className="muted small">p90 {secondsText(s.p90_ms)}</span>
        </div>
      </div>
    </div>
  );
}

export function StatsPanel({ stats }: { stats: TokenStats | null }) {
  const t = useT();
  if (!stats) return <div className="muted small">{t('claude.loading')}</div>;
  if (!stats.days.some((d) => d.count > 0)) return <div className="muted small">{t('stats.empty')}</div>;
  const kind = (k: string) => t(`stats.kind_${k}`);
  return (
    <div className="column stats">
      <Period title={t('stats.today')} s={stats.today} />
      <Period title={t('stats.week')} s={stats.week} />
      <div className="card-title">{t('stats.trend')}</div>
      <DayBars days={stats.days} />
      <StatTable title={t('stats.by_kind')} rows={stats.kinds.map((r) => ({ ...r, key: kind(r.key) }))} />
      <StatTable title={t('stats.by_model')} rows={stats.models} />
      <div className="card-title">{t('stats.top')}</div>
      {stats.top.map((x) => (
        <div key={`${x.kind}:${x.session || x.name}`} className="stat-top">
          <span className="grow ellipsis">{kind(x.kind)}{x.name ? `: ${x.name}` : ''} <span className="muted small">×{x.count}</span></span>
          <span className="muted small">{tokensText(x.tokens)}</span>
          <b>{usdText(x.cost)}</b>
        </div>
      ))}
      {stats.retries ? <div className="muted small">{t('stats.retries', { count: stats.retries })}</div> : null}
      <div className="muted small">{t('stats.note')}</div>
    </div>
  );
}
