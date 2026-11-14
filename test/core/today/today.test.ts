import { describe, expect, it } from 'vitest';
import { agoText, todayPlan } from '../../../src/core/today/today';

const row = (id: number, at: Date, label: string) => ({ id, kind: 'say', label, next: at.getTime() / 1000, repeat: false, report_mtime: 0 });

describe('today panel', () => {
  it('lists what is left of today, soonest first', () => {
    const now = new Date(2026, 10, 14, 12, 0, 0);
    const rows = [
      row(1, new Date(2026, 10, 14, 18, 0), 'Przegląd poczty'),
      row(2, new Date(2026, 10, 14, 14, 30), 'Rozmowa'),
      row(3, new Date(2026, 10, 14, 9, 0), 'Rano (minęło)'),
      row(4, new Date(2026, 10, 15, 8, 0), 'Jutro'),
    ];
    expect(todayPlan(rows, now).map((i) => `${i.time} ${i.label}`)).toEqual(['14:30 Rozmowa', '18:00 Przegląd poczty']);
    expect(todayPlan(rows, now, 1)).toHaveLength(1);
  });

  it('says how long ago', () => {
    expect(agoText(0, 1000, 'pl')).toBe('');
    expect(agoText(990, 1000, 'pl')).toBe('przed chwilą');
    expect(agoText(1000 - 300, 1000, 'pl')).toBe('5 min temu');
    expect(agoText(1000 - 7200, 1000, 'en')).toBe('2 h ago');
  });
});
