// Calls draw(time) on every animation frame (about 60 per second; the browser pauses it while the
// window is hidden).
import { useEffect, useRef } from 'react';

export function useAnimationFrame(draw: (now: number) => void) {
  const saved = useRef(draw);
  saved.current = draw;
  useEffect(() => {
    let id = 0;
    const loop = (t: number) => {
      saved.current(t / 1000);
      id = window.requestAnimationFrame(loop);
    };
    id = window.requestAnimationFrame(loop);
    return () => window.cancelAnimationFrame(id);
  }, []);
}
