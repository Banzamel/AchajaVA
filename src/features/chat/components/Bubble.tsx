// One message: who and when, a bubble as wide as its text (the user's dark with a thin neon
// frame on the right, Achaja's like a card on the left, the spoken part marked ♪), and a visible
// "Copy" under Achaja's answers. Text can be selected and copied too.
import { useState } from 'react';
import { copyText } from '../../../core/assistant/messages.ts';
import type { Message } from '../../../core/assistant/types.ts';
import { copyToClipboard } from '../../../platform/index.ts';
import { useT } from '../../../shared/i18n/useT.ts';

export function Bubble({ m }: { m: Message }) {
  const t = useT();
  const [copied, setCopied] = useState(false);
  if (m.kind === 'note') return <div className="chat-note">{m.body}</div>;
  const mine = m.role === 'user';
  const copy = () => {
    void copyToClipboard(copyText(m));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div className={`chat-msg ${mine ? 'mine' : 'theirs'}`}>
      <div className="chat-who">
        {mine ? t('chat.you') : t('chat.achaja')}
        <span className="chat-time">{m.time}</span>
      </div>
      <div className="bubble">
        {m.body ? <div className="bubble-body">{m.body}</div> : null}
        {m.voice ? <div className="bubble-voice">♪  {m.voice}</div> : null}
      </div>
      {mine ? null : (
        <button type="button" className="copy-link" onClick={copy}>{copied ? `✓ ${t('chat.copied')}` : `⧉ ${t('chat.copy')}`}</button>
      )}
    </div>
  );
}
