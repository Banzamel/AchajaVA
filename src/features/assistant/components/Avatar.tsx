// The animated avatar (see avatarPainter), drawn at the screen's pixel density.
import { useRef } from 'react';
import { STATE_COLORS, type StateKey } from '../../../core/assistant/types.ts';
import { useService, useStore } from '../../../shared/hooks/services.tsx';
import { useAnimationFrame } from '../../../shared/hooks/useAnimationFrame.ts';
import { ThemeService } from '../../../shared/lib/ThemeService.ts';
import { AvatarPainter } from '../canvas/avatarPainter.ts';

export function Avatar({ stateKey, size }: { stateKey: StateKey; size: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const painter = useRef(new AvatarPainter());
  const { palette } = useStore(useService(ThemeService).store);
  useAnimationFrame((now) => {
    const c = ref.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const ratio = window.devicePixelRatio || 1;
    const px = Math.round(size * ratio);
    if (c.width !== px) {
      c.width = px;
      c.height = px;
    }
    painter.current.draw(ctx, px, stateKey, palette[STATE_COLORS[stateKey]], palette, now);
  });
  return <canvas ref={ref} className="avatar" style={{ width: size, height: size }} />;
}
