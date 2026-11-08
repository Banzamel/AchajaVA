// The band of processor traces under the header (and in the settings sidebar): a chip in the
// middle, the main trace and branches bent at 45° to both sides, ending in vias, fading towards
// the ends. While Achaja listens, thinks or speaks the core pulses in the state's colour and data
// packets run along the traces. A port of the Python window's ornament.py.
import { blend, type Palette } from '../../../core/theme/palettes.ts';

const BRANCHES: [number, number, number][] = [[0.06, -1, 0.42], [0.22, 1, 0.66], [0.5, -1, 0.9], [0.72, 1, 0.97]];

type Pt = [number, number];

function paths(cx: number, chip: number, span: number, mid: number, lane: number): [Pt[], boolean, number][] {
  const out: [Pt[], boolean, number][] = [];
  for (const side of [-1, 1]) {
    const x = (f: number) => cx + side * (chip + f * span);
    out.push([[[x(0), mid], [x(1), mid]], false, 0]);
    BRANCHES.forEach(([start, dir, end], i) => {
      const y = mid + dir * lane;
      const x0 = x(start);
      out.push([[[x0, mid], [x0 + side * lane, y], [x(end), y]], true, (i + (side > 0 ? 1 : 0)) % 3]);
    });
  }
  return out;
}

export function drawBand(ctx: CanvasRenderingContext2D, w: number, h: number, p: Palette, bgKey: string, color: string, active: boolean, now: number) {
  const bg = p[bgKey] ?? p.bg;
  const traces = [p.trace, p.trace2, p.trace3];
  const neons = [p.accent, p.cyan, p.violet];
  ctx.clearRect(0, 0, w, h);
  const mid = h / 2;
  const cx = w / 2;
  const chip = h * 0.55;
  const lane = h * 0.3;
  const span = cx - chip - h * 0.2;
  const lw = Math.max(1, h * 0.07);
  const faded = (col: string, f: number) => blend(bg, col, Math.max(0, (1 - f) ** 1.3));
  for (const [pts, via, hue] of paths(cx, chip, span, mid, lane)) {
    const line = blend(traces[hue], neons[hue], active ? 0.55 : 0.4);
    const glow = blend(bg, traces[hue], 0.45);
    for (let s = 0; s + 1 < pts.length; s++) {
      const [[xa, ya], [xb, yb]] = [pts[s], pts[s + 1]];
      const n = Math.max(1, Math.floor(Math.abs(xb - xa) / 6));
      for (let i = 0; i < n; i++) {
        const [t0, t1] = [i / n, (i + 1) / n];
        const f = Math.abs((xa + (xb - xa) * (t0 + t1) / 2) - cx) / (cx || 1);
        for (const [col, width] of [[faded(glow, f), lw * 3], [faded(line, f), lw]] as [string, number][]) {
          ctx.beginPath();
          ctx.moveTo(xa + (xb - xa) * t0, ya + (yb - ya) * t0);
          ctx.lineTo(xa + (xb - xa) * t1, ya + (yb - ya) * t1);
          ctx.strokeStyle = col; ctx.lineWidth = width; ctx.stroke();
        }
      }
    }
    if (via) {
      const [vx, vy] = pts[pts.length - 1];
      ctx.beginPath(); ctx.arc(vx, vy, h * 0.1, 0, Math.PI * 2);
      ctx.fillStyle = bg; ctx.fill();
      ctx.strokeStyle = faded(neons[hue], (Math.abs(vx - cx) / (cx || 1)) * 0.85); ctx.lineWidth = lw; ctx.stroke();
    }
    if (active) packet(ctx, pts, via, color, bg, cx, mid, h, now);
  }
  core(ctx, cx, mid, chip, h, lw, p, color, active, now);
}

function packet(ctx: CanvasRenderingContext2D, pts: Pt[], via: boolean, color: string, bg: string, cx: number, mid: number, h: number, now: number) {
  const seg = pts.slice(1).map((b, i) => Math.hypot(b[0] - pts[i][0], b[1] - pts[i][1]));
  const total = seg.reduce((a, b) => a + b, 0) || 1;
  let d = ((now * (via ? 0.8 : 0.55) + (pts[pts.length - 1][1] - mid) * 0.01 + pts[0][0] * 0.003) % 1) * total;
  for (let i = 0; i < seg.length; i++) {
    if (d <= seg[i]) {
      const t = d / (seg[i] || 1);
      const qx = pts[i][0] + (pts[i + 1][0] - pts[i][0]) * t;
      const qy = pts[i][1] + (pts[i + 1][1] - pts[i][1]) * t;
      const f = Math.abs(qx - cx) / (cx || 1);
      for (const [r, mix] of [[h * 0.22, 0.75], [h * 0.12, 0]]) {
        ctx.beginPath(); ctx.arc(qx, qy, r, 0, Math.PI * 2);
        ctx.fillStyle = blend(color, bg, Math.min(1, mix + f * 0.8)); ctx.fill();
      }
      return;
    }
    d -= seg[i];
  }
}

function core(ctx: CanvasRenderingContext2D, cx: number, mid: number, chip: number, h: number, lw: number, p: Palette, color: string, active: boolean, now: number) {
  const pulse = active ? 0.5 + 0.5 * Math.sin(now * 4) : 0;
  const halfH = h * 0.36;
  if (active) {
    const g = chip * (1.25 + 0.2 * pulse);
    ctx.fillStyle = blend(color, p.bg, 0.82 - 0.1 * pulse);
    ctx.fillRect(cx - g, mid - halfH * 1.5, g * 2, halfH * 3);
  }
  ctx.strokeStyle = p.accent; ctx.lineWidth = lw;
  for (let i = 0; i < 4; i++) {
    const x = cx - chip * 0.66 + i * chip * 0.44;
    for (const sy of [-1, 1]) {
      ctx.beginPath(); ctx.moveTo(x, mid + sy * halfH); ctx.lineTo(x, mid + sy * (halfH + h * 0.13)); ctx.stroke();
    }
  }
  ctx.fillStyle = p.surface2;
  ctx.fillRect(cx - chip, mid - halfH, chip * 2, halfH * 2);
  ctx.strokeRect(cx - chip, mid - halfH, chip * 2, halfH * 2);
  const r = h * (0.13 + 0.04 * pulse);
  ctx.fillStyle = active ? blend(p.accent, color, 0.5 + 0.5 * pulse) : p.accent;
  ctx.fillRect(cx - r, mid - r, r * 2, r * 2);
}
