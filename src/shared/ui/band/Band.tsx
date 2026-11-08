// The processor-traces band (see bandPainter): animated while `active`, drawn once otherwise.
import { useEffect, useRef } from 'react';
import { useService, useStore } from '../../hooks/services.tsx';
import { useAnimationFrame } from '../../hooks/useAnimationFrame.ts';
import { ThemeService } from '../../lib/ThemeService.ts';
import { drawBand } from './bandPainter.ts';

interface Props {
  height: number;
  color?: string;
  active?: boolean;
  background?: string;
}

export function Band({ height, color, active = false, background = 'bg' }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);
  const { palette } = useStore(useService(ThemeService).store);
  const draw = (now: number) => {
    const c = ref.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const ratio = window.devicePixelRatio || 1;
    const w = c.clientWidth;
    if (c.width !== Math.round(w * ratio)) {
      c.width = Math.round(w * ratio);
      c.height = Math.round(height * ratio);
    }
    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    drawBand(ctx, w, height, palette, background, color ?? palette.accent, active, now);
  };
  useAnimationFrame((now) => {
    if (active) draw(now);
  });
  useEffect(() => draw(0));
  return <canvas ref={ref} className="band" style={{ height }} />;
}
