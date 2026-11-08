// One bar per usage window of the plan: the name, the percent and when it resets.
import { resetText, usageColor, windowLabel } from '../../../core/claude/format.ts';
import type { UsageWindow } from '../../../platform/index.ts';

export function UsageBars({ windows, lang }: { windows: UsageWindow[]; lang: string }) {
  return (
    <div className="usage">
      {windows.map((w) => {
        const pct = Math.max(0, Math.min(100, w.percent));
        return (
          <div key={w.key} className="usage-row">
            <div className="usage-head">
              <span className="usage-name">{windowLabel(w.key, lang)}</span>
              <span className="muted small">{pct.toFixed(0)}%  {resetText(w.resets_at, lang, new Date())}</span>
            </div>
            <div className="usage-track">
              <div className={`usage-fill ${usageColor(pct)}`} style={{ width: `${Math.max(pct, 1)}%` }} />
            </div>
          </div>
        );
      })}
    </div>
  );
}
