// Mail rules: what matters to the user (in their own words, for the classifier), always important
// and ignored senders, quiet hours, report times, purging after N days, what is said aloud.
import { ANNOUNCE } from '../../../core/settings/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Check } from '../../../shared/ui/Check.tsx';
import { Field } from '../../../shared/ui/Field.tsx';
import { TextArea } from '../../../shared/ui/TextArea.tsx';
import { SettingsService } from '../SettingsService.ts';

export function RulesPage() {
  const t = useT();
  const svc = useService(SettingsService);
  const { form } = useStore(svc.store);
  if (!form) return null;
  const m = form.mail;
  const set = (patch: Partial<typeof m>) => svc.update((f) => ({ ...f, mail: { ...f.mail, ...patch } }));
  const toggle = (c: string, on: boolean) => set({ announce: on ? [...m.announce, c] : m.announce.filter((x) => x !== c) });
  return (
    <div className="page-grid">
      <div className="span2"><TextArea label={t('set.hint')} rows={3} value={m.important_hint} onChange={(important_hint) => set({ important_hint })} /></div>
      <Field label={t('set.important')} value={m.important_senders} onChange={(important_senders) => set({ important_senders })} />
      <Field label={t('set.ignored')} value={m.ignored_senders} onChange={(ignored_senders) => set({ ignored_senders })} />
      <Field label={t('set.quiet')} value={m.quiet_hours} onChange={(quiet_hours) => set({ quiet_hours })} />
      <Field label={t('set.digest')} value={m.digest_at} onChange={(digest_at) => set({ digest_at })} />
      <Field label={t('set.delete')} value={String(m.delete_after_days)} className="narrow" onChange={(d) => set({ delete_after_days: Number(d.replace(/\D/g, '')) || 0 })} />
      <div />
      <div className="span2 row-gap">
        <span className="field-label">{t('set.announce')}</span>
        {ANNOUNCE.map((c) => <Check key={c} label={t(`set.c_${c}`)} checked={m.announce.includes(c)} onChange={(on) => toggle(c, on)} />)}
      </div>
    </div>
  );
}
