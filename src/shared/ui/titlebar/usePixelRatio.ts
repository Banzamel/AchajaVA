// Screen pixels per CSS pixel: changes when the window moves to a screen with another scale.
import { useEffect, useState } from 'react';

const current = () => window.devicePixelRatio || 1;

export function usePixelRatio(): number {
  const [ratio, setRatio] = useState(current);
  useEffect(() => {
    // A query for the current ratio stops matching as soon as the ratio changes.
    const query = window.matchMedia(`(resolution: ${ratio}dppx)`);
    const onChange = () => setRatio(current());
    query.addEventListener('change', onChange);
    return () => query.removeEventListener('change', onChange);
  }, [ratio]);
  return ratio;
}
