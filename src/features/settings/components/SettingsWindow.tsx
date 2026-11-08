// The settings window. Editing existing settings: sections in a sidebar, "Save changes" from any
// of them, the window stays open. First run: steps one by one, the summary, then Save closes it.
import { useState } from 'react';
import { commands } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { ModalService } from '../../../shared/ui/modal/ModalService.ts';
import { SettingsService } from '../SettingsService.ts';
import { LanguageDialog } from './LanguageDialog.tsx';
import { ModelNotice } from './ModelNotice.tsx';
import { PageBody } from './PageBody.tsx';
import { Sidebar } from './Sidebar.tsx';
import { WizardHead } from './WizardHead.tsx';

export function SettingsWindow() {
  const t = useT();
  const svc = useService(SettingsService);
  const modal = useService(ModalService);
  const { form, index, status } = useStore(svc.store);
  const [lang, setLang] = useState(false);
  if (!form) return <div className="settings-loading" />;
  const list = svc.pages();
  const key = list[index];
  const close = () => void commands.closeSettings();
  const save = async () => {
    if (!(await svc.save())) return;
    if (form.editing) {
      svc.setStatus(`✓ ${t('set.saved')}`);
      window.setTimeout(() => svc.setStatus(''), 4000);
    } else {
      await modal.alert(t('set.saved'), t('btn.ok'));
      close();
    }
  };
  if (form.editing) {
    return (
      <div className="settings editing">
        <Sidebar onLanguage={() => setLang(true)} />
        <div className="settings-main">
          <div className="settings-head">
            <div className="page-title">{t(`set.${key}`)}</div>
            <div className="page-sub">{t(`set.${key}_sub`)}</div>
          </div>
          <div className="settings-notice"><ModelNotice /></div>
          <PageBody page={key} />
          <div className="settings-nav">
            <Button onClick={close}>{t('set.close')}</Button>
            <span className="grow" />
            <span className="muted">{status}</span>
            <Button variant="accent" onClick={() => void save()}>{t('set.save_changes')}</Button>
          </div>
        </div>
        {lang ? <LanguageDialog onClose={(saved) => { setLang(false); if (saved) svc.setStatus(t('set.lang_saved')); }} /> : null}
      </div>
    );
  }
  const last = key === 'summary';
  return (
    <div className="settings wizard">
      <WizardHead />
      <div className="settings-notice"><ModelNotice /></div>
      <PageBody page={key} />
      <div className="settings-nav">
        <Button onClick={close}>{t('set.cancel')}</Button>
        <span className="grow" />
        <Button disabled={index === 0} onClick={() => svc.go(index - 1)}>{t('set.back')}</Button>
        <Button variant="accent" onClick={() => (last ? void save() : svc.go(index + 1))}>{last ? t('set.save') : `${t('set.next')}  ›`}</Button>
      </div>
    </div>
  );
}
