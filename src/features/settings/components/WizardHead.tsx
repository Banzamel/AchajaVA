// The first run (after installing): "Step n of m", the page's title and subtitle, the logo, a
// progress bar, and clickable chips of every step.
import logo from '../../../assets/achaja.png';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { SettingsService } from '../SettingsService.ts';

export function WizardHead() {
  const t = useT();
  const svc = useService(SettingsService);
  const { index } = useStore(svc.store);
  const list = svc.pages();
  const key = list[index];
  return (
    <div className="wizard-head">
      <div className="wizard-title-row">
        <div className="grow">
          <div className="section-label">{t('set.step', { n: index + 1, total: list.length })}</div>
          <div className="page-title">{t(`set.${key}`)}</div>
          <div className="page-sub">{t(`set.${key}_sub`)}</div>
        </div>
        <img src={logo} alt="" className="wizard-logo" />
      </div>
      <div className="progress"><div className="progress-fill" style={{ width: `${((index + 1) / list.length) * 100}%` }} /></div>
      <div className="chips">
        {list.map((k, i) => (
          <Button key={k} variant="chip" selected={i === index} onClick={() => svc.go(i)}>{t(`set.${k}`)}</Button>
        ))}
      </div>
    </div>
  );
}
