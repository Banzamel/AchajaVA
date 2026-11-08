// The first run's last step: what will be saved.
import { summaryLines } from '../../../core/settings/form.ts';
import { LANGUAGES } from '../../../core/settings/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { SettingsService } from '../SettingsService.ts';

export function SummaryPage() {
  const t = useT();
  const { form, status } = useStore(useService(SettingsService).store);
  if (!form) return null;
  const words = {
    language: t('set.language'), model: t('set.model'), mic: t('set.sum_mic'), out: t('set.sum_out'), cast: t('set.sum_cast'),
    roots: t('set.sum_roots'), accounts: t('set.sum_accounts'), mailOff: t('set.sum_mail_off'), ha: t('set.sum_ha'), off: t('set.off'),
    systemDefault: t('set.default_dev'),
  };
  return (
    <div className="column-gap">
      <div className="card">
        {summaryLines(form, words, LANGUAGES[form.language] ?? form.language).map((line) => (
          <div key={line} className="summary-line"><span className="accent">✓</span> {line}</div>
        ))}
      </div>
      <div className="muted">{status}</div>
    </div>
  );
}
