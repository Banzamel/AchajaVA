import { describe, expect, it } from 'vitest';
import { finishAccount, matchDevice, newAccount, pages, phoneMode, portFor, resultsText, setPhoneMode, summaryLines } from '../../../src/core/settings/form';
import type { SettingsForm } from '../../../src/core/settings/types';

const form = (): SettingsForm => ({
  language: 'pl', model: 'sonnet', effort: 'low', theme: 'auto',
  app: { update_check: true, update_install_on_quit: true, close_to_tray: true, show_console_on_prompt: false },
  voice: { microphone: '', local_device: 'JBL', rate: '+15%', cast_enabled: true, cast_name: 'Kuchnia', cast_host: '10.0.0.5', default_cast: false },
  project_roots: ['C:\\projekty'],
  ha: { enabled: false, url: '', token: '', has_token: false, phone: { enabled: false, notify_service: '', push: true, voice: true, silent_override: true, voice_hours: '', gmail: true, imap: true, calendar: false, calendar_minutes: 15 } },
  mail: { enabled: true, important_hint: '', quiet_hours: '', delete_after_days: 7, important_senders: '', ignored_senders: '', digest_at: '', announce: [], accounts: [{ ...newAccount(), name: 'firma', user: 'a@b.pl', host: 'h' }] },
  autostart: false,
  editing: false,
});

describe('settings form', () => {
  it('lists the pages of the wizard and of editing', () => {
    expect(pages(false).at(-1)).toBe('summary');
    expect(pages(true)).not.toContain('summary');
  });

  it('maps the phone notification mode', () => {
    expect(phoneMode({ push: true, voice: false })).toBe('push');
    expect(phoneMode({ push: false, voice: true })).toBe('voice');
    expect(setPhoneMode('both')).toEqual({ push: true, voice: true });
  });

  it('finishes accounts and picks ports', () => {
    expect(finishAccount(newAccount())).toBeNull();
    expect(finishAccount({ ...newAccount(), host: 'h', user: 'u@x', password: 'p' })).toMatchObject({ name: 'u@x', has_password: true });
    expect(portFor('starttls')).toBe(143);
  });

  it('summarizes and matches devices', () => {
    const lines = summaryLines(form(), { language: 'Język', model: 'Model', mic: 'mikrofon: {}', out: 'wyjście: {}', cast: 'głośnik: {}', roots: 'foldery: {}', accounts: 'konta: {}', mailOff: 'off', ha: 'HA: {}', off: 'wył.', systemDefault: '(domyślne)' }, 'polski');
    expect(lines).toContain('mikrofon: (domyślne)');
    expect(lines).toContain('głośnik: Kuchnia (10.0.0.5)');
    expect(lines).toContain('konta: firma');
    expect(matchDevice('jbl', ['Głośniki (Realtek)', 'Słuchawki (JBL TUNE510BT)'])).toBe('Słuchawki (JBL TUNE510BT)');
    expect(resultsText([[true, 'ok'], [false, 'bad']])).toBe('✓ ok\n✗ bad');
  });
});
