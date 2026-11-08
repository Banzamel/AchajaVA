import { beforeEach, describe, expect, it, vi } from 'vitest';

const calls = vi.hoisted(() => ({ save: vi.fn(), load: vi.fn() }));

vi.mock('../../../src/platform/index.ts', () => ({
  commands: {
    settingsLoad: calls.load,
    settingsSave: calls.save,
    modelStatus: () => Promise.resolve(null),
    audioDevices: () => Promise.resolve({ input: [], output: [] }),
  },
  errorMessage: (e: unknown) => String(e),
  onEvent: vi.fn(() => Promise.resolve(() => undefined)),
}));

import { SettingsService } from '../../../src/features/settings/SettingsService';
import { onEvent } from '../../../src/platform/index.ts';

describe('SettingsService (the Settings view of the one window)', () => {
  beforeEach(() => {
    calls.save.mockReset();
    calls.load.mockReset();
    calls.load.mockResolvedValue({ language: 'pl', editing: true });
  });

  it('reloads the form and starts on the first page every time the view opens', async () => {
    const svc = new SettingsService();
    await svc.load();
    svc.go(3);
    await svc.load();
    expect(svc.store.get().index).toBe(0);
    expect(calls.load).toHaveBeenCalledTimes(2);
    expect(onEvent).toHaveBeenCalledTimes(1);
  });

  it('lets the window apply the saved language and theme', async () => {
    const onSaved = vi.fn(() => Promise.resolve());
    const svc = new SettingsService(onSaved);
    await svc.load();
    calls.save.mockResolvedValue(undefined);
    expect(await svc.save()).toBe(true);
    expect(onSaved).toHaveBeenCalledOnce();
  });

  it('reports a missing speech model instead of saving', async () => {
    const onSaved = vi.fn(() => Promise.resolve());
    const svc = new SettingsService(onSaved);
    await svc.load();
    calls.save.mockRejectedValue('model:vosk-model-small-pl-0.22');
    expect(await svc.save()).toBe(false);
    expect(svc.store.get().missingModel).toBe('vosk-model-small-pl-0.22');
    expect(onSaved).not.toHaveBeenCalled();
  });
});
