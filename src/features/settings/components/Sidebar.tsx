// Changing settings (from Achaja's menu): sections with icons on the left, the processor traces
// under the title, and the language in its own dialog below.
import { PAGE_ICONS, LANGUAGES } from '../../../core/settings/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Band } from '../../../shared/ui/band/Band.tsx';
import { SettingsService } from '../SettingsService.ts';

export function Sidebar({ onLanguage }: { onLanguage: () => void }) {
  const t = useT();
  const svc = useService(SettingsService);
  const { index, form } = useStore(svc.store);
  return (
    <nav className="settings-side">
      <div className="side-title">{t('set.title_short')}</div>
      <div className="side-band"><Band height={14} background="surface" /></div>
      {svc.pages().map((key, i) => (
        <button type="button" key={key} className={`side-item ${i === index ? 'selected' : ''}`} onClick={() => svc.go(i)}>
          <span className="side-icon">{PAGE_ICONS[key]}</span>
          <span>{t(`set.${key}`)}</span>
        </button>
      ))}
      <div className="side-sep" />
      <button type="button" className="side-item" onClick={onLanguage}>
        <span className="side-icon">🌐</span>
        <span>{t('set.language')}: {LANGUAGES[form?.language ?? 'pl'] ?? form?.language}</span>
      </button>
    </nav>
  );
}
