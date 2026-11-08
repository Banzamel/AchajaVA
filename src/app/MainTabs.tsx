// The tab strip as "pills" with a neon line under it; the gear at its end opens Settings.
import type { TabKey } from '../features/assistant/index.ts';
import { useT } from '../shared/i18n/useT.ts';

const TABS: TabKey[] = ['chat', 'term', 'agents', 'plan', 'claude', 'log'];

export function MainTabs({ tab, onTab }: { tab: TabKey; onTab: (t: TabKey) => void }) {
  const t = useT();
  return (
    <div className="tabs">
      {TABS.map((k) => (
        <button type="button" key={k} className={`tab ${k === tab ? 'selected' : ''}`} onClick={() => onTab(k)}>
          {t(`tab.${k}`)}
        </button>
      ))}
      <button type="button" className="tab gear" title={t('tab.settings')} aria-label={t('tab.settings')} onClick={() => onTab('settings')}>
        ⚙
      </button>
    </div>
  );
}
