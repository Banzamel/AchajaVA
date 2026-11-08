// The entries of the Achaja menu and what they do.
import { commands, errorMessage } from '../../../platform/index.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useT } from '../../../shared/i18n/useT.ts';
import { ModalService } from '../../../shared/ui/modal/ModalService.ts';
import type { AssistantService } from '../AssistantService.ts';
import { useQuit } from './useQuit.ts';

export interface MenuItem {
  icon: string;
  label: string;
  run: () => void;
}

export function useMenuActions(svc: AssistantService): (MenuItem | null)[] {
  const t = useT();
  const modal = useService(ModalService);
  const { info } = useStore(svc.store);
  const quit = useQuit();
  const backup = async () => {
    try {
      const [file, count] = await commands.backupNow();
      svc.note(t('note.backup_done', { file, count }));
    } catch (e) {
      svc.note(t('note.backup_err', { error: errorMessage(e) }));
    }
  };
  return [
    { icon: '＋', label: t('menu.new'), run: () => svc.request('clear') },
    { icon: '↻', label: t('menu.repeat'), run: () => void commands.repeatLast() },
    null,
    { icon: '⚙', label: t('menu.settings'), run: () => svc.setTab('settings') },
    { icon: '✎', label: t('menu.edit_config'), run: () => void commands.editConfig() },
    null,
    { icon: '▣', label: t('menu.console'), run: () => void commands.toggleConsole() },
    { icon: '⟳', label: t('menu.restart'), run: () => void commands.restartListener() },
    { icon: '⇩', label: t('menu.check_update'), run: () => void commands.updateCheck() },
    { icon: '⛁', label: t('menu.backup'), run: () => void backup() },
    { icon: '▤', label: t('menu.backup_dir'), run: () => void commands.openBackups() },
    { icon: 'ⓘ', label: t('menu.about'), run: () => void modal.alert(t('about.text', { version: info?.version ?? '' }), t('btn.ok')) },
    null,
    { icon: '⏻', label: t('menu.quit'), run: () => void quit(true) },
  ];
}
