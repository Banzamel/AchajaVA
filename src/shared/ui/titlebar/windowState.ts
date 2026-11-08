// The window's state for the title bar: maximized or not, and who draws the border.
import type { WindowFrame, WindowSize, WindowState } from './types';

// Until the first measurement: a normal window with the border drawn by the page.
export const INITIAL_WINDOW_STATE: WindowState = { maximized: false, border: 'drawn' };

export function windowState(size: WindowSize): WindowState {
  if (size.maximized) return { maximized: true, border: 'none' };
  // The native frame (Tauri's window shadow, used on Windows 11) keeps its resize borders outside
  // the page, so the window is wider than the page and Windows draws the border itself.
  return { maximized: false, border: size.outerWidth > size.innerWidth ? 'native' : 'drawn' };
}

export const sameWindowState = (a: WindowState, b: WindowState) =>
  a.maximized === b.maximized && a.border === b.border;

// Measures the window now and after every resize (maximize and restore resize it too). Resizes come
// in bursts while an edge is dragged, so one measurement runs at a time and one more follows it if
// events arrived meanwhile. Outside the app (a browser tab) the frame fails and nothing changes.
// Returns the function that stops watching.
export function watchWindow(frame: WindowFrame, onChange: (state: WindowState) => void): () => void {
  let stopped = false;
  let running = false;
  let again = false;
  let unlisten: (() => void) | null = null;

  const update = async () => {
    if (running) {
      again = true;
      return;
    }
    running = true;
    try {
      do {
        again = false;
        const state = windowState(await frame.measure());
        if (!stopped) onChange(state);
      } while (again && !stopped);
    } catch {
      // Not inside the app window: keep the current state.
    }
    running = false;
  };

  Promise.resolve()
    .then(() => frame.onResized(() => void update()))
    .then((stop) => {
      if (stopped) stop();
      else unlisten = stop;
    })
    .catch(() => undefined);
  void update();

  return () => {
    stopped = true;
    if (unlisten) unlisten();
  };
}
