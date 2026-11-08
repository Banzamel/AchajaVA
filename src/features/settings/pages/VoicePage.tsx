// Voice and microphone: the wake-word microphone and the local output (picked from the devices
// Windows shows), the speech rate, and a Google Cast speaker (found on the network).
import { useState } from 'react';
import { matchDevice } from '../../../core/settings/form.ts';
import { commands } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Check } from '../../../shared/ui/Check.tsx';
import { Field } from '../../../shared/ui/Field.tsx';
import { SelectField } from '../../../shared/ui/SelectField.tsx';
import { SettingsService } from '../SettingsService.ts';

export function VoicePage() {
  const t = useT();
  const svc = useService(SettingsService);
  const { form, devices } = useStore(svc.store);
  const [found, setFound] = useState('');
  const [searching, setSearching] = useState(false);
  if (!form) return null;
  const v = form.voice;
  const set = (patch: Partial<typeof v>) => svc.update((f) => ({ ...f, voice: { ...f.voice, ...patch } }));
  const list = (names: string[], current: string) => {
    const shown = matchDevice(current, names);
    const all = shown && !names.includes(shown) ? [shown, ...names] : names;
    return [{ value: '', label: t('set.default_dev') }, ...all.map((n) => ({ value: n, label: n }))];
  };
  const find = async () => {
    setSearching(true);
    setFound(t('set.searching'));
    const speakers = await commands.castDiscover();
    setSearching(false);
    if (!speakers.length) return setFound(t('set.none_found'));
    set({ cast_name: speakers[0].name, cast_host: speakers[0].host });
    setFound(t('set.found', { list: speakers.map((s) => `${s.name} (${s.host})`).join(', ') }));
  };
  return (
    <div className="page-grid">
      <SelectField label={t('set.mic')} value={matchDevice(v.microphone, devices.input)} options={list(devices.input, v.microphone)} onChange={(microphone) => set({ microphone })} />
      <SelectField label={t('set.out')} value={matchDevice(v.local_device, devices.output)} options={list(devices.output, v.local_device)} onChange={(local_device) => set({ local_device })} />
      <Field label={t('set.rate')} value={v.rate} onChange={(rate) => set({ rate })} className="narrow" />
      <div />
      <div className="span2"><Check label={t('set.cast')} checked={v.cast_enabled} onChange={(cast_enabled) => set({ cast_enabled })} /></div>
      {v.cast_enabled ? (
        <>
          <Field label={t('set.cast_name')} value={v.cast_name} onChange={(cast_name) => set({ cast_name })} />
          <Field label={t('set.cast_host')} value={v.cast_host} onChange={(cast_host) => set({ cast_host })} />
          <div className="span2"><Check label={t('set.cast_default')} checked={v.default_cast} onChange={(default_cast) => set({ default_cast })} /></div>
          <div className="span2 row-gap">
            <Button disabled={searching} onClick={() => void find()}>{t('set.cast_find')}</Button>
            <span className="muted">{found}</span>
          </div>
        </>
      ) : null}
    </div>
  );
}
