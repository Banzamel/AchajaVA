// Claude tab texts: sizes, the names of the plan's usage windows and when they reset.

export function human(n: number): string {
  let v = n;
  for (const unit of ['B', 'KB', 'MB', 'GB']) {
    if (v < 1024 || unit === 'GB') return unit === 'B' ? `${v.toFixed(0)} ${unit}` : `${v.toFixed(1)} ${unit}`;
    v /= 1024;
  }
  return `${v.toFixed(1)} GB`;
}

export function windowLabel(key: string, lang: string): string {
  const pl = lang === 'pl';
  const names: [string, string][] = [['five_hour', pl ? '5 godzin' : '5 hours'], ['seven_day', pl ? '7 dni' : '7 days']];
  for (const [prefix, name] of names) {
    if (key === prefix) return name;
    if (key.startsWith(`${prefix}_`)) {
      const rest = key.slice(prefix.length + 1).replace(/_/g, ' ');
      return `${name} · ${rest.charAt(0).toUpperCase()}${rest.slice(1)}`;
    }
  }
  return key.replace(/_/g, ' ');
}

const DAYS = { pl: ['nd', 'pon', 'wt', 'śr', 'czw', 'pt', 'sob'], en: ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'] };

export function resetText(iso: string | null | undefined, lang: string, now: Date): string {
  if (!iso) return '';
  const when = new Date(iso);
  const left = Math.floor((when.getTime() - now.getTime()) / 1000);
  if (Number.isNaN(left) || left <= 0) return '';
  const pl = lang === 'pl';
  if (left < 24 * 3600) {
    const h = Math.floor(left / 3600);
    const m = Math.floor((left % 3600) / 60);
    const span = h ? `${h} h ${m} min` : `${m} min`;
    return pl ? `odnowi się za ${span}` : `resets in ${span}`;
  }
  const day = (pl ? DAYS.pl : DAYS.en)[when.getDay()];
  const clock = `${String(when.getHours()).padStart(2, '0')}:${String(when.getMinutes()).padStart(2, '0')}`;
  return pl ? `odnowi się w ${day} ${clock}` : `resets ${day} ${clock}`;
}

// Usage bar colour: calm (the accent) below 70 %, orange below 90 %, red above.
export function usageColor(pct: number): 'green' | 'orange' | 'red' {
  return pct < 70 ? 'green' : pct < 90 ? 'orange' : 'red';
}

export function shortDate(ts: number): string {
  const d = new Date(ts * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${p(d.getDate())}.${p(d.getMonth() + 1)} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

export function baseName(path: string): string {
  const parts = path.replace(/[\\/]+$/, '').split(/[\\/]/);
  return parts[parts.length - 1] || path;
}
