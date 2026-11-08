// The current translator; replaced when the language changes, so components using useT re-render.
import { Store } from '../lib/Store.ts';
import { Translator } from './Translator.ts';

export class I18nService {
  readonly store = new Store<{ translator: Translator }>({ translator: new Translator('pl') });

  setLanguage(lang: string) {
    document.documentElement.lang = lang;
    this.store.set({ translator: new Translator(lang) });
  }

  get t() {
    return this.store.get().translator.t;
  }
}
