// The current window (show once rendered, which window this page is), events from the native
// side, the folder picker, the clipboard.
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

export const insideTauri = () => isTauri();

export const currentWindow = {
  show: async () => {
    const w = getCurrentWindow();
    await w.show();
    await w.setFocus();
  },
  close: () => getCurrentWindow().close(),
  label: () => (insideTauri() ? getCurrentWindow().label : 'main'),
};

// A native event ("achaja://quit-ask", "achaja://model-progress"); returns the unlisten function.
export function onEvent<T>(name: string, handler: (payload: T) => void): Promise<() => void> {
  if (!insideTauri()) return Promise.resolve(() => undefined);
  return listen<T>(name, (e) => handler(e.payload));
}

export async function pickFolder(): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === 'string' ? picked : null;
}

export async function copyToClipboard(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}
