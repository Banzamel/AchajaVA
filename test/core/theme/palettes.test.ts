import { describe, expect, it } from 'vitest';
import { blend, cssVariables, PALETTES, resolveMode } from '../../../src/core/theme/palettes';

describe('palettes', () => {
  it('blends colours', () => {
    expect(blend('#000000', '#ffffff', 0.5)).toBe('#808080');
    expect(blend('#19e08a', '#19e08a', 0.3)).toBe('#19e08a');
  });

  it('follows Windows in auto mode', () => {
    expect(resolveMode('auto', true)).toBe('dark');
    expect(resolveMode('auto', false)).toBe('light');
    expect(resolveMode('light', true)).toBe('light');
  });

  it('has the same keys in both modes and sets the title bar colours', () => {
    expect(Object.keys(PALETTES.light).sort()).toEqual(Object.keys(PALETTES.dark).sort());
    const vars = cssVariables(PALETTES.dark);
    expect(vars['--c-accent-text']).toBe('#c4a8ff');
    expect(vars['--titlebar-bg']).toBe(PALETTES.dark.bg);
  });
});
