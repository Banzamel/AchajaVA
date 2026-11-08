// The settings window (index.html#settings).
import { SettingsWindow } from '../features/settings/index.ts';
import { useT } from '../shared/i18n/useT.ts';
import { ModalHost } from '../shared/ui/modal/ModalHost.tsx';
import { AppTitleBar } from './AppTitleBar.tsx';

export function SettingsApp() {
  const t = useT();
  return (
    <div className="window">
      <AppTitleBar name={t('set.title')} />
      <main className="main settings-root">
        <SettingsWindow />
      </main>
      <ModalHost />
    </div>
  );
}
