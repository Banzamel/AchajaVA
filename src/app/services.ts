// The app's service instances, created once and found by components through useService.
import { AssistantService } from '../features/assistant/index.ts';
import { ClaudeService } from '../features/claudeAccount/index.ts';
import { SettingsService } from '../features/settings/index.ts';
import { TodayService } from '../features/today/index.ts';
import { ServiceRegistry } from '../shared/hooks/services.tsx';
import { I18nService } from '../shared/i18n/I18nService.ts';
import { ThemeService } from '../shared/lib/ThemeService.ts';
import { ModalService } from '../shared/ui/modal/ModalService.ts';
import { applyConfig } from './startApp.ts';

export function createServices(): ServiceRegistry {
  const r = new ServiceRegistry();
  const i18n = r.add(I18nService, new I18nService());
  r.add(ThemeService, new ThemeService());
  r.add(ModalService, new ModalService());
  r.add(AssistantService, new AssistantService(i18n));
  r.add(ClaudeService, new ClaudeService());
  r.add(TodayService, new TodayService());
  r.add(SettingsService, new SettingsService(() => applyConfig(r)));
  return r;
}
