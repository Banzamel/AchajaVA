// The main window's assistant state: polls the native side every 300 ms (the listener, the Claude
// console, the session's work, new log lines, notes and chat messages), turns it into the shown
// state, and opens the Terminal tab (with a note and a spoken hint) when Claude waits for a choice.
import { appendMessages, toMessage } from '../../core/assistant/messages.ts';
import { stateKey } from '../../core/assistant/stateKey.ts';
import type { Message, StateKey, Tick } from '../../core/assistant/types.ts';
import { commands, type AppInfo } from '../../platform/index.ts';
import type { I18nService } from '../../shared/i18n/I18nService.ts';
import { Store } from '../../shared/lib/Store.ts';

export type TabKey = 'chat' | 'term' | 'agents' | 'plan' | 'claude' | 'log';

export interface AssistantState {
  info: AppInfo | null;
  tick: Tick | null;
  key: StateKey;
  messages: Message[];
  logs: string[];
  tab: TabKey;
}

const POLL_MS = 300;

export class AssistantService {
  readonly store = new Store<AssistantState>({ info: null, tick: null, key: 'start', messages: [], logs: [], tab: 'chat' });
  private feedAfter = 0;
  private chatOffset = 0;
  private dialogSeen = false;
  private timer = 0;

  constructor(private readonly i18n: I18nService) {}

  async start() {
    const info = await commands.appInfo();
    const [lines, offset] = await commands.chatHistory();
    this.chatOffset = offset;
    this.store.set({ info, messages: appendMessages([], lines.map(toMessage)) });
    this.timer = window.setInterval(() => void this.poll(), POLL_MS);
  }

  stop() {
    window.clearInterval(this.timer);
  }

  setTab(tab: TabKey) {
    this.store.set({ tab });
    void commands.setTerminalOpen(tab === 'term');
  }

  note(text: string) {
    this.store.set((s) => ({ messages: appendMessages(s.messages, [{ kind: 'note', body: text }]) }));
  }

  private noteText(text: string): string {
    return text.startsWith('@') ? this.i18n.t(`note.${text.slice(1)}`) : text;
  }

  private async poll() {
    let tick: Tick;
    try {
      tick = await commands.tick(this.feedAfter, this.chatOffset);
    } catch {
      return;
    }
    this.chatOffset = tick.chat_offset;
    const last = tick.feed[tick.feed.length - 1];
    if (last) this.feedAfter = last.seq;
    const key = stateKey(tick);
    const notes: Message[] = tick.feed.filter((f) => f.kind === 'note').map((f) => ({ kind: 'note', body: this.noteText(f.text) }));
    const logs = tick.feed.filter((f) => f.kind === 'log').map((f) => f.text);
    this.store.set((s) => ({
      tick,
      key,
      messages: appendMessages(s.messages, [...tick.chat.map(toMessage), ...notes]),
      logs: logs.length ? s.logs.concat(logs).slice(-3000) : s.logs,
    }));
    this.watchDialog(key, tick);
  }

  // A choice window in the console (e.g. a new MCP server at start): without an answer Achaja
  // can neither record nor reply, so the Terminal tab comes up once per dialog.
  private watchDialog(key: StateKey, tick: Tick) {
    const problem = tick.problem;
    if (key !== 'dialog' && !problem) {
      this.dialogSeen = false;
      return;
    }
    if (this.dialogSeen) return;
    this.dialogSeen = true;
    this.setTab('term');
    this.note(this.i18n.t(problem === 'rec_failed' ? 'note.rec_failed' : 'note.dialog'));
    const waiting = tick.work === 'waiting' && tick.work_kind === 'permission_prompt';
    if (key === 'dialog' && !waiting) void commands.request(`say:${this.i18n.t('note.dialog_say')}`);
  }

  request(action: string) {
    void commands.request(action);
  }

  talk() {
    this.request(this.store.get().tick?.state === 'recording' ? 'send' : 'record');
  }

  togglePause() {
    const paused = this.store.get().tick?.paused ?? false;
    void commands.setPaused(!paused);
  }
}
