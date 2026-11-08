// The one state shown in the window, from the listener, the session hooks and the console screen
// (the screen also sees choice dialogs the hooks do not report). The order is the priority.
import type { StateKey, Tick } from './types.ts';

export function isWaiting(t: Pick<Tick, 'work' | 'work_kind'>): boolean {
  return t.work === 'waiting' && t.work_kind === 'permission_prompt';
}

export function stateKey(t: Tick): StateKey {
  const s = t.screen;
  if (t.state === 'closed' || t.state === 'error' || t.state === 'start' || t.state === 'off') return t.state;
  if (t.state === 'recording' || (s && s.rec)) return 'rec';
  if (t.state === 'wake') return 'wake';
  if (isWaiting(t)) return 'wait';
  if (s && s.dialog) return 'dialog';
  if (t.speaking) return 'speak';
  if (t.work === 'busy' || (s && s.busy)) return 'busy';
  if (t.paused || t.state === 'paused') return 'paused';
  return 'idle';
}

// "m:ss" of the work so far ("" when unknown), for "Thinking… 1:05".
export function busyFor(t: Tick, now: number): string {
  if (t.work !== 'busy' || !t.work_since) return '';
  const secs = Math.max(0, Math.floor(now - t.work_since));
  if (!secs) return '';
  return `${Math.floor(secs / 60)}:${String(secs % 60).padStart(2, '0')}`;
}

// The live line under the header: what is being dictated, or what Claude is doing.
export function liveText(key: StateKey, t: Tick): { kind: 'dictation' | 'activity' | 'none'; text: string } {
  if (key === 'rec') return { kind: 'dictation', text: (t.screen && t.screen.input) || t.heard };
  if (key === 'busy') return { kind: 'activity', text: (t.screen && t.screen.activity) || '' };
  return { kind: 'none', text: '' };
}

// Which voice commands the hint chip shows now.
export function hintKind(key: StateKey): 'rec' | 'dialog' | 'paused' | 'start' | 'error' | 'off' | 'idle' {
  if (key === 'rec' || key === 'wake') return 'rec';
  if (key === 'wait' || key === 'dialog') return 'dialog';
  if (key === 'paused') return 'paused';
  if (key === 'error' || key === 'off') return key;
  if (key === 'start' || key === 'closed') return 'start';
  return 'idle';
}
