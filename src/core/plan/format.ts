// The Plan tab's rows: an icon per kind and when it runs next (↻ for repeated items).

export interface PlanRow {
  id: number;
  kind: string;
  label: string;
  next: number;
  repeat: boolean;
  report_mtime: number;
}

const ICONS: Record<string, string> = { say: '🔔', prompt: '💬', job: '⚙' };

export function planIcon(kind: string): string {
  return ICONS[kind] ?? '•';
}

export function planWhen(row: PlanRow): string {
  const d = new Date(row.next * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getDate())}.${p(d.getMonth() + 1)} ${p(d.getHours())}:${p(d.getMinutes())}${row.repeat ? '  ↻' : ''}`;
}

// A list key that changes when the plan changes (rows are redrawn only then).
export function planKey(rows: PlanRow[]): string {
  return rows.map((r) => `${r.id}:${r.next}:${r.report_mtime}`).join('|');
}
