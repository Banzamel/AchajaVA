// Where phone alerts come from: Gmail through the claude.ai connector, the mail accounts of the
// Mail section (either works alone), and Google Calendar reminders, which exist only through the
// connector - greyed out until the account has it connected.
import { useEffect, useState } from 'react';
import type { SettingsForm } from '../../../core/settings/types.ts';
import { commands } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Check } from '../../../shared/ui/Check.tsx';
import { Field } from '../../../shared/ui/Field.tsx';

type Phone = SettingsForm['ha']['phone'];

export function PhoneSources({ phone, accounts, onChange }: { phone: Phone; accounts: number; onChange: (patch: Partial<Phone>) => void }) {
  const t = useT();
  const [conn, setConn] = useState<Record<string, string> | null>(null);
  const load = (refresh: boolean) => {
    setConn(null);
    void commands.phoneConnectors(refresh).then(setConn).catch(() => setConn({}));
  };
  useEffect(() => load(false), []);
  const status = (key: string) => {
    if (!conn) return <span className="muted small">{t('set.ph_conn_unknown')}</span>;
    const ok = conn[key] === 'connected';
    return <span className={`small ${ok ? 'conn-ok' : 'muted'}`}>{ok ? `● ${t('set.ph_conn_ok')}` : `○ ${t('set.ph_conn_off')}`}</span>;
  };
  const calendarOk = conn?.calendar === 'connected';
  return (
    <div className="column phone-sources">
      <div className="row-gap">
        <span className="field-label grow">{t('set.ph_sources')}</span>
        <Button variant="key" onClick={() => load(true)}>{t('set.ph_conn_check')}</Button>
      </div>
      <div className="row-gap">
        <Check label={t('set.ph_gmail')} checked={phone.gmail} onChange={(gmail) => onChange({ gmail })} />
        {status('gmail')}
      </div>
      <Check label={accounts ? t('set.ph_imap', { count: accounts }) : t('set.ph_imap_none')} checked={phone.imap} onChange={(imap) => onChange({ imap })} />
      <div className="row-gap">
        <Check label={t('set.ph_calendar')} checked={phone.calendar && calendarOk} disabled={!calendarOk} onChange={(calendar) => onChange({ calendar })} />
        {status('calendar')}
      </div>
      {calendarOk ? (
        <div className="page-grid">
          <Field label={t('set.ph_cal_minutes')} value={String(phone.calendar_minutes || 15)} onChange={(v) => onChange({ calendar_minutes: Number(v.replace(/\D/g, '')) || 15 })} />
        </div>
      ) : conn ? (
        <div className="muted small">{t('set.ph_conn_need', { name: 'Google Calendar' })}</div>
      ) : null}
    </div>
  );
}
