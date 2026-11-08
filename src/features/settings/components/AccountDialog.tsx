// One mail account: name, address, server, password (empty keeps the saved one), encryption and
// port (993 for SSL, 143 for STARTTLS), the name and signature for replies; Test before OK.
import { useState } from 'react';
import { finishAccount, portFor, resultsText } from '../../../core/settings/form.ts';
import type { AccountForm } from '../../../core/settings/types.ts';
import { commands, errorMessage } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Field } from '../../../shared/ui/Field.tsx';
import { SelectField } from '../../../shared/ui/SelectField.tsx';
import { TextArea } from '../../../shared/ui/TextArea.tsx';

interface Props {
  account: AccountForm;
  onOk: (a: AccountForm) => void;
  onCancel: () => void;
}

export function AccountDialog({ account, onOk, onCancel }: Props) {
  const t = useT();
  const [a, setA] = useState(account);
  const [status, setStatus] = useState('');
  const set = (patch: Partial<AccountForm>) => setA((x) => ({ ...x, ...patch }));
  const test = async () => {
    if (!finishAccount(a)) return setStatus(t('set.need_host'));
    setStatus(t('set.testing'));
    setStatus(resultsText(await commands.mailTest(a).catch((e) => [[false, errorMessage(e)]] as [boolean, string][])));
  };
  const ok = () => {
    const done = finishAccount(a);
    if (done) onOk(done);
    else setStatus(t('set.need_host'));
  };
  return (
    <div className="modal-backdrop">
      <div className="modal card wide" role="dialog">
        <div className="card-title">{t('set.acc_title')}</div>
        <div className="page-grid">
          <Field label={t('set.acc_name')} value={a.name} onChange={(name) => set({ name })} />
          <Field label={t('set.acc_user')} value={a.user} onChange={(user) => set({ user })} />
          <Field label={t('set.acc_host')} value={a.host} onChange={(host) => set({ host })} />
          <Field label={t('set.acc_pass')} type="password" value={a.password} placeholder={a.has_password ? '••••••' : ''} onChange={(password) => set({ password })} />
          <SelectField label={t('set.acc_sec')} value={a.security} options={['ssl', 'starttls'].map((v) => ({ value: v, label: v }))} onChange={(security) => set({ security, port: portFor(security) })} />
          <Field label={t('set.acc_port')} value={String(a.port)} onChange={(p) => set({ port: Number(p.replace(/\D/g, '')) || 0 })} className="narrow" />
          <Field label={t('set.acc_display')} value={a.display_name} onChange={(display_name) => set({ display_name })} />
          <div />
          <div className="span2"><TextArea label={t('set.acc_sig')} rows={3} value={a.signature} onChange={(signature) => set({ signature })} /></div>
        </div>
        <pre className="muted status">{status}</pre>
        <div className="modal-buttons">
          <Button onClick={() => void test()}>{t('set.test')}</Button>
          <span className="grow" />
          <Button variant="accent" onClick={ok}>{t('btn.ok')}</Button>
          <Button onClick={onCancel}>{t('set.cancel')}</Button>
        </div>
      </div>
    </div>
  );
}
