// Follows the window (maximized, who draws the border) and marks the page for titlebar.css:
// <html data-window-border="drawn"> draws the 1px border around the window.
import { useEffect, useState } from 'react';
import type { WindowFrame, WindowState } from './types';
import { INITIAL_WINDOW_STATE, sameWindowState, watchWindow } from './windowState';

export function useWindowState(frame: WindowFrame): WindowState {
  const [state, setState] = useState(INITIAL_WINDOW_STATE);
  useEffect(() => watchWindow(frame, (next) => setState((prev) => (sameWindowState(prev, next) ? prev : next))), [frame]);
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.windowBorder = state.border;
    return () => {
      delete root.dataset.windowBorder;
    };
  }, [state.border]);
  return state;
}
