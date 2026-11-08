import { describe, expect, it } from 'vitest';
import en from '../../../src/lang/en.json';
import pl from '../../../src/lang/pl.json';
import { Translator } from '../../../src/shared/i18n/Translator';

describe('Translator', () => {
  it('fills params and falls back to English, then the key', () => {
    const t = new Translator('pl').t;
    expect(t('agents.working', { min: 3 })).toBe('pracuje 3 min');
    expect(t('missing.key')).toBe('missing.key');
    expect(new Translator('de').lang).toBe('en');
  });

  it('has every text in both languages', () => {
    expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  });
});
