// Achaja's one window: title bar, the head (avatar, state, menu), live text, update banner, Talk
// and Pause, the hint of voice commands, and the tabs. Settings (and the first-run wizard) take
// the whole window below the title bar until they are closed.
import { AgentsView } from '../features/agents/index.ts';
import { AssistantService, ControlBar, Header, HintChip, LiveBar, UpdateBanner, useQuitRequests } from '../features/assistant/index.ts';
import { ChatView } from '../features/chat/index.ts';
import { ClaudeView } from '../features/claudeAccount/index.ts';
import { LogView } from '../features/log/index.ts';
import { PlanView } from '../features/plan/index.ts';
import { SettingsView } from '../features/settings/index.ts';
import { TerminalView } from '../features/terminal/index.ts';
import { useService, useStore } from '../shared/hooks/services.tsx';
import { useT } from '../shared/i18n/useT.ts';
import { ModalHost } from '../shared/ui/modal/ModalHost.tsx';
import { AppTitleBar } from './AppTitleBar.tsx';
import { MainTabs } from './MainTabs.tsx';

export function MainApp() {
  const t = useT();
  const svc = useService(AssistantService);
  const s = useStore(svc.store);
  useQuitRequests();
  const note = (text: string) => svc.note(text);
  const closeSettings = (saved: boolean) => {
    svc.setTab('chat');
    if (!saved && s.info?.first_run) note(t('note.setup_needed'));
    if (saved && s.info) svc.store.set({ info: { ...s.info, first_run: false } });
  };
  const views = {
    chat: () => <ChatView messages={s.messages} onSend={(text) => svc.request(`type:${text}`)} />,
    term: () => <TerminalView screen={s.tick?.screen ?? null} dialog={s.key === 'dialog' || s.key === 'wait'} />,
    agents: () => <AgentsView onNote={note} />,
    plan: () => <PlanView onNote={note} />,
    claude: () => <ClaudeView onNote={note} />,
    log: () => <LogView lines={s.logs} />,
  };
  return (
    <div className="window">
      <AppTitleBar name={s.tab === 'settings' ? t('set.title') : t('app.name')} />
      {s.tab === 'settings' ? (
        <main className="main settings-root">
          <SettingsView onClose={closeSettings} />
        </main>
      ) : (
        <main className="main">
          <Header />
          <LiveBar />
          <UpdateBanner />
          <ControlBar />
          <HintChip />
          <MainTabs tab={s.tab} onTab={(tab) => svc.setTab(tab)} />
          {views[s.tab]()}
        </main>
      )}
      <ModalHost />
    </div>
  );
}
