// Pure helpers of the Settings view: its pages, the phone notification mode, a new account,
// the port that follows the encryption, and the summary of the first-run wizard.
import type { AccountForm, PageKey, SettingsForm } from './types.ts';

// Editing existing settings shows sections; the first run walks through them with a summary.
export function pages(editing: boolean): PageKey[] {
  const list: PageKey[] = ['general', 'voice', 'agents', 'mail', 'rules', 'ha'];
  return editing ? list : [...list, 'summary'];
}

export type PhoneMode = 'both' | 'push' | 'voice';

export function phoneMode(p: { push: boolean; voice: boolean }): PhoneMode {
  if (!p.voice) return 'push';
  if (!p.push) return 'voice';
  return 'both';
}

export function setPhoneMode(mode: PhoneMode): { push: boolean; voice: boolean } {
  return { push: mode !== 'voice', voice: mode !== 'push' };
}

export function newAccount(): AccountForm {
  return { key: '', name: '', user: '', host: '', port: 993, security: 'ssl', display_name: '', signature: '', password: '', has_password: false };
}

export function portFor(security: string): number {
  return security === 'ssl' ? 993 : 143;
}

// An account can be saved once it has a server and an address; its name defaults to the address.
export function finishAccount(a: AccountForm): AccountForm | null {
  if (!a.host.trim() || !a.user.trim()) return null;
  return { ...a, name: a.name.trim() || a.user.trim(), has_password: a.has_password || a.password !== '' };
}

export interface SummaryWords {
  language: string;
  model: string;
  mic: string;
  out: string;
  cast: string;
  roots: string;
  accounts: string;
  mailOff: string;
  ha: string;
  off: string;
  systemDefault: string;
}

// The summary lines (texts with {} filled in by the caller's words).
export function summaryLines(f: SettingsForm, w: SummaryWords, languageName: string): string[] {
  const device = (d: string) => d || w.systemDefault;
  const accounts = f.mail.accounts.map((a) => a.name || a.user);
  return [
    `${w.language}: ${languageName} · ${w.model}: ${f.model} / ${f.effort}`,
    w.mic.replace('{}', device(f.voice.microphone)),
    w.out.replace('{}', device(f.voice.local_device)),
    w.cast.replace('{}', f.voice.cast_enabled ? `${f.voice.cast_name} (${f.voice.cast_host})` : w.off),
    w.roots.replace('{}', f.project_roots.join(', ') || '—'),
    f.mail.enabled && accounts.length ? w.accounts.replace('{}', accounts.join(', ')) : w.mailOff,
    w.ha.replace('{}', f.ha.enabled ? f.ha.url : w.off),
  ];
}

// The part of a device name config.json keeps; the list shows it as the full matching name.
export function matchDevice(wanted: string, names: string[]): string {
  if (!wanted) return '';
  return names.find((n) => n.toLowerCase().includes(wanted.toLowerCase())) ?? wanted;
}

// The results of a test, one line each: ✓ or ✗ and the description.
export function resultsText(results: [boolean, string][]): string {
  return results.map(([ok, text]) => `${ok ? '✓' : '✗'} ${text}`).join('\n');
}
