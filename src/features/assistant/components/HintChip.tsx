// The hint (not clickable): which voice commands work now — the wake word while idle, the end
// word, "cancel" and "new conversation" while recording.
import { hintKind } from '../../../core/assistant/stateKey.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { AssistantService } from '../AssistantService.ts';

export function HintChip() {
  const t = useT();
  const { key, info } = useStore(useService(AssistantService).store);
  const w = info?.hint_words ?? { end: '', cancel: '', clear: '' };
  const text = t(`hint.${hintKind(key)}`, { wake: t('chat.achaja'), end: w.end, cancel: w.cancel, clear: w.clear });
  return <div className="hint-chip">{text}</div>;
}
