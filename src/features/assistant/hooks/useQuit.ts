// Quitting: asks first (the X without the tray, or the menu); closes the listener, the hidden
// Claude session and everything in the background, and installs a downloaded update.
import { useEffect, useRef } from 'react';
import { commands, onEvent } from '../../../platform/index.ts';
import { useService } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ModalService } from '../../../shared/ui/modal/ModalService.ts';

export function useQuit(): (ask: boolean) => Promise<void> {
  const t = useT();
  const modal = useService(ModalService);
  return async (ask: boolean) => {
    if (ask && !(await modal.confirm(t('quit.ask'), t('btn.yes'), t('btn.no')))) return;
    await commands.quit(false);
  };
}

// The native side asks when the window's X is clicked and the tray is off.
export function useQuitRequests() {
  const quit = useRef(useQuit());
  quit.current = useQuit();
  useEffect(() => {
    const stop = onEvent<null>('achaja://quit-ask', () => void quit.current(true));
    return () => void stop.then((f) => f());
  }, []);
}
