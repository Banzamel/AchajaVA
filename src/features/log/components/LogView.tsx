// What the listener hears and does: times faint, errors red.
import { useEffect, useRef } from 'react';
import { logLine } from '../../../core/assistant/terminal.ts';

export function LogView({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);
  useEffect(() => {
    const el = ref.current;
    if (el && atEnd.current) el.scrollTop = el.scrollHeight;
  }, [lines]);
  const onScroll = () => {
    const el = ref.current;
    if (el) atEnd.current = el.scrollTop + el.clientHeight >= el.scrollHeight - 8;
  };
  return (
    <div className="tab-page">
      <div className="card grow-card log" ref={ref} onScroll={onScroll}>
        {lines.map((l, i) => {
          const x = logLine(l);
          return (
            <div key={i} className={x.error ? 'log-line err' : 'log-line'}>
              <span className="log-time">{x.time} </span>
              {x.text}
            </div>
          );
        })}
      </div>
    </div>
  );
}
