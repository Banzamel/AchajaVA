// The Today panel's pure parts: what is left in the plan for today, and how long ago the phone
// alerts last checked the mail.
import type { PlanRow } from '../plan/format.ts';

export interface TodayItem {
  id: number;
  time: string;
  label: string;
  kind: string;
}

// Plan items still to come today, soonest first (at most `limit`).
export function todayPlan(rows: PlanRow[], now: Date, limit = 4): TodayItem[] {
  const end = new Date(now);
  end.setHours(23, 59, 59, 999);
  const p = (n: number) => String(n).padStart(2, '0');
  return rows
    .filter((r) => r.next * 1000 >= now.getTime() && r.next * 1000 <= end.getTime())
    .sort((a, b) => a.next - b.next)
    .slice(0, limit)
    .map((r) => {
      const d = new Date(r.next * 1000);
      return { id: r.id, time: `${p(d.getHours())}:${p(d.getMinutes())}`, label: r.label, kind: r.kind };
    });
}

// "przed chwilą" / "5 min temu" / "2 h temu"; empty when it never checked.
export function agoText(ts: number, now: number, lang: string): string {
  if (!ts) return '';
  const min = Math.max(0, Math.floor((now - ts) / 60));
  const pl = lang === 'pl';
  if (min < 1) return pl ? 'przed chwilą' : 'just now';
  if (min < 60) return pl ? `${min} min temu` : `${min} min ago`;
  const h = Math.floor(min / 60);
  return pl ? `${h} h temu` : `${h} h ago`;
}
