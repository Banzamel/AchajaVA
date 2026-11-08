// A read-only text area in a card that keeps following the end while the user is at the end
// (logs, the agent's progress, the console).
import { useEffect, useRef } from 'react';

export function TextPanel({ text, mono, className }: { text: string; mono?: boolean; className?: string }) {
  const ref = useRef<HTMLPreElement>(null);
  const atEnd = useRef(true);
  useEffect(() => {
    const el = ref.current;
    if (el && atEnd.current) el.scrollTop = el.scrollHeight;
  }, [text]);
  const onScroll = () => {
    const el = ref.current;
    if (el) atEnd.current = el.scrollTop + el.clientHeight >= el.scrollHeight - 8;
  };
  return <pre ref={ref} className={`text-panel ${mono ? 'mono' : ''} ${className ?? ''}`} onScroll={onScroll}>{text}</pre>;
}
