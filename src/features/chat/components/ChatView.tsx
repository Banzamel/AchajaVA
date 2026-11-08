// The conversation: the prompts and answers filled by the session hooks, notes from Achaja, and
// the box to write in. Keeps following the newest message while the user is at the end.
import { useEffect, useRef } from 'react';
import { useT } from '../../../shared/i18n/useT.ts';
import type { Message } from '../../../core/assistant/types.ts';
import { Bubble } from './Bubble.tsx';
import { ChatInput } from './ChatInput.tsx';

export function ChatView({ messages, onSend }: { messages: Message[]; onSend: (text: string) => void }) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  useEffect(() => {
    const el = box.current;
    if (el && atEnd.current) el.scrollTop = el.scrollHeight;
  }, [messages]);
  const onScroll = () => {
    const el = box.current;
    if (el) atEnd.current = el.scrollTop + el.clientHeight >= el.scrollHeight - 24;
  };
  return (
    <div className="tab-page chat">
      <div className="chat-list" ref={box} onScroll={onScroll}>
        {messages.length === 0 ? (
          <div className="chat-empty">
            <div className="chat-empty-title">{t('chat.empty_title')}</div>
            <div>{t('chat.empty')}</div>
          </div>
        ) : (
          messages.map((m, i) => <Bubble key={i} m={m} />)
        )}
      </div>
      <ChatInput onSend={onSend} />
    </div>
  );
}
