// What the native side reports several times a second (the `tick` command), and what the window
// derives from it.

export interface Screen {
  lines: string[];
  rec: boolean;
  input: string;
  activity: string;
  busy: boolean;
  dialog: boolean;
}

export interface FeedItem {
  seq: number;
  kind: 'log' | 'note';
  text: string;
}

export interface ChatLine {
  role: string;
  text: string;
  at: number;
}

export interface UpdateView {
  phase: '' | 'checking' | 'available' | 'downloading' | 'ready' | 'latest' | 'error';
  version: string;
  pct: number;
  error: string;
  manual: boolean;
}

export interface Tick {
  state: string;
  paused: boolean;
  heard: string;
  problem: string | null;
  screen: Screen | null;
  work: string;
  work_since: number;
  work_kind: string;
  speaking: boolean;
  feed: FeedItem[];
  chat: ChatLine[];
  chat_offset: number;
  update: UpdateView;
  tray: boolean;
}

export type StateKey = 'idle' | 'wake' | 'rec' | 'busy' | 'wait' | 'dialog' | 'speak' | 'paused' | 'start' | 'closed' | 'error' | 'off';

// The palette colour of each state (avatar, band, status line).
export const STATE_COLORS: Record<StateKey, string> = {
  idle: 'green', wake: 'red', rec: 'red', busy: 'violet', wait: 'orange', dialog: 'orange',
  speak: 'cyan', paused: 'grey', start: 'grey', closed: 'grey', error: 'red', off: 'grey',
};

export interface Message {
  kind: 'msg' | 'note';
  role?: 'user' | 'ai';
  time?: string;
  body: string;
  voice?: string;
}
