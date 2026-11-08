// The chosen language's speech model is not downloaded yet: a note and a download button
// (about 50 MB from alphacephei.com, into the data folder).
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { SettingsService } from '../SettingsService.ts';

export function ModelNotice() {
  const t = useT();
  const svc = useService(SettingsService);
  const { missingModel, downloadPct } = useStore(svc.store);
  if (!missingModel) return <div />;
  return (
    <div className="notice">
      <span className="muted">{t('set.lang_missing', { model: missingModel })}</span>
      {downloadPct === null ? (
        <Button onClick={() => void svc.downloadModel()}>{t('set.download')}</Button>
      ) : (
        <span className="muted">{t('set.downloading', { pct: downloadPct })}</span>
      )}
    </div>
  );
}
