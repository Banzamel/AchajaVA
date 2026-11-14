// The head of the conversation column: the mic orb (it pulses in the state's colour while Achaja
// listens, thinks or speaks), the state with its dot and the hint of voice commands under it,
// Talk and Pause, and the menu.
import { busyFor } from '../../../core/assistant/stateKey.ts';
import { STATE_COLORS } from '../../../core/assistant/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ThemeService } from '../../../shared/lib/ThemeService.ts';
import { AssistantService } from '../AssistantService.ts';
import { AppMenu } from './AppMenu.tsx';
import { ControlBar } from './ControlBar.tsx';
import { HintChip } from './HintChip.tsx';
import { MicOrb } from './MicOrb.tsx';

export function Header() {
  const t = useT();
  const { key, tick } = useStore(useService(AssistantService).store);
  const { palette } = useStore(useService(ThemeService).store);
  const color = palette[STATE_COLORS[key]];
  const time = tick && key === 'busy' ? busyFor(tick, Date.now() / 1000) : '';
  const status = key === 'busy' ? t('st.busy_for', { time }).trim() : t(`st.${key}`);
  return (
    <header className="head">
      <MicOrb stateKey={key} color={color} />
      <div className="head-info">
        <div className="state-line">
          <span className="state-dot" style={{ background: color }} />
          <span className="ellipsis">{status}</span>
        </div>
        <HintChip />
      </div>
      <ControlBar />
      <AppMenu />
    </header>
  );
}
