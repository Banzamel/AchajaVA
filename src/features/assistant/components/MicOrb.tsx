// The round mic in the head: violet with a soft glow; while Achaja listens, records, thinks or
// speaks a ring in the state's colour pulses around it (a CSS animation, nothing drawn per frame).
import type { StateKey } from '../../../core/assistant/types.ts';
import { Icon } from '../../../shared/ui/Icon.tsx';

const ACTIVE: StateKey[] = ['rec', 'wake', 'busy', 'speak'];

export function MicOrb({ stateKey, color }: { stateKey: StateKey; color: string }) {
  const active = ACTIVE.includes(stateKey);
  const dim = ['paused', 'off', 'closed', 'error'].includes(stateKey);
  return (
    <div className={`orb ${active ? 'active' : ''} ${dim ? 'dim' : ''}`} style={{ ['--orb-ring' as string]: color }} aria-hidden="true">
      <Icon name="mic" size={24} />
    </div>
  );
}
