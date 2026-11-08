import { describe, expect, it } from 'vitest';
import { Store } from '../../../src/shared/lib/Store';

describe('Store', () => {
  it('replaces state and notifies subscribers', () => {
    const s = new Store({ a: 1, b: 'x' });
    let calls = 0;
    const stop = s.subscribe(() => calls++);
    const before = s.get();
    s.set({ a: 2 });
    s.set((st) => ({ b: `${st.b}y` }));
    expect(s.get()).toEqual({ a: 2, b: 'xy' });
    expect(s.get()).not.toBe(before);
    stop();
    s.set({ a: 3 });
    expect(calls).toBe(2);
  });
});
