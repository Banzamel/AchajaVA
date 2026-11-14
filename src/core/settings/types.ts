// The settings form as the native side reads and writes it (passwords and the HA token are never
// sent to the window: has_password / has_token say one is saved; an empty field keeps it).

export interface AccountForm {
  key: string;
  name: string;
  user: string;
  host: string;
  port: number;
  security: string;
  display_name: string;
  signature: string;
  password: string;
  has_password: boolean;
}

export interface SettingsForm {
  language: string;
  model: string;
  effort: string;
  theme: string;
  app: { update_check: boolean; update_install_on_quit: boolean; close_to_tray: boolean; show_console_on_prompt: boolean };
  voice: { microphone: string; local_device: string; rate: string; cast_enabled: boolean; cast_name: string; cast_host: string; default_cast: boolean };
  project_roots: string[];
  ha: {
    enabled: boolean;
    url: string;
    token: string;
    has_token: boolean;
    phone: {
      enabled: boolean;
      notify_service: string;
      push: boolean;
      voice: boolean;
      silent_override: boolean;
      voice_hours: string;
      gmail: boolean;
      imap: boolean;
      calendar: boolean;
      calendar_minutes: number;
    };
  };
  mail: {
    enabled: boolean;
    important_hint: string;
    quiet_hours: string;
    delete_after_days: number;
    important_senders: string;
    ignored_senders: string;
    digest_at: string;
    announce: string[];
    accounts: AccountForm[];
  };
  autostart: boolean;
  editing: boolean;
}

export type PageKey = 'general' | 'voice' | 'agents' | 'mail' | 'rules' | 'ha' | 'summary';

export const PAGE_ICONS: Record<PageKey, string> = { general: '⚙', voice: '🎙', agents: '📁', mail: '✉', rules: '☷', ha: '⌂', summary: '✓' };

export const LANGUAGES: Record<string, string> = { pl: 'polski', en: 'English' };
export const MODELS = ['sonnet', 'opus', 'haiku'];
export const EFFORTS = ['low', 'medium', 'high'];
export const THEMES = ['auto', 'light', 'dark'];
export const ANNOUNCE = ['important', 'threat', 'ad', 'spam'];
export const APP_FLAGS = ['update_check', 'update_install_on_quit', 'close_to_tray', 'show_console_on_prompt'] as const;
