// Window button glyphs in the Windows 11 style: 10×10 px with 1 px lines at 100 %. They are drawn
// in whole screen pixels at every display scale (125 %, 150 %…), so lines never fall between
// pixels and blur; minimize, maximize and restore are filled rectangles, the close X a stroke.
import type { GlyphKind } from './types';

export interface GlyphMetrics {
  size: number; // the glyph's width and height in screen pixels
  line: number; // line width in screen pixels
  shift: number; // restore: how far the back window is shifted up and right
  cssSize: number; // the size in CSS pixels at this scale
}

export function glyphMetrics(pixelRatio: number): GlyphMetrics {
  const ratio = Number.isFinite(pixelRatio) && pixelRatio > 0 ? pixelRatio : 1;
  const size = Math.round(10 * ratio);
  return {
    size,
    line: Math.max(1, Math.round(ratio)),
    shift: Math.max(2, Math.round(2 * ratio)),
    cssSize: size / ratio,
  };
}

// Filled with the non-zero rule: clockwise rectangles add up (overlaps stay filled), a
// counter-clockwise one cuts a hole.
const rect = (x: number, y: number, w: number, h: number) => `M${x} ${y}h${w}v${h}h${-w}z`;
const hole = (x: number, y: number, w: number, h: number) => `M${x} ${y}v${h}h${w}v${-h}z`;

// A square outline of side `side` at (x, y).
const outline = (x: number, y: number, side: number, line: number) =>
  rect(x, y, side, side) + hole(x + line, y + line, side - 2 * line, side - 2 * line);

// The SVG path of a glyph in a viewBox of `size`×`size` screen pixels.
export function glyphPath(kind: GlyphKind, m: GlyphMetrics): string {
  const { size, line, shift } = m;
  if (kind === 'minimize') return rect(0, Math.floor((size - line) / 2), size, line);
  if (kind === 'maximize') return outline(0, 0, size, line);
  if (kind === 'close') return `M0 0L${size} ${size}M${size} 0L0 ${size}`;
  // Restore: the front window at the bottom left, of the back one only its top and right edges.
  const side = size - shift;
  return outline(0, shift, side, line) + rect(shift, 0, side, line) + rect(size - line, 0, line, side);
}
