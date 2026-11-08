// Start-up of a window: language and theme from the configuration, the services, then the
// window is shown once its first frame is drawn (no white flash).
import { AssistantService } from '../features/assistant/index.ts';
import { SettingsService } from '../features/settings/index.ts';
import { commands, currentWindow, insideTauri } from '../platform/index.ts';
import type { ServiceRegistry } from '../shared/hooks/services.tsx';
import { I18nService } from '../shared/i18n/I18nService.ts';
import { ThemeService } from '../shared/lib/ThemeService.ts';

export async function startApp(r: ServiceRegistry, settings: boolean) {
  const i18n = r.get(I18nService);
  const theme = r.get(ThemeService);
  if (!insideTauri()) {
    theme.setPreference('auto');
    return;
  }
  const info = await commands.appInfo();
  i18n.setLanguage(info.language);
  theme.setPreference(info.theme);
  if (settings) {
    await r.get(SettingsService).load();
    return;
  }
  await r.get(AssistantService).start();
  const t = i18n.t;
  void commands.trayTexts({ show: t('tray.show'), talk: t('btn.talk'), pause: t('btn.pause'), resume: t('btn.resume'), quit: t('menu.quit') });
}

// The main window stays hidden when started with --minimized (it is shown from the tray).
export function showWhenDrawn(minimized: boolean) {
  if (!insideTauri() || minimized) return;
  window.requestAnimationFrame(() => window.requestAnimationFrame(() => void currentWindow.show()));
}
