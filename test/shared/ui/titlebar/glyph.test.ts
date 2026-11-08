import { describe, expect, it } from 'vitest';
import { glyphMetrics, glyphPath } from '../../../../src/shared/ui/titlebar/glyph';

describe('glyphMetrics', () => {
  it('is 10 px with 1 px lines at 100 %', () => {
    expect(glyphMetrics(1)).toEqual({ size: 10, line: 1, shift: 2, cssSize: 10 });
  });

  it('keeps whole screen pixels at 125 % and 150 %', () => {
    expect(glyphMetrics(1.25)).toEqual({ size: 13, line: 1, shift: 3, cssSize: 10.4 });
    expect(glyphMetrics(1.5)).toEqual({ size: 15, line: 2, shift: 3, cssSize: 10 });
    expect(glyphMetrics(2)).toEqual({ size: 20, line: 2, shift: 4, cssSize: 10 });
  });

  it('falls back to 100 % for a broken ratio', () => {
    expect(glyphMetrics(0).size).toBe(10);
    expect(glyphMetrics(Number.NaN).size).toBe(10);
  });
});

describe('glyphPath', () => {
  const m = glyphMetrics(1);

  it('draws minimize as one line across the middle', () => {
    expect(glyphPath('minimize', m)).toBe('M0 4h10v1h-10z');
  });

  it('draws maximize as a square outline', () => {
    expect(glyphPath('maximize', m)).toBe('M0 0h10v10h-10zM1 1v8h8v-8z');
  });

  it('draws restore as a front square and the back square shifted up and right', () => {
    expect(glyphPath('restore', m)).toBe('M0 2h8v8h-8zM1 3v6h6v-6zM2 0h8v1h-8zM9 0h1v8h-1z');
  });

  it('draws close as two diagonals', () => {
    expect(glyphPath('close', glyphMetrics(1.5))).toBe('M0 0L15 15M15 0L0 15');
  });

  it('uses only whole pixels at 125 %', () => {
    for (const kind of ['minimize', 'maximize', 'restore'] as const) {
      const numbers = glyphPath(kind, glyphMetrics(1.25)).match(/-?\d+(\.\d+)?/g) ?? [];
      expect(numbers.every((n) => Number.isInteger(Number(n)))).toBe(true);
    }
  });
});
