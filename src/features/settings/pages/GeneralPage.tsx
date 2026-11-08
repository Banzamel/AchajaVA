// General: language (in the first-run wizard; later in its own dialog), the model and effort of
// the Achaja session, the window theme, the window's update/tray/console options, autostart.
import { APP_FLAGS, EFFORTS, LANGUAGES, MODELS, THEMES } from '../../../core/settings/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ThemeService } from '../../../shared/lib/ThemeService.ts';
import { Check } from '../../../shared/ui/Check.tsx';
import { SelectField } from '../../../shared/ui/SelectField.tsx';
import { SettingsService } from '../SettingsService.ts';

export function GeneralPage() {
  const t = useT();
  const svc = useService(SettingsService);
  const theme = useService(ThemeService);
  const { form } = useStore(svc.store);
  if (!form) return null;
  const opts = (list: string[]) => list.map((v) => ({ value: v, label: v }));
  const setLanguage = (language: string) => {
    svc.update((f) => ({ ...f, language }));
    void svc.checkModel(language);
  };
  const setTheme = (value: string) => {
    svc.update((f) => ({ ...f, theme: value }));
    theme.setPreference(value);
  };
  return (
    <div className="page-grid">
      {form.editing ? null : (
        <SelectField label={t('set.language')} value={form.language} onChange={setLanguage} options={Object.entries(LANGUAGES).map(([value, label]) => ({ value, label }))} />
      )}
      <SelectField label={t('set.model')} value={form.model} options={opts(MODELS)} onChange={(model) => svc.update((f) => ({ ...f, model }))} />
      <SelectField label={t('set.effort')} value={form.effort} options={opts(EFFORTS)} onChange={(effort) => svc.update((f) => ({ ...f, effort }))} />
      <SelectField label={t('set.theme')} value={form.theme} options={THEMES.map((v) => ({ value: v, label: t(`set.theme_${v}`) }))} onChange={setTheme} />
      <div className="span2 checks">
        {APP_FLAGS.map((k) => (
          <Check key={k} label={t(`set.app_${k}`)} checked={form.app[k]} onChange={(v) => svc.update((f) => ({ ...f, app: { ...f.app, [k]: v } }))} />
        ))}
        <Check label={t('set.autostart')} checked={form.autostart} onChange={(autostart) => svc.update((f) => ({ ...f, autostart }))} />
      </div>
    </div>
  );
}
