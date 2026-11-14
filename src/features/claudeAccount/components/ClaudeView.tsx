// The Claude tab: the signed-in account and plan, usage bars, then either every Claude Code session
// on this PC by project with tidying (Delete, also several; Clean older than 30 days; open the
// folder) or the token statistics.
import { useState } from 'react';
import { human } from '../../../core/claude/format.ts';
import { commands } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { usePoll } from '../../../shared/hooks/usePoll.ts';
import { useT, useTranslator } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { ModalService } from '../../../shared/ui/modal/ModalService.ts';
import { ClaudeService } from '../ClaudeService.ts';
import { SessionTree } from './SessionTree.tsx';
import { StatsPanel } from './StatsPanel.tsx';
import { UsageBars } from './UsageBars.tsx';

const CLEAN_DAYS = 30;

export function ClaudeView({ onNote }: { onNote: (text: string) => void }) {
  const t = useT();
  const { lang } = useTranslator();
  const svc = useService(ClaudeService);
  const modal = useService(ModalService);
  const s = useStore(svc.store);
  const [view, setView] = useState<'sessions' | 'stats'>('sessions');
  usePoll(() => {
    const st = svc.store.get();
    if (Date.now() - st.loadedAt > 60_000) void svc.refresh(st.projects.length > 0);
  }, 5000);
  const { projects, sessions } = svc.targets();
  const n = projects.length + sessions.length;
  const ask = (): string => {
    if (n > 1) return t('claude.del_many', { sessions: sessions.length, projects: projects.length });
    if (projects.length) return t('claude.del_project', { path: s.projects.find((p) => p.id === projects[0])?.path ?? projects[0] });
    const [p, id] = sessions[0];
    const title = s.sessions[p]?.find((x) => x.id === id)?.title ?? id;
    return t('claude.del_session', { title: title.slice(0, 80) });
  };
  const remove = async () => {
    if (!(await modal.confirm(ask(), t('btn.yes'), t('btn.no')))) return;
    const out = await svc.remove();
    out.errors.forEach(onNote);
    onNote(t('note.freed', { size: human(out.freed) }));
  };
  const clean = async () => {
    const [ns, nf, bytes] = await commands.claudeCleanPreview(CLEAN_DAYS);
    if (!ns && !nf) {
      onNote(t('note.clean_none', { days: CLEAN_DAYS }));
      return;
    }
    const question = t('claude.clean_ask', { sessions: ns, days: CLEAN_DAYS, files: nf, size: human(bytes) });
    if (!(await modal.confirm(question, t('btn.yes'), t('btn.no')))) return;
    onNote(t('note.freed', { size: human(await commands.claudeClean(CLEAN_DAYS)) }));
    void svc.refresh();
  };
  const a = s.account;
  const sub = [a?.plan ? `Claude ${a.plan}` : '', a?.org ?? '', s.usageError ? t('claude.usage_err', { error: s.usageError }) : ''].filter(Boolean).join(' · ');
  const sessionsSize = s.projects.reduce((sum, p) => sum + p.size, 0);
  return (
    <div className="tab-page">
      <div className="card">
        <div className="card-head">
          <span className="card-title grow">{a?.email || (s.loadedAt ? t('claude.acct_none') : t('claude.loading'))}</span>
          <Button variant="key" onClick={() => void svc.refresh()}>{t('btn.refresh')}</Button>
        </div>
        <div className="muted small">{sub}</div>
        <UsageBars windows={s.usage} lang={lang} />
      </div>
      <div className="card grow-card column">
        <div className="card-head">
          <Button variant="chip" selected={view === 'sessions'} onClick={() => setView('sessions')}>{t('claude.view_sessions')}</Button>
          <Button variant="chip" selected={view === 'stats'} onClick={() => setView('stats')}>{t('claude.view_stats')}</Button>
        </div>
        {view === 'stats' ? (
          <StatsPanel stats={s.stats} />
        ) : (
          <>
            <div className="card-title">{t('claude.sessions')}</div>
            <div className="muted small">{s.total ? t('claude.total', { total: human(s.total), sessions: human(sessionsSize) }) : t('claude.loading')}</div>
            <SessionTree />
          </>
        )}
      </div>
      {view === 'sessions' ? (
        <div className="key-row">
          <Button variant="key" disabled={!n} onClick={() => void remove()}>{t('btn.delete')}{n > 1 ? ` (${n})` : ''}</Button>
          <Button variant="key" onClick={() => void clean()}>{t('claude.clean', { days: CLEAN_DAYS })}</Button>
          <span className="grow" />
          <Button variant="key" onClick={() => void commands.claudeOpenDir(projects[0] ?? null)}>{t('btn.folder')}</Button>
        </div>
      ) : null}
    </div>
  );
}
