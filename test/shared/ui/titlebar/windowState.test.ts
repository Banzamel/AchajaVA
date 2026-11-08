import { describe, expect, it } from 'vitest';
import type { WindowFrame, WindowSize } from '../../../../src/shared/ui/titlebar/types';
import { watchWindow, windowState } from '../../../../src/shared/ui/titlebar/windowState';

describe('windowState', () => {
  it('has no border when maximized', () => {
    expect(windowState({ maximized: true, outerWidth: 1936, innerWidth: 1920 })).toEqual({ maximized: true, border: 'none' });
  });

  it('leaves the border to Windows when the native frame is wider than the page', () => {
    expect(windowState({ maximized: false, outerWidth: 1376, innerWidth: 1360 })).toEqual({ maximized: false, border: 'native' });
  });

  it('draws the border when the window has no frame', () => {
    expect(windowState({ maximized: false, outerWidth: 1360, innerWidth: 1360 })).toEqual({ maximized: false, border: 'drawn' });
  });
});

const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

// A window whose size the test sets; resize() fires the resize handlers.
function fakeFrame(size: WindowSize) {
  const handlers = new Set<() => void>();
  let measures = 0;
  const frame: WindowFrame = {
    minimize: async () => undefined,
    toggleMaximize: async () => undefined,
    close: async () => undefined,
    measure: async () => {
      measures++;
      return { ...size };
    },
    onResized: async (handler) => {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  };
  return {
    frame,
    handlers,
    measures: () => measures,
    resize: (next: Partial<WindowSize>) => {
      Object.assign(size, next);
      handlers.forEach((h) => h());
    },
  };
}

describe('watchWindow', () => {
  it('reports the state at start and after a resize', async () => {
    const fake = fakeFrame({ maximized: false, outerWidth: 1360, innerWidth: 1360 });
    const seen: string[] = [];
    const stop = watchWindow(fake.frame, (s) => seen.push(`${s.maximized} ${s.border}`));
    await flush();
    fake.resize({ maximized: true });
    await flush();
    expect(seen).toEqual(['false drawn', 'true none']);
    stop();
  });

  it('measures once more after a burst of resizes, not once per event', async () => {
    const fake = fakeFrame({ maximized: false, outerWidth: 1360, innerWidth: 1360 });
    const stop = watchWindow(fake.frame, () => undefined);
    await flush();
    const before = fake.measures();
    for (let i = 0; i < 5; i++) fake.resize({ outerWidth: 1360 + i, innerWidth: 1360 + i });
    await flush();
    expect(fake.measures() - before).toBe(2);
    stop();
  });

  it('stops listening and reporting when stopped', async () => {
    const fake = fakeFrame({ maximized: false, outerWidth: 1360, innerWidth: 1360 });
    const seen: unknown[] = [];
    const stop = watchWindow(fake.frame, (s) => seen.push(s));
    await flush();
    stop();
    expect(fake.handlers.size).toBe(0);
    fake.resize({ maximized: true });
    await flush();
    expect(seen).toHaveLength(1);
  });

  it('keeps quiet outside the app window', async () => {
    const frame = fakeFrame({ maximized: false, outerWidth: 0, innerWidth: 0 }).frame;
    const broken: WindowFrame = { ...frame, measure: () => { throw new Error('no window'); }, onResized: () => Promise.reject(new Error('no window')) };
    const seen: unknown[] = [];
    const stop = watchWindow(broken, (s) => seen.push(s));
    await flush();
    expect(seen).toEqual([]);
    stop();
  });
});
