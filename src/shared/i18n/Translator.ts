// Translation keys -> texts of the language in config.json (Polish or English). Params are
// interpolated as {name}; a key missing in a language falls back to English, then to the key.
import en from '../../lang/en.json';
import pl from '../../lang/pl.json';

export type Dictionary = Record<string, string>;
export type Params = Record<string, string | number | null | undefined>;

const LANGUAGES: Record<string, Dictionary> = { en, pl };

export class Translator {
  readonly lang: string;
  private readonly dict: Dictionary;

  constructor(lang: string) {
    this.lang = LANGUAGES[lang] ? lang : 'en';
    this.dict = LANGUAGES[this.lang];
  }

  has = (key: string): boolean => key in this.dict || key in en;

  t = (key: string, params: Params = {}): string => {
    const text = this.dict[key] ?? (en as Dictionary)[key] ?? key;
    return text.replace(/\{(\w+)\}/g, (m, name: string) => (name in params ? String(params[name] ?? '') : m));
  };
}
