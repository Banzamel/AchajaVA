// The sections on the left: the logo, one item per tab with its icon, the listener's state at the
// bottom and Settings. In a narrow window only the icons stay (styles/shell.css).
import logo from '../assets/achaja.svg';
import { AssistantService, type TabKey } from '../features/assistant/index.ts';
import { useService, useStore } from '../shared/hooks/services.tsx';
import { useT } from '../shared/i18n/useT.ts';
import { Icon, type IconName } from '../shared/ui/Icon.tsx';

const TABS: [TabKey, IconName][] = [
  ['chat', 'chat'],
  ['term', 'terminal'],
  ['agents', 'agents'],
  ['plan', 'plan'],
  ['claude', 'claude'],
  ['log', 'log'],
];

function listenerText(key: string, paused: boolean): string {
  if (key === 'off') return 'side.off';
  if (paused || key === 'paused') return 'side.paused';
  if (key === 'error') return 'side.error';
  return 'side.listening';
}

export function Sidebar({ tab, onTab }: { tab: TabKey; onTab: (t: TabKey) => void }) {
  const t = useT();
  const { key, tick } = useStore(useService(AssistantService).store);
  const listening = listenerText(key, tick?.paused ?? false);
  const item = (k: TabKey, icon: IconName) => (
    <button type="button" key={k} className={`side-nav ${k === tab ? 'selected' : ''}`} title={t(`tab.${k}`)} onClick={() => onTab(k)}>
      <Icon name={icon} />
      <span className="side-label">{t(`tab.${k}`)}</span>
    </button>
  );
  return (
    <nav className="shell-side" aria-label={t('side.sections')}>
      <div className="side-brand">
        <img src={logo} alt="" width={36} height={36} />
        <div className="side-label column">
          <span className="side-name">{t('chat.achaja')}</span>
          <span className="muted small">{t('side.subtitle')}</span>
        </div>
      </div>
      {TABS.map(([k, icon]) => item(k, icon))}
      <span className="grow" />
      <div className="side-listener" title={t(listening)}>
        <span className={`dot ${listening === 'side.listening' ? 'on' : listening === 'side.error' ? 'err' : ''}`} />
        <span className="side-label small">{t(listening)}</span>
      </div>
      {item('settings', 'settings')}
    </nav>
  );
}
