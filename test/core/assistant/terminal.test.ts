import { describe, expect, it } from 'vitest';
import { logLine, terminalText, TERMINAL_KEYS } from '../../../src/core/assistant/terminal';

describe('terminal', () => {
  it('shortens separator lines and trims', () => {
    expect(terminalText(['', '──────────────', '> hi   '])).toBe('────\n> hi');
  });

  it('offers the dialog keys', () => {
    expect(TERMINAL_KEYS.map((k) => k.label)).toEqual(['↑', '↓', 'Enter', 'Esc', '1', '2', '3']);
  });

  it('splits log lines and marks errors', () => {
    expect(logLine('12:00:01 BLAD mikrofonu')).toEqual({ time: '12:00:01', text: 'BLAD mikrofonu', error: true });
    expect(logLine('plain').time).toBe('');
  });
});
