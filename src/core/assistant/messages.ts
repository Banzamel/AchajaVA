// Chat lines from state\chat.jsonl turned into what the conversation shows: the user's prompts
// on the right; Achaja's answers on the left, split into the text and the spoken [GŁOS] part.
import type { ChatLine, Message } from './types.ts';

const MARKER = /^\s*\[(?:G[ŁL]OS|VOICE)\]\s*:?\s*/gim;

export function splitVoice(text: string): { body: string; voice: string } {
  const matches = [...(text || '').matchAll(MARKER)];
  if (!matches.length) return { body: (text || '').trim(), voice: '' };
  const last = matches[matches.length - 1];
  const at = last.index ?? 0;
  return { body: text.slice(0, at).trim(), voice: text.slice(at + last[0].length).trim() };
}

export function clock(at: number): string {
  const d = new Date(at * 1000);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

export function toMessage(line: ChatLine): Message {
  const time = clock(line.at || Date.now() / 1000);
  if (line.role === 'user') return { kind: 'msg', role: 'user', time, body: line.text.trim(), voice: '' };
  const { body, voice } = splitVoice(line.text);
  return { kind: 'msg', role: 'ai', time, body, voice };
}

// The conversation keeps its last 200 messages once it passes 300.
export function appendMessages(list: Message[], more: Message[]): Message[] {
  const next = list.concat(more.filter((m) => m.kind === 'note' || m.body || m.voice));
  return next.length > 300 ? next.slice(-200) : next;
}

// The text copied by "Copy" under an answer.
export function copyText(m: Message): string {
  return [m.body, m.voice].filter(Boolean).join('\n\n');
}
