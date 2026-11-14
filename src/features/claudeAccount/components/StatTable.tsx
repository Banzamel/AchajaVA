// A small table of the statistics: one row per kind or model with runs, errors, tokens, the
// estimated cost and the average response time.
import { secondsText, tokensText, usdText } from '../../../core/claude/stats.ts';
import type { StatRow } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';

export function StatTable({ title, rows }: { title: string; rows: StatRow[] }) {
  const t = useT();
  if (!rows.length) return null;
  return (
    <table className="stat-table">
      <thead>
        <tr>
          <th className="left">{title}</th>
          <th>{t('stats.col_count')}</th>
          <th>{t('stats.col_errors')}</th>
          <th>{t('stats.col_tokens')}</th>
          <th>{t('stats.col_cost')}</th>
          <th>{t('stats.col_time')}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.key}>
            <td className="left ellipsis">{r.key}</td>
            <td>{r.count}</td>
            <td className={r.errors ? 'stat-bad' : 'muted'}>{r.errors}</td>
            <td>{tokensText(r.tokens)}</td>
            <td>{usdText(r.cost)}</td>
            <td>{secondsText(r.avg_ms)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
