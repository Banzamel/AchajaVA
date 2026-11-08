// The title bar of the frameless window. This folder is the same in LoL Helper, WinRate and
// PC Cleaner; each app passes its icon, name, translated labels and the window adapter from
// src/platform, and sets the --titlebar-* colours in its theme.

export interface WindowSize {
  maximized: boolean;
  outerWidth: number;
  innerWidth: number;
}

// What the title bar needs from the window (Tauri's current window, see src/platform).
export interface WindowFrame {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  // A close request, so the app's own handling of the X still applies (e.g. hiding to the tray).
  close(): Promise<void>;
  measure(): Promise<WindowSize>;
  onResized(handler: () => void): Promise<() => void>;
}

// Who draws the 1px window border: Windows (the native frame on Windows 11), the page (Windows 10,
// whose frame would have no top edge) or nobody (maximized).
export type WindowBorder = 'native' | 'drawn' | 'none';

export interface WindowState {
  maximized: boolean;
  border: WindowBorder;
}

export interface TitleBarLabels {
  minimize: string;
  maximize: string;
  restore: string;
  close: string;
}

export type GlyphKind = 'minimize' | 'maximize' | 'restore' | 'close';
