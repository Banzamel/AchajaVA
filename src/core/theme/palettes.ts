// Achaja's look: graphite and neon green with cyan, magenta, violet and amber accents, traces like
// on a processor board; by day the same green on a light, minty background. The same palettes as
// the Python windows had.

export type Mode = 'light' | 'dark';
export type Palette = Record<string, string>;

export const PALETTES: Record<Mode, Palette> = {
  light: {
    bg: '#f2f4f4', surface: '#ffffff', surface2: '#eaeeee', hover: '#e0e6e5', pressed: '#d4dbda',
    border: '#d2d9d8', text: '#16201d', muted: '#596864', faint: '#99a6a2',
    accent: '#0a9a5c', accent_hover: '#0cb06a', accent_pressed: '#087a49', on_accent: '#ffffff',
    accent_soft: '#e4f3ec', accent_text: '#087a49', select: '#c9ebd9',
    accent_fill: '#f0f8f4', mine: '#e8f4ef', mine_line: '#8fd3b2',
    green: '#0a9a5c', red: '#d1135a', orange: '#b47500', grey: '#8d9a96', note: '#0a7f9c',
    cyan: '#0a9fbf', violet: '#7a3fe0', trace: '#9fd6bd', trace2: '#9fd5e2', trace3: '#c9b8ee',
  },
  dark: {
    bg: '#0a0c0d', surface: '#111416', surface2: '#161a1d', hover: '#1c2125', pressed: '#22282c',
    border: '#262d31', text: '#e3e8e6', muted: '#95a29d', faint: '#58645f',
    accent: '#19e08a', accent_hover: '#4cf0a8', accent_pressed: '#12b06c', on_accent: '#04110b',
    accent_soft: '#13201b', accent_text: '#62e6ab', select: '#1f3a30',
    accent_fill: '#0f1a16', mine: '#132019', mine_line: '#1f7a52',
    green: '#45e39a', red: '#ff4f8b', orange: '#ffc75a', grey: '#5d6864', note: '#6fd8f0',
    cyan: '#2bd8f5', violet: '#b47cff', trace: '#1f5c43', trace2: '#1b5260', trace3: '#3d3166',
  },
};

// The colour between c1 and c2 (t = 0 -> c1, t = 1 -> c2).
export function blend(c1: string, c2: string, t: number): string {
  const rgb = (c: string) => [1, 3, 5].map((i) => parseInt(c.slice(i, i + 2), 16));
  const a = rgb(c1);
  const b = rgb(c2);
  return `#${a.map((x, i) => Math.round(x + (b[i] - x) * t).toString(16).padStart(2, '0')).join('')}`;
}

// "auto" follows Windows' app mode (the WebView reports it as prefers-color-scheme).
export function resolveMode(pref: string, systemDark: boolean): Mode {
  if (pref === 'light' || pref === 'dark') return pref;
  return systemDark ? 'dark' : 'light';
}

// CSS custom properties for a palette (--c-bg, --c-accent...), plus the title bar's colours.
export function cssVariables(p: Palette): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const [k, v] of Object.entries(p)) vars[`--c-${k.replace(/_/g, '-')}`] = v;
  vars['--titlebar-bg'] = p.bg;
  vars['--titlebar-line'] = p.border;
  vars['--titlebar-text'] = p.accent_text;
  vars['--titlebar-button'] = p.muted;
  vars['--titlebar-button-hover'] = p.hover;
  vars['--titlebar-button-hover-text'] = p.text;
  vars['--titlebar-button-press'] = p.pressed;
  vars['--window-border'] = p.border;
  vars['--c-pill-bg'] = blend(p.surface, p.bg, 0.5);
  vars['--c-pill-line'] = blend(p.trace, p.bg, 0.45);
  vars['--c-pill-text'] = blend(p.muted, p.bg, 0.25);
  return vars;
}
