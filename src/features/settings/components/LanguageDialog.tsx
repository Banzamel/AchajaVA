// Changing the language (from the sidebar): saved at once (the listener restarts with the new
// control words); a missing speech model is downloaded first.
import { useEffect, useState } from 'react';
import { LANGUAGES } from '../../../core/settings/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { SelectField } from '../../../shared/ui/SelectField.tsx';
import { SettingsService } from '../SettingsService.ts';
import { ModelNotice } from './ModelNotice.tsx';

export function LanguageDialog({ onClose }: { onClose: (saved: boolean) => void }) {
  const t = useT();
  const svc = useService(SettingsService);
  const { form, missingModel } = useStore(svc.store);
  const [lang, setLang] = useState(form?.language ?? 'pl');
  useEffect(() => {
    void svc.checkModel(lang);
  }, [lang, svc]);
  const apply = async () => {
    svc.update((f) => ({ ...f, language: lang }));
    onClose(await svc.save());
  };
  return (
    <div className="modal-backdrop">
      <div className="modal card" role="dialog">
        <div className="page-title">{t('set.lang_title')}</div>
        <div className="page-sub">{t('set.lang_sub')}</div>
        <SelectField label={t('set.language')} value={lang} options={Object.entries(LANGUAGES).map(([value, label]) => ({ value, label }))} onChange={setLang} />
        <ModelNotice />
        <div className="modal-buttons">
          <Button onClick={() => onClose(false)}>{t('set.cancel')}</Button>
          <Button variant="accent" disabled={!!missingModel} onClick={() => void apply()}>{t('set.save')}</Button>
        </div>
      </div>
    </div>
  );
}
