// Mail accounts: switch the watcher on, add / edit / remove accounts, test the login of one.
import { useState } from 'react';
import { newAccount, resultsText } from '../../../core/settings/form.ts';
import type { AccountForm } from '../../../core/settings/types.ts';
import { commands, errorMessage } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Check } from '../../../shared/ui/Check.tsx';
import { AccountDialog } from '../components/AccountDialog.tsx';
import { SettingsService } from '../SettingsService.ts';

export function MailPage() {
  const t = useT();
  const svc = useService(SettingsService);
  const { form } = useStore(svc.store);
  const [sel, setSel] = useState<number | null>(null);
  const [editing, setEditing] = useState<{ index: number | null; account: AccountForm } | null>(null);
  const [status, setStatus] = useState('');
  if (!form) return null;
  const accounts = form.mail.accounts;
  const setAccounts = (list: AccountForm[]) => svc.update((f) => ({ ...f, mail: { ...f.mail, accounts: list } }));
  const saveAccount = (a: AccountForm) => {
    setAccounts(editing?.index === null || editing === null ? [...accounts, a] : accounts.map((x, i) => (i === editing.index ? a : x)));
    setEditing(null);
  };
  const test = async () => {
    if (sel === null) return;
    setStatus(t('set.testing'));
    setStatus(resultsText(await commands.mailTest(accounts[sel]).catch((e) => [[false, errorMessage(e)]] as [boolean, string][])));
  };
  return (
    <div className="column-gap">
      <Check label={t('set.mail_on')} checked={form.mail.enabled} onChange={(enabled) => svc.update((f) => ({ ...f, mail: { ...f.mail, enabled } }))} />
      <div className="side-layout">
        <div className="card list-card tall">
          <div className="list-head"><span className="col">{t('set.col_name')}</span><span className="col">{t('set.col_user')}</span><span className="col">{t('set.col_host')}</span></div>
          {accounts.map((a, i) => (
            <button type="button" key={`${a.key}${i}`} className={`list-row ${sel === i ? 'selected' : ''}`} onClick={() => setSel(i)} onDoubleClick={() => setEditing({ index: i, account: { ...a, password: '' } })}>
              <span className="col">{a.name || a.user}</span><span className="col">{a.user}</span><span className="col">{a.host}</span>
            </button>
          ))}
        </div>
        <div className="side-buttons">
          <Button onClick={() => setEditing({ index: null, account: newAccount() })}>{t('set.add')}</Button>
          <Button disabled={sel === null} onClick={() => sel !== null && setEditing({ index: sel, account: { ...accounts[sel], password: '' } })}>{t('set.edit')}</Button>
          <Button disabled={sel === null} onClick={() => { setAccounts(accounts.filter((_, i) => i !== sel)); setSel(null); }}>{t('set.remove')}</Button>
          <Button disabled={sel === null} onClick={() => void test()}>{t('set.test')}</Button>
        </div>
      </div>
      <pre className="muted status">{status}</pre>
      {editing ? <AccountDialog account={editing.account} onOk={saveAccount} onCancel={() => setEditing(null)} /> : null}
    </div>
  );
}
