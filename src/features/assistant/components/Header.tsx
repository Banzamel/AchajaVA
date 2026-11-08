// The head of the window: the avatar, the name and the state in its colour (readable from afar),
// the menu button; under it the band of processor traces, lively while Achaja listens, thinks or
// speaks.
import { busyFor } from '../../../core/assistant/stateKey.ts';
import { STATE_COLORS } from '../../../core/assistant/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ThemeService } from '../../../shared/lib/ThemeService.ts';
import { Band } from '../../../shared/ui/band/Band.tsx';
import { AssistantService } from '../AssistantService.ts';
import { AppMenu } from './AppMenu.tsx';
import { Avatar } from './Avatar.tsx';

export function Header() {
  const t = useT();
  const { key, tick } = useStore(useService(AssistantService).store);
  const { palette } = useStore(useService(ThemeService).store);
  const color = palette[STATE_COLORS[key]];
  const time = tick && key === 'busy' ? busyFor(tick, Date.now() / 1000) : '';
  const status = key === 'busy' ? t('st.busy_for', { time }).trim() : t(`st.${key}`);
  const active = ['rec', 'wake', 'busy', 'speak'].includes(key);
  return (
    <>
      <div className="head">
        <Avatar stateKey={key} size={72} />
        <div className="head-info">
          <div className="brand">{t('chat.achaja')}</div>
          <div className="state-line" style={{ color }}>{status}</div>
        </div>
        <AppMenu />
      </div>
      <div className="head-band"><Band height={28} color={color} active={active} /></div>
    </>
  );
}
