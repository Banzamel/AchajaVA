// Writing to Achaja without speaking: Enter sends, Shift+Enter adds a line. While Claude works,
// it queues the message and sends it afterwards.
import { useState, type KeyboardEvent } from 'react';
import { useT } from '../../../shared/i18n/useT.ts';

export function ChatInput({ onSend }: { onSend: (text: string) => void }) {
  const t = useT();
  const [text, setText] = useState('');
  const send = () => {
    const value = text.trim();
    if (!value) return;
    setText('');
    onSend(value);
  };
  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };
  return (
    <div className="chat-input card">
      <textarea rows={2} value={text} placeholder={t('chat.placeholder')} onChange={(e) => setText(e.target.value)} onKeyDown={onKey} />
      <button type="button" className={`send-btn ${text.trim() ? 'active' : ''}`} onClick={send} aria-label={t('btn.send')}>
        <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M5 4.5 19 12 5 19.5l1.7-6.7L13 12l-6.3-.8z" fill="currentColor" /></svg>
      </button>
    </div>
  );
}
