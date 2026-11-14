// Talk (or Send the recording), and Pause/Resume listening — while recording, Cancel takes the
// pause button's place. Without a listener (turned off) there is nothing to pause.
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { Button } from '../../../shared/ui/Button.tsx';
import { Icon } from '../../../shared/ui/Icon.tsx';
import { AssistantService } from '../AssistantService.ts';

export function ControlBar() {
  const t = useT();
  const svc = useService(AssistantService);
  const { key, tick } = useStore(svc.store);
  const rec = key === 'rec';
  return (
    <div className="control-bar">
      <Button variant="primary" onClick={() => svc.talk()}>
        <Icon name={rec ? 'send' : 'mic'} size={16} />
        <span className="control-label">{rec ? t('btn.send_rec') : t('btn.talk')}</span>
      </Button>
      {rec ? (
        <Button variant="pill" onClick={() => svc.request('cancel')}>{t('btn.cancel')}</Button>
      ) : (
        <Button variant="pill" disabled={key === 'off'} onClick={() => svc.togglePause()}>{tick?.paused ? t('btn.resume') : t('btn.pause')}</Button>
      )}
    </div>
  );
}
