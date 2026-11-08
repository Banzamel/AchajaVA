// The light or dark look of the windows: app.theme ("auto" follows Windows' app mode, which the
// WebView reports as prefers-color-scheme and updates live). Sets the palette as CSS variables.
import { cssVariables, PALETTES, resolveMode, type Mode, type Palette } from '../../core/theme/palettes.ts';
import { Store } from './Store.ts';

export class ThemeService {
  readonly store = new Store<{ mode: Mode; palette: Palette }>({ mode: 'dark', palette: PALETTES.dark });
  private pref = 'auto';
  private readonly query = window.matchMedia('(prefers-color-scheme: dark)');

  constructor() {
    this.query.addEventListener('change', () => this.apply());
  }

  setPreference(pref: string) {
    this.pref = pref;
    this.apply();
  }

  private apply() {
    const mode = resolveMode(this.pref, this.query.matches);
    const palette = PALETTES[mode];
    const root = document.documentElement;
    for (const [k, v] of Object.entries(cssVariables(palette))) root.style.setProperty(k, v);
    root.dataset.theme = mode;
    root.style.colorScheme = mode;
    this.store.set({ mode, palette });
  }
}
