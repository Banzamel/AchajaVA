// One window button glyph, drawn in screen pixels (see glyph.ts) and sized back to CSS pixels.
import { glyphMetrics, glyphPath } from './glyph';
import type { GlyphKind } from './types';

export function WindowGlyph({ kind, pixelRatio }: { kind: GlyphKind; pixelRatio: number }) {
  const m = glyphMetrics(pixelRatio);
  const d = glyphPath(kind, m);
  const close = kind === 'close';
  return (
    <svg
      className="titlebar-glyph"
      width={m.cssSize}
      height={m.cssSize}
      viewBox={`0 0 ${m.size} ${m.size}`}
      shapeRendering={close ? 'geometricPrecision' : 'crispEdges'}
      aria-hidden="true"
      focusable="false"
    >
      {close ? <path d={d} fill="none" stroke="currentColor" strokeWidth={m.line} /> : <path d={d} fill="currentColor" />}
    </svg>
  );
}
