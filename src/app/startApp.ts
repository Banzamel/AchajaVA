// Start-up of the window: language and theme from the configuration, the services, then the
// window is shown once its first frame is drawn (no white flash).
import { AssistantService } from '../features/assistant/index.ts';
import { commands, currentWindow, insideTauri, type AppInfo } from '../platform/index.ts';
import type { ServiceRegistry } from '../shared/hooks/services.tsx';
import { I18nService } from '../shared/i18n/I18nService.ts';
import { ThemeService } from '../shared/lib/ThemeService.ts';

function applyInfo(r: ServiceRegistry, info: AppInfo) {
  const i18n = r.get(I18nService);
  i18n.setLanguage(info.language);
  r.get(ThemeService).setPreference(info.theme);
  const t = i18n.t;
  void commands.trayTexts({ show: t('tray.show'), talk: t('btn.talk'), pause: t('btn.pause'), resume: t('btn.resume'), settings: t('tab.settings'), quit: t('menu.quit') });
}

export async function startApp(r: ServiceRegistry) {
  if (!insideTauri()) {
    r.get(ThemeService).setPreference('auto');
    return;
  }
  // read once: it carries the one-time "start on Settings" flag
  const info = await commands.appInfo();
  applyInfo(r, info);
  await r.get(AssistantService).start(info);
}

// After Settings are saved: the window follows the new language and theme at once.
export async function applyConfig(r: ServiceRegistry) {
  const info = await commands.appInfo();
  applyInfo(r, info);
  r.get(AssistantService).store.set({ info });
}

// The main window stays hidden when started with --minimized (it is shown from the tray).
export function showWhenDrawn(minimized: boolean) {
  if (!insideTauri() || minimized) return;
  window.requestAnimationFrame(() => window.requestAnimationFrame(() => void currentWindow.show()));
}
