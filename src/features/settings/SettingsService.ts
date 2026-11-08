// The Settings view's state: the form (from config.json, without secrets), the current page,
// the audio devices, and saving. A language whose speech model is missing is offered for
// download first (the listener could not start without it).
import { pages } from '../../core/settings/form.ts';
import type { PageKey, SettingsForm } from '../../core/settings/types.ts';
import { commands, errorMessage, onEvent, type Devices } from '../../platform/index.ts';
import { Store } from '../../shared/lib/Store.ts';

export interface SettingsState {
  form: SettingsForm | null;
  index: number;
  devices: Devices;
  status: string;
  missingModel: string | null;
  downloadPct: number | null;
}

export class SettingsService {
  readonly store = new Store<SettingsState>({ form: null, index: 0, devices: { input: [], output: [] }, status: '', missingModel: null, downloadPct: null });

  private listening = false;

  // onSaved: the window takes the saved language and theme at once.
  constructor(private readonly onSaved: () => Promise<void> = async () => {}) {}

  // Fresh from config.json each time the view opens (an editor may have changed it meanwhile).
  async load() {
    const form = await commands.settingsLoad();
    this.store.set({ form, index: 0, status: '' });
    void this.checkModel(form.language);
    void commands.audioDevices().then((devices) => this.store.set({ devices }));
    if (this.listening) return;
    this.listening = true;
    await onEvent<number>('achaja://model-progress', (f) => this.store.set({ downloadPct: Math.round(f * 100) }));
  }

  pages(): PageKey[] {
    return pages(this.store.get().form?.editing ?? true);
  }

  page(): PageKey {
    return this.pages()[this.store.get().index] ?? 'general';
  }

  go(index: number) {
    this.store.set({ index: Math.max(0, Math.min(index, this.pages().length - 1)) });
  }

  update(change: (f: SettingsForm) => SettingsForm) {
    const form = this.store.get().form;
    if (form) this.store.set({ form: change(form) });
  }

  setStatus(status: string) {
    this.store.set({ status });
  }

  async checkModel(language: string) {
    this.store.set({ missingModel: await commands.modelStatus(language).catch(() => null) });
  }

  async downloadModel(): Promise<boolean> {
    const name = this.store.get().missingModel;
    if (!name) return true;
    this.store.set({ downloadPct: 0 });
    try {
      await commands.modelDownload(name);
      this.store.set({ missingModel: null, downloadPct: null });
      return true;
    } catch (e) {
      this.store.set({ downloadPct: null, status: errorMessage(e) });
      return false;
    }
  }

  // Saves the form; a missing model of a new language is reported in missingModel.
  async save(): Promise<boolean> {
    const form = this.store.get().form;
    if (!form) return false;
    try {
      await commands.settingsSave(form);
      await this.onSaved().catch(() => undefined);
      return true;
    } catch (e) {
      const msg = errorMessage(e);
      if (msg.startsWith('model:')) this.store.set({ missingModel: msg.slice(6) });
      else this.store.set({ status: msg });
      return false;
    }
  }
}
