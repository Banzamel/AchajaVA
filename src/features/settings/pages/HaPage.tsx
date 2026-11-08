// Home Assistant: the address and a long-lived token (never shown; empty keeps the saved one), a
// connection test, and important mail on the phone through the HA app.
import { useState } from 'react';
import { phoneMode, setPhoneMode, type PhoneMode } from '../../../core/settings/form.ts';
import { commands, errorMessage } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Check } from '../../../shared/ui/Check.tsx';
import { Field } from '../../../shared/ui/Field.tsx';
import { SelectField } from '../../../shared/ui/SelectField.tsx';
import { SettingsService } from '../SettingsService.ts';

export function HaPage() {
  const t = useT();
  const svc = useService(SettingsService);
  const { form } = useStore(svc.store);
  const [status, setStatus] = useState('');
  const [phones, setPhones] = useState<string[]>([]);
  if (!form) return null;
  const h = form.ha;
  const set = (patch: Partial<typeof h>) => svc.update((f) => ({ ...f, ha: { ...f.ha, ...patch } }));
  const setPhone = (patch: Partial<typeof h.phone>) => set({ phone: { ...h.phone, ...patch } });
  const test = async () => {
    setStatus(t('set.testing'));
    const [ok, text] = await commands.haTest(h.url, h.token).catch((e) => [false, errorMessage(e)] as [boolean, string]);
    setStatus(`${ok ? '✓' : '✗'} ${text}`);
  };
  const find = async () => {
    const names = await commands.haPhones(h.url, h.token).catch(() => []);
    setPhones(names);
    if (!names.length) setStatus(t('set.ph_none'));
    else if (!h.phone.notify_service) setPhone({ notify_service: names[0] });
  };
  const modes: PhoneMode[] = ['both', 'push', 'voice'];
  return (
    <div className="column-gap">
      <Check label={t('set.ha_on')} checked={h.enabled} onChange={(enabled) => set({ enabled })} />
      <div className="page-grid">
        <Field label={t('set.ha_url')} value={h.url} onChange={(url) => set({ url })} />
        <Field label={t('set.ha_token')} type="password" value={h.token} placeholder={h.has_token ? '••••••' : ''} onChange={(token) => set({ token })} />
      </div>
      <div className="row-gap"><Button onClick={() => void test()}>{t('set.test')}</Button><span className="muted">{status}</span></div>
      <fieldset className="card group">
        <legend>{t('set.ph_title')}</legend>
        <Check label={t('set.ph_on')} checked={h.phone.enabled} onChange={(enabled) => setPhone({ enabled })} />
        <div className="page-grid">
          <label className="field">
            <span className="field-label">{t('set.ph_service')}</span>
            <input className="input" list="ha-phones" value={h.phone.notify_service} onChange={(e) => setPhone({ notify_service: e.target.value })} />
            <datalist id="ha-phones">{phones.map((p) => <option key={p} value={p} />)}</datalist>
          </label>
          <div className="align-end"><Button onClick={() => void find()}>{t('set.ph_find')}</Button></div>
          <SelectField label={t('set.ph_mode')} value={phoneMode(h.phone)} options={modes.map((m) => ({ value: m, label: t(`set.ph_${m}`) }))} onChange={(m) => setPhone(setPhoneMode(m as PhoneMode))} />
          <Field label={t('set.ph_hours')} value={h.phone.voice_hours} onChange={(voice_hours) => setPhone({ voice_hours })} />
        </div>
        <Check label={t('set.ph_silent')} checked={h.phone.silent_override} onChange={(silent_override) => setPhone({ silent_override })} />
      </fieldset>
    </div>
  );
}
