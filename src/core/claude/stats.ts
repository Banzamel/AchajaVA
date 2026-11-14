// Texts and bar heights of the token statistics: short token counts, the estimated cost in
// USD, response times and the daily trend scaled to its largest day.
import type { StatDay } from '../../platform/index.ts';

export function tokensText(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1000) return `${Math.round(n / 1000)}k`;
  return String(n);
}

// Small amounts keep two significant digits: a voice command often costs a fraction of a cent.
export function usdText(v: number): string {
  if (v <= 0) return '$0';
  if (v < 0.01) return `$${v.toPrecision(2)}`;
  return `$${v.toFixed(2)}`;
}

export function secondsText(ms: number): string {
  return ms > 0 ? `${(ms / 1000).toFixed(1)} s` : '—';
}

// "2026-11-14" -> "14.11"
export function dayLabel(date: string): string {
  const [, m, d] = date.split('-');
  return m && d ? `${d}.${m}` : date;
}

// Percent heights of the daily bars (a day with any work gets at least a sliver).
export function barHeights(days: StatDay[], by: 'cost' | 'tokens'): number[] {
  const max = Math.max(0, ...days.map((d) => d[by]));
  return days.map((d) => (max > 0 && d[by] > 0 ? Math.max(3, Math.round((d[by] / max) * 100)) : 0));
}
