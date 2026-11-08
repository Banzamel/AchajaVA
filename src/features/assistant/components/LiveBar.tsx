// "Live": the prompt being dictated (Claude's box, or what Vosk hears before it appears) or what
// Claude is doing right now.
import { liveText } from '../../../core/assistant/stateKey.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { AssistantService } from '../AssistantService.ts';

export function LiveBar() {
  const t = useT();
  const { key, tick, info } = useStore(useService(AssistantService).store);
  if (!tick) return null;
  const live = liveText(key, tick);
  let text = live.text;
  if (live.kind === 'dictation') text = text ? `„${text}”` : t('live.rec', { end: info?.hint_words.end ?? '' });
  if (!text) return null;
  return <div className="banner live">{text}</div>;
}
