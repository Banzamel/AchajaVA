// Achaja's avatar, an AI core: a chip with a neon frame and pins, traces running from it like on a
// processor board (bent at 45°, ending in vias), neon eyes and mouth on the chip. The state shows
// from afar: idle breathes and blinks, wake/rec listen (waves), busy thinks (data packets, a scan
// line, eyes up), speak talks (an equalizer mouth), wait/dialog glow, paused sleeps, start/closed/
// error are faded. A port of the Python window's avatar.py.
import { blend, type Palette } from '../../../core/theme/palettes.ts';
import type { StateKey } from '../../../core/assistant/types.ts';

const TRACES = 8;

export class AvatarPainter {
  private blinkAt = 2;

  draw(ctx: CanvasRenderingContext2D, s: number, key: StateKey, color: string, p: Palette, now: number) {
    const c = s / 2;
    const bg = p.bg;
    const faded = ['start', 'closed', 'error', 'paused', 'off'].includes(key);
    const tone = (col: string) => (faded ? blend(col, p.grey, 0.6) : col);
    const neon = tone(p.accent);
    const state = tone(color);
    const lw = Math.max(1, s / 70);
    ctx.clearRect(0, 0, s, s);
    const circle = (x: number, y: number, r: number, fill?: string, stroke?: string, w = 1) => {
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      if (fill) { ctx.fillStyle = fill; ctx.fill(); }
      if (stroke) { ctx.strokeStyle = stroke; ctx.lineWidth = w; ctx.stroke(); }
    };
    const breath = 0.5 + 0.5 * Math.sin(now * 1.6);
    const half = s * 0.2;
    if (['rec', 'wake', 'speak', 'wait', 'dialog', 'start'].includes(key)) {
      const period = key === 'rec' || key === 'wake' ? 0.9 : 1.4;
      for (const shift of [0, 0.5]) {
        const f = (now / period + shift) % 1;
        circle(c, c, half * 1.5 + f * (s * 0.5 - half * 1.5), undefined, blend(state, bg, 0.3 + 0.7 * f), lw * 1.3);
      }
    }
    circle(c, c, s * 0.34, blend(state, bg, 0.93 - (key === 'idle' ? 0.04 * breath : 0)));
    const hues = [['trace', 'accent'], ['trace2', 'cyan'], ['trace3', 'violet']].map(([t, n]) => [tone(blend(p[t], p[n], 0.45)), tone(p[n])]);
    const paths: [number, number][][] = [];
    ctx.lineJoin = 'round';
    for (let i = 0; i < TRACES; i++) {
      const a = (Math.PI / 4) * i + Math.PI / 8;
      const [ux, uy] = [Math.cos(a), Math.sin(a)];
      const p0: [number, number] = [c + ux * half * 1.15, c + uy * half * 1.15];
      const p1: [number, number] = [c + ux * s * 0.33, c + uy * s * 0.33];
      const turn = a + (i % 2 ? Math.PI / 4 : -Math.PI / 4);
      const p2: [number, number] = [p1[0] + Math.cos(turn) * s * 0.09, p1[1] + Math.sin(turn) * s * 0.09];
      paths.push([p0, p1, p2]);
      const [line, via] = hues[i % 3];
      ctx.beginPath();
      ctx.moveTo(...p0); ctx.lineTo(...p1); ctx.lineTo(...p2);
      ctx.strokeStyle = line; ctx.lineWidth = lw; ctx.stroke();
      circle(p2[0], p2[1], s * 0.022, bg, blend(via, bg, 0.2), lw);
    }
    if (['busy', 'speak', 'rec', 'wake'].includes(key)) {
      const speed = key === 'busy' ? 1.4 : 0.9;
      paths.forEach(([p0, p1, p2], i) => {
        const f = (now * speed + i * 0.37) % 1;
        const [a, b, t] = f < 0.7 ? [p0, p1, f / 0.7] : [p1, p2, (f - 0.7) / 0.3];
        const x = a[0] + (b[0] - a[0]) * t;
        const y = a[1] + (b[1] - a[1]) * t;
        circle(x, y, s * 0.03, blend(state, bg, 0.6));
        circle(x, y, s * 0.015, state);
      });
    }
    this.chip(ctx, s, c, half, lw, neon, state, p, key, now);
  }

