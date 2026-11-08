import { describe, expect, it } from 'vitest';
import { busyFor, hintKind, liveText, stateKey } from '../../../src/core/assistant/stateKey';
import type { Screen, Tick } from '../../../src/core/assistant/types';

const screen = (patch: Partial<Screen> = {}): Screen => ({ lines: [], rec: false, input: '', activity: '', busy: false, dialog: false, ...patch });
const tick = (patch: Partial<Tick> = {}): Tick => ({
  state: 'idle', paused: false, heard: '', problem: null, screen: screen(), work: 'idle', work_since: 0, work_kind: '',
  speaking: false, feed: [], chat: [], chat_offset: 0, update: { phase: '', version: '', pct: 0, error: '', manual: false }, tray: true,
  ...patch,
});

describe('stateKey', () => {
  it('follows the listener, the hooks and the screen in priority order', () => {
    expect(stateKey(tick({ state: 'start' }))).toBe('start');
    expect(stateKey(tick({ screen: screen({ rec: true }) }))).toBe('rec');
    expect(stateKey(tick({ state: 'wake' }))).toBe('wake');
    expect(stateKey(tick({ work: 'waiting', work_kind: 'permission_prompt' }))).toBe('wait');
    expect(stateKey(tick({ screen: screen({ dialog: true }) }))).toBe('dialog');
    expect(stateKey(tick({ speaking: true, work: 'busy' }))).toBe('speak');
    expect(stateKey(tick({ work: 'busy' }))).toBe('busy');
    expect(stateKey(tick({ paused: true }))).toBe('paused');
    expect(stateKey(tick())).toBe('idle');
  });

  it('shows how long Achaja works', () => {
    expect(busyFor(tick({ work: 'busy', work_since: 100 }), 165)).toBe('1:05');
    expect(busyFor(tick(), 165)).toBe('');
  });

  it('picks the live text and the hint', () => {
    const t = tick({ heard: 'otwórz', screen: screen({ input: 'otwórz kalkulator' }) });
    expect(liveText('rec', t)).toEqual({ kind: 'dictation', text: 'otwórz kalkulator' });
    expect(liveText('busy', tick({ screen: screen({ activity: '✻ Pondering…' }) })).text).toBe('✻ Pondering…');
    expect(hintKind('wake')).toBe('rec');
    expect(hintKind('wait')).toBe('dialog');
    expect(hintKind('error')).toBe('error');
    expect(hintKind('off')).toBe('off');
    expect(stateKey(tick({ state: 'off' }))).toBe('off');
  });
});
