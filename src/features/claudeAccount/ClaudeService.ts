// The Claude tab's data: the account and plan usage (refreshed every minute while the tab is
// shown), the projects with sessions (their sessions loaded when a project is opened), the
// selection, and deleting / tidying (each confirmed by the user in the window).
import { commands, errorMessage, type Account, type Project, type Session, type UsageWindow } from '../../platform/index.ts';
import { Store } from '../../shared/lib/Store.ts';

export interface ClaudeState {
  account: Account | null;
  usage: UsageWindow[];
  usageError: string;
  projects: Project[];
  total: number;
  sessions: Record<string, Session[]>;
  open: string[];
  selected: string[]; // "p:<id>" or "s:<project>:<session>"
  loadedAt: number;
  busy: boolean;
}

export class ClaudeService {
  readonly store = new Store<ClaudeState>({ account: null, usage: [], usageError: '', projects: [], total: 0, sessions: {}, open: [], selected: [], loadedAt: 0, busy: false });

  async refresh(usageOnly = false) {
    if (this.store.get().busy) return;
    this.store.set({ busy: true });
    const account = await commands.claudeAccount().catch(() => null);
    this.store.set({ account, usage: await commands.claudeUsage(true).catch(() => []) });
    try {
      this.store.set({ usage: await commands.claudeUsage(false), usageError: '' });
    } catch (e) {
      this.store.set({ usageError: errorMessage(e) });
    }
    if (!usageOnly || !this.store.get().projects.length) {
      const p = await commands.claudeProjects().catch(() => ({ projects: [], total: 0 }));
      this.store.set({ projects: p.projects, total: p.total, sessions: {}, open: [], selected: [] });
    }
    this.store.set({ busy: false, loadedAt: Date.now() });
  }

  async toggle(id: string) {
    const open = this.store.get().open;
    if (open.includes(id)) {
      this.store.set({ open: open.filter((x) => x !== id) });
      return;
    }
    this.store.set({ open: [...open, id] });
    const list = await commands.claudeSessions(id).catch(() => []);
    this.store.set((s) => ({ sessions: { ...s.sessions, [id]: list } }));
  }

  select(key: string, add: boolean) {
    const sel = this.store.get().selected;
    if (!add) this.store.set({ selected: [key] });
    else this.store.set({ selected: sel.includes(key) ? sel.filter((x) => x !== key) : [...sel, key] });
  }

  // Selected projects, and sessions not inside a selected project.
  targets(): { projects: string[]; sessions: [string, string][] } {
    const sel = this.store.get().selected;
    const projects = sel.filter((s) => s.startsWith('p:')).map((s) => s.slice(2));
    const sessions = sel
      .filter((s) => s.startsWith('s:'))
      .map((s) => {
        const [, p, id] = s.split(':');
        return [p, id] as [string, string];
      })
      .filter(([p]) => !projects.includes(p));
    return { projects, sessions };
  }

  async remove(): Promise<{ freed: number; errors: string[] }> {
    const { projects, sessions } = this.targets();
    const out = await commands.claudeDelete(projects, sessions);
    await this.refresh();
    return out;
  }
}
