// The daily cost trend: one bar per day, scaled to the most expensive day, the cost on hover;
// every other day is labelled, counting back from today.
import { barHeights, dayLabel, tokensText, usdText } from '../../../core/claude/stats.ts';
import type { StatDay } from '../../../platform/index.ts';

export function DayBars({ days }: { days: StatDay[] }) {
  const heights = barHeights(days, 'cost');
  return (
    <div className="day-bars">
      {days.map((d, i) => (
        <div key={d.date} className="day-bar" title={`${dayLabel(d.date)}: ${usdText(d.cost)} · ${tokensText(d.tokens)} · ×${d.count}`}>
          <div className="day-fill" style={{ height: `${heights[i]}%` }} />
          <span className="day-label">{(days.length - 1 - i) % 2 ? '' : dayLabel(d.date)}</span>
        </div>
      ))}
    </div>
  );
}
