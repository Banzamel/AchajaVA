// Achaja's look: neutral surfaces (white and light grey by day, graphite at night) with violet
// only in the elements that matter - the talk button, the selected section, the user's bubbles,
// the bars and the mic. State colours (green, red, orange, cyan) stay soft enough to rest the eyes.

export type Mode = 'light' | 'dark';
export type Palette = Record<string, string>;

export const PALETTES: Record<Mode, Palette> = {
  light: {
    bg: '#f3f4f7', surface: '#ffffff', surface2: '#f5f6f9', hover: '#eceef3', pressed: '#e2e5ec',
    border: '#e3e6ec', text: '#1f2230', muted: '#5f6475', faint: '#9a9fad',
    accent: '#7c4ddb', accent_hover: '#8b5ce6', accent_pressed: '#6a3fc9', on_accent: '#ffffff',
    accent_soft: '#efe8fd', accent_text: '#6236c4', select: '#e2d6fb',
    accent_fill: '#f6f1fe', mine: '#7c4ddb', mine_line: '#7c4ddb', mine_text: '#ffffff',
    green: '#2e9d6a', red: '#d1435b', orange: '#c27a12', grey: '#8a8f9c', note: '#6236c4',
    cyan: '#2b8fd6', violet: '#7c4ddb', trace: '#d9cdf5', trace2: '#cfe0f2', trace3: '#e3d6fb',
  },
  dark: {
    bg: '#17191e', surface: '#22252c', surface2: '#1c1e24', hover: '#2a2e36', pressed: '#31353f',
    border: '#2e323b', text: '#eceef2', muted: '#a3a8b4', faint: '#6b7080',
    accent: '#7c4ddb', accent_hover: '#8b5ce6', accent_pressed: '#6a3fc9', on_accent: '#ffffff',
    accent_soft: '#2c2443', accent_text: '#c4a8ff', select: '#3a2f5c',
    accent_fill: '#241f33', mine: '#6a3fc9', mine_line: '#6a3fc9', mine_text: '#ffffff',
    green: '#5fd3a0', red: '#ff7a8c', orange: '#f5b259', grey: '#6b7080', note: '#c4a8ff',
    cyan: '#6bb8f2', violet: '#a57bff', trace: '#3a2f5c', trace2: '#23384d', trace3: '#4a3a75',
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
