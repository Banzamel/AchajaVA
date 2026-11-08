// Minimize, maximize/restore and close, in the Windows 11 layout (46 px wide, full bar height).
import type { TitleBarLabels, WindowFrame } from './types';
import { usePixelRatio } from './usePixelRatio';
import { WindowGlyph } from './WindowGlyph';

interface Props {
  frame: WindowFrame;
  labels: TitleBarLabels;
  maximized: boolean;
}

export function WindowControls({ frame, labels, maximized }: Props) {
  const pixelRatio = usePixelRatio();
  const resize = maximized ? 'restore' : 'maximize';
  return (
    <div className="titlebar-controls">
      <button type="button" className="titlebar-button" title={labels.minimize} aria-label={labels.minimize} onClick={() => void frame.minimize()}>
        <WindowGlyph kind="minimize" pixelRatio={pixelRatio} />
      </button>
      <button type="button" className="titlebar-button" title={labels[resize]} aria-label={labels[resize]} onClick={() => void frame.toggleMaximize()}>
        <WindowGlyph kind={resize} pixelRatio={pixelRatio} />
      </button>
      <button type="button" className="titlebar-button close" title={labels.close} aria-label={labels.close} onClick={() => void frame.close()}>
        <WindowGlyph kind="close" pixelRatio={pixelRatio} />
      </button>
    </div>
  );
}
