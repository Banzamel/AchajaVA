// A new version: downloading, ready (installs on quit, or "Install now"), or the result of a
// manual check.
import { commands } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { AssistantService } from '../AssistantService.ts';

export function UpdateBanner() {
  const t = useT();
  const { tick, info } = useStore(useService(AssistantService).store);
  const u = tick?.update;
  if (!u || !u.phase) return null;
  const params = { version: u.version || info?.version || '', pct: u.pct, error: u.error };
  const text: Record<string, string> = {
    checking: u.manual ? t('upd.checking') : '',
    downloading: t('upd.downloading', params),
    ready: t('upd.ready', params),
    available: t('upd.available', params),
    latest: t('upd.none', { version: info?.version ?? '' }),
    error: t('upd.error', params),
  };
  if (!text[u.phase]) return null;
  return (
    <div className="banner update">
      <span className="grow">{text[u.phase]}</span>
      {u.phase === 'ready' ? <Button variant="accent" onClick={() => void commands.updateInstall()}>{t('upd.install')}</Button> : null}
    </div>
  );
}
