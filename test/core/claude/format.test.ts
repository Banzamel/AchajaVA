import { describe, expect, it } from 'vitest';
import { baseName, human, resetText, usageColor, windowLabel } from '../../../src/core/claude/format';

describe('claude format', () => {
  it('formats sizes', () => {
    expect(human(512)).toBe('512 B');
    expect(human(1536)).toBe('1.5 KB');
    expect(human(5 * 1024 ** 3)).toBe('5.0 GB');
  });

  it('names usage windows', () => {
    expect(windowLabel('five_hour', 'pl')).toBe('5 godzin');
    expect(windowLabel('seven_day_opus', 'en')).toBe('7 days · Opus');
  });

  it('says when a window resets', () => {
    const now = new Date('2026-11-06T10:00:00');
    expect(resetText(new Date('2026-11-06T12:15:00').toISOString(), 'pl', now)).toBe('odnowi się za 2 h 15 min');
    expect(resetText(new Date('2026-11-06T10:40:00').toISOString(), 'en', now)).toBe('resets in 40 min');
    expect(resetText(new Date('2026-11-09T14:00:00').toISOString(), 'pl', now)).toBe('odnowi się w pon 14:00');
    expect(resetText(null, 'pl', now)).toBe('');
  });

  it('colours usage and takes folder names', () => {
    expect([usageColor(10), usageColor(80), usageColor(95)]).toEqual(['green', 'orange', 'red']);
    expect(baseName('C:\\projekty\\sklep\\')).toBe('sklep');
  });
});
