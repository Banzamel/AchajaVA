// The translate function of the current language; the component re-renders on a language change.
import { useService, useStore } from '../hooks/services.tsx';
import { I18nService } from './I18nService.ts';
import type { Translator } from './Translator.ts';

export function useTranslator(): Translator {
  return useStore(useService(I18nService).store).translator;
}

export function useT(): Translator['t'] {
  return useTranslator().t;
}
