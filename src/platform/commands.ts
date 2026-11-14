// Typed calls of the native side's commands (src-tauri/src/commands). Errors come back as the
// native error's message (a string).
import { invoke } from '@tauri-apps/api/core';
import type { ChatLine, Tick } from '../core/assistant/types.ts';
import type { PlanRow } from '../core/plan/format.ts';
import type { AccountForm, SettingsForm } from '../core/settings/types.ts';

export interface AppInfo {
  version: string;
  language: string;
  theme: string;
  hint_words: { end: string; cancel: string; clear: string };
  data: string;
  first_run: boolean;
  open_settings: boolean;
}
export interface AgentRow { name: string; working: boolean; minutes: number; finished_at: number }
export interface Account { email: string; name: string; org: string; plan: string; logged_in: boolean }
export interface UsageWindow { key: string; percent: number; resets_at: string | null }
export interface Project { id: string; path: string; size: number; sessions: number; last: number }
export interface Session { id: string; title: string; messages: number; size: number; mtime: number; active: boolean; current: boolean; agent: string }
export interface Tokens { input: number; output: number; cache_read: number; cache_5m: number; cache_1h: number }
export interface StatTotals { count: number; errors: number; tokens: Tokens; cost: number; avg_ms: number; p90_ms: number }
export interface StatRow { key: string; count: number; errors: number; tokens: number; cost: number; avg_ms: number }
export interface StatDay { date: string; count: number; tokens: number; cost: number }
export interface StatTop { kind: string; name: string; session: string; last: number; count: number; tokens: number; cost: number }
export interface TokenStats { today: StatTotals; week: StatTotals; kinds: StatRow[]; models: StatRow[]; days: StatDay[]; top: StatTop[]; retries: number }
export interface Devices { input: string[]; output: string[] }
export interface Speaker { name: string; host: string; model: string }
export interface TrayTexts { show: string; talk: string; pause: string; resume: string; settings: string; quit: string }

export const commands = {
  appInfo: () => invoke<AppInfo>('app_info'),
  tick: (feedAfter: number, chatOffset: number) => invoke<Tick>('tick', { feedAfter, chatOffset }),
  chatHistory: () => invoke<[ChatLine[], number]>('chat_history'),
  request: (action: string) => invoke<void>('request', { action }),
  setPaused: (paused: boolean) => invoke<void>('set_paused', { paused }),
  setTerminalOpen: (open: boolean) => invoke<void>('set_terminal_open', { open }),
  toggleConsole: () => invoke<void>('toggle_console'),
  repeatLast: () => invoke<void>('repeat_last'),
  restartListener: () => invoke<void>('restart_listener'),
  hideWindow: () => invoke<void>('hide_window'),
  trayTexts: (texts: TrayTexts) => invoke<void>('tray_texts', { texts }),
  quit: (installNow: boolean) => invoke<void>('quit', { installNow }),
  agentsList: () => invoke<AgentRow[]>('agents_list'),
  agentLog: (name: string) => invoke<[string, number]>('agent_log', { name }),
  agentStop: (name: string) => invoke<boolean>('agent_stop', { name }),
  agentRemove: (name: string) => invoke<void>('agent_remove', { name }),
  planItems: () => invoke<PlanRow[]>('plan_items'),
  planDetail: (id: number) => invoke<[string, string | null] | null>('plan_detail', { id }),
  planRemove: (id: number) => invoke<number>('plan_remove', { id }),
  planRun: (id: number) => invoke<string>('plan_run', { id }),
  claudeAccount: () => invoke<Account>('claude_account'),
  claudeUsage: (cached: boolean) => invoke<UsageWindow[]>('claude_usage', { cached }),
  claudeStats: () => invoke<TokenStats>('claude_stats'),
  claudeProjects: () => invoke<{ projects: Project[]; total: number }>('claude_projects'),
  claudeSessions: (project: string) => invoke<Session[]>('claude_sessions', { project }),
  claudeDelete: (projects: string[], sessions: [string, string][]) => invoke<{ freed: number; errors: string[] }>('claude_delete', { projects, sessions }),
  claudeCleanPreview: (days: number) => invoke<[number, number, number]>('claude_clean_preview', { days }),
  claudeClean: (days: number) => invoke<number>('claude_clean', { days }),
  claudeOpenDir: (project: string | null) => invoke<void>('claude_open_dir', { project }),
  settingsLoad: () => invoke<SettingsForm>('settings_load'),
  settingsSave: (form: SettingsForm) => invoke<void>('settings_save', { form }),
  audioDevices: () => invoke<Devices>('audio_devices'),
  castDiscover: () => invoke<Speaker[]>('cast_discover'),
  mailTest: (row: AccountForm) => invoke<[boolean, string][]>('mail_test', { row }),
  haTest: (url: string, token: string) => invoke<[boolean, string]>('ha_test', { url, token }),
  haPhones: (url: string, token: string) => invoke<string[]>('ha_phones', { url, token }),
  phoneConnectors: (refresh: boolean) => invoke<Record<string, string>>('phone_connectors', { refresh }),
  modelStatus: (language: string) => invoke<string | null>('model_status', { language }),
  modelDownload: (name: string) => invoke<void>('model_download', { name }),
  editConfig: () => invoke<void>('edit_config'),
  backupNow: () => invoke<[string, number]>('backup_now'),
  openBackups: () => invoke<void>('open_backups'),
  openUrl: (url: string) => invoke<void>('open_url', { url }),
  updateCheck: () => invoke<void>('update_check'),
  updateInstall: () => invoke<void>('update_install'),
};

export function errorMessage(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
}
