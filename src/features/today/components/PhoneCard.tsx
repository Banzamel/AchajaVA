// The phone alerts in the Today panel: which sources are on (green when their connector is
// connected or, for the mailboxes, configured) and when the mail was last checked.
import { agoText } from '../../../core/today/today.ts';
import type { PhoneOverview } from '../../../platform/index.ts';
import { useT, useTranslator } from '../../../shared/i18n/useT.ts';

export function PhoneCard({ phone }: { phone: PhoneOverview | null }) {
  const t = useT();
  const { lang } = useTranslator();
  if (!phone) return null;
  const chip = (on: boolean, label: string) => <span key={label} className={`today-chip ${on ? 'on' : ''}`}>{on ? '●' : '○'} {label}</span>;
  const chips = [
    phone.gmail ? chip(phone.gmail_status === 'connected', t('today.gmail')) : null,
    phone.imap && phone.accounts ? chip(true, t('today.boxes', { count: phone.accounts })) : null,
    phone.calendar ? chip(phone.calendar_status === 'connected', t('today.calendar')) : null,
  ].filter(Boolean);
  const ago = agoText(phone.last_check, Date.now() / 1000, lang);
  return (
    <section className="today-card">
      <span className="today-title">{t('today.phone')}</span>
      {phone.enabled ? (
        <>
          <div className="today-chips">{chips}</div>
          {ago ? <span className="muted small">{t('today.checked', { ago })}</span> : null}
        </>
      ) : (
        <span className="muted small">{t('today.phone_off')}</span>
      )}
    </section>
  );
}
