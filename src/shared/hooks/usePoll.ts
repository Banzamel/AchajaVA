// Calls fn now and every `ms` while the component is shown (and `active` is true).
import { useEffect, useRef } from 'react';

export function usePoll(fn: () => void, ms: number, active = true) {
  const saved = useRef(fn);
  saved.current = fn;
  useEffect(() => {
    if (!active) return undefined;
    saved.current();
    const id = window.setInterval(() => saved.current(), ms);
    return () => window.clearInterval(id);
  }, [ms, active]);
}