  private chip(ctx: CanvasRenderingContext2D, s: number, c: number, half: number, lw: number, neon: string, state: string, p: Palette, key: StateKey, now: number) {
    const pin = s * 0.045;
    ctx.strokeStyle = neon; ctx.lineWidth = lw * 2;
    for (let k = 0; k < 3; k++) {
      const o = (k - 1) * half * 0.6;
      for (const [sx, sy] of [[0, -1], [0, 1], [-1, 0], [1, 0]]) {
        ctx.beginPath();
        if (sx) { ctx.moveTo(c + sx * half, c + o); ctx.lineTo(c + sx * (half + pin), c + o); }
        else { ctx.moveTo(c + o, c + sy * half); ctx.lineTo(c + o, c + sy * (half + pin)); }
        ctx.stroke();
      }
    }
    ctx.beginPath();
    ctx.roundRect(c - half, c - half, half * 2, half * 2, s * 0.04);
    ctx.fillStyle = p.surface2; ctx.fill();
    ctx.strokeStyle = neon; ctx.lineWidth = lw * 1.5; ctx.stroke();
    if (key === 'busy') {
      const y = c - half * 0.85 + ((now * 1.2) % 1) * half * 1.7;
      ctx.beginPath(); ctx.moveTo(c - half * 0.8, y); ctx.lineTo(c + half * 0.8, y);
      ctx.strokeStyle = blend(state, p.surface2, 0.45); ctx.lineWidth = lw; ctx.stroke();
    }
    this.face(ctx, c, half, lw, key === 'idle' ? neon : state, state, p, key, now);
  }

  private face(ctx: CanvasRenderingContext2D, c: number, half: number, lw: number, eye: string, state: string, p: Palette, key: StateKey, now: number) {
    const [dx, ey] = [half * 0.42, c - half * 0.18];
    if (now > this.blinkAt + 0.14) this.blinkAt = now + 2.5 + Math.random() * 3.5;
    const blinking = this.blinkAt <= now && now <= this.blinkAt + 0.14;
    for (const sx of [-1, 1]) {
      const x = c + sx * dx;
      if (key === 'paused' || blinking) {
        ctx.beginPath(); ctx.moveTo(x - half * 0.16, ey); ctx.lineTo(x + half * 0.16, ey);
        ctx.strokeStyle = eye; ctx.lineWidth = lw * 2; ctx.stroke();
        continue;
      }
      const gy = key === 'busy' ? -half * 0.08 : 0;
      const [ew, eh] = [half * 0.15, half * 0.22 * (key === 'rec' || key === 'wake' ? 1.2 : 1)];
      ctx.beginPath(); ctx.arc(x, ey + gy, eh * 1.5, 0, Math.PI * 2); ctx.fillStyle = blend(eye, p.surface2, 0.75); ctx.fill();
      ctx.beginPath(); ctx.roundRect(x - ew, ey + gy - eh, ew * 2, eh * 2, ew); ctx.fillStyle = eye; ctx.fill();
    }
    const my = c + half * 0.45;
    ctx.fillStyle = state; ctx.strokeStyle = eye; ctx.lineWidth = lw * 2;
    if (key === 'speak') {
      const bw = half * 0.12;
      for (let i = 0; i < 5; i++) {
        const h = half * (0.06 + 0.22 * Math.abs(Math.sin(now * (7 + i * 1.7) + i)));
        ctx.fillRect(c + (i - 2) * bw * 1.6 - bw / 2, my - h, bw, h * 2);
      }
    } else if (['rec', 'wake', 'wait', 'dialog'].includes(key)) {
      ctx.beginPath(); ctx.arc(c, my, half * 0.09, 0, Math.PI * 2); ctx.fill();
    } else if (['closed', 'error', 'busy'].includes(key)) {
      ctx.beginPath(); ctx.moveTo(c - half * 0.22, my); ctx.lineTo(c + half * 0.22, my); ctx.stroke();
    } else {
      ctx.beginPath(); ctx.ellipse(c, my - half * 0.075, half * 0.3, half * 0.175, 0, Math.PI * 0.14, Math.PI * 0.86); ctx.stroke();
    }
  }
}
