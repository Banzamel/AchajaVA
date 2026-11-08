import { describe, expect, it } from 'vitest';
import { appendMessages, copyText, splitVoice, toMessage } from '../../../src/core/assistant/messages';
import type { Message } from '../../../src/core/assistant/types';

describe('messages', () => {
  it('splits the spoken part off the last marker', () => {
    expect(splitVoice('Długi tekst.\n[GŁOS] Gotowe.')).toEqual({ body: 'Długi tekst.', voice: 'Gotowe.' });
    expect(splitVoice('a\n[VOICE]: x\n[voice] y')).toEqual({ body: 'a\n[VOICE]: x', voice: 'y' });
    expect(splitVoice('bez znacznika')).toEqual({ body: 'bez znacznika', voice: '' });
  });

  it('turns chat lines into messages', () => {
    const user = toMessage({ role: 'user', text: ' hej ', at: 0 });
    expect(user.role).toBe('user');
    expect(user.body).toBe('hej');
    const ai = toMessage({ role: 'assistant', text: 'Tekst\n[GŁOS] Tak.', at: 0 });
    expect([ai.role, ai.body, ai.voice]).toEqual(['ai', 'Tekst', 'Tak.']);
    expect(copyText(ai)).toBe('Tekst\n\nTak.');
  });

  it('keeps the newest 200 once past 300 and skips empty ones', () => {
    const many: Message[] = Array.from({ length: 301 }, (_, i) => ({ kind: 'note', body: String(i) }));
    const out = appendMessages([], many);
    expect(out).toHaveLength(200);
    expect(out[199].body).toBe('300');
    expect(appendMessages([], [{ kind: 'msg', role: 'ai', body: '', voice: '' }])).toHaveLength(0);
  });
});
