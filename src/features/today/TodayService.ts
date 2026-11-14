// The Today panel's data: the plan's usage windows, today's token statistics, the plan and the
// phone alerts, refreshed every minute while the panel is shown (each part on its own, so a slow
// or failing one never blanks the others).
import { commands, type PhoneOverview, type TokenStats, type UsageWindow } from '../../platform/index.ts';
import type { PlanRow } from '../../core/plan/format.ts';
import { Store } from '../../shared/lib/Store.ts';

export interface TodayState {
  usage: UsageWindow[];
  stats: TokenStats | null;
  plan: PlanRow[];
  phone: PhoneOverview | null;
  loadedAt: number;
}

export class TodayService {
  readonly store = new Store<TodayState>({ usage: [], stats: null, plan: [], phone: null, loadedAt: 0 });

  async refresh() {
    this.store.set({ loadedAt: Date.now() });
    const [usage, stats, plan, phone] = await Promise.all([
      commands.claudeUsage(true).catch(() => this.store.get().usage),
      commands.claudeStats().catch(() => this.store.get().stats),
      commands.planItems().catch(() => this.store.get().plan),
      commands.phoneOverview().catch(() => this.store.get().phone),
    ]);
    this.store.set({ usage, stats, plan, phone });
  }
}
