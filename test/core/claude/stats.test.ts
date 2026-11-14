import { describe, expect, it } from 'vitest';
import { barHeights, dayLabel, secondsText, tokensText, usdText } from '../../../src/core/claude/stats';

const day = (date: string, cost: number, tokens: number) => ({ date, count: 1, cost, tokens });

describe('token statistics texts', () => {
  it('shortens token counts', () => {
    expect(tokensText(512)).toBe('512');
    expect(tokensText(34_400)).toBe('34k');
    expect(tokensText(2_500_000)).toBe('2.5M');
  });

  it('formats the cost and times', () => {
    expect(usdText(0)).toBe('$0');
    expect(usdText(0.0042)).toBe('$0.0042');
    expect(usdText(1.234)).toBe('$1.23');
    expect(secondsText(4200)).toBe('4.2 s');
    expect(secondsText(0)).toBe('—');
    expect(dayLabel('2026-11-04')).toBe('04.11');
  });

  it('scales the daily bars to the largest day', () => {
    expect(barHeights([day('a', 2, 10), day('b', 0, 0), day('c', 0.01, 1000)], 'cost')).toEqual([100, 0, 3]);
    expect(barHeights([day('a', 0, 10), day('b', 0, 20)], 'tokens')).toEqual([50, 100]);
    expect(barHeights([day('a', 0, 0)], 'cost')).toEqual([0]);
  });
});
