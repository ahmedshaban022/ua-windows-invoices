/**
 * Brand colour handling.
 *
 * The owner picks one colour in Settings. Everything that carries the brand
 * follows it: the header bar, the primary buttons, the focused inputs, the
 * invoice rules, the table head and the totals row. The derived tones are
 * computed from the chosen colour so the interface stays readable whatever
 * colour is picked — a pale yellow still gets dark text on it, and a very
 * dark colour still gets light text.
 */

/** The colour the application starts with. */
export const DEFAULT_BRAND_COLOR = "#0b68e6";

interface Rgb {
  r: number;
  g: number;
  b: number;
}

const WHITE: Rgb = { r: 255, g: 255, b: 255 };
const INK: Rgb = { r: 17, g: 24, b: 39 };

/** Read a `#rrggbb` colour, or `null` when the text is not one. */
export function parseHexColor(value: string | null | undefined): Rgb | null {
  const hex = (value ?? "").trim().replace(/^#/, "");
  if (!/^[0-9a-f]{6}$/i.test(hex)) return null;
  return {
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
  };
}

/** Write an `#rrggbb` colour. */
export function formatHexColor(rgb: Rgb): string {
  const part = (value: number) =>
    Math.round(Math.min(255, Math.max(0, value)))
      .toString(16)
      .padStart(2, "0");
  return `#${part(rgb.r)}${part(rgb.g)}${part(rgb.b)}`;
}

/** Blend towards white for a positive amount, towards black for a negative one. */
function blend(rgb: Rgb, amount: number): Rgb {
  const target = amount >= 0 ? 255 : 0;
  const ratio = Math.abs(amount);
  return {
    r: rgb.r + (target - rgb.r) * ratio,
    g: rgb.g + (target - rgb.g) * ratio,
    b: rgb.b + (target - rgb.b) * ratio,
  };
}

/** The WCAG relative luminance of a colour. */
function luminance(rgb: Rgb): number {
  const channel = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(rgb.r) + 0.7152 * channel(rgb.g) + 0.0722 * channel(rgb.b);
}

/** The WCAG contrast ratio between two colours. */
function contrastRatio(a: Rgb, b: Rgb): number {
  const first = luminance(a);
  const second = luminance(b);
  const light = Math.max(first, second);
  const dark = Math.min(first, second);
  return (light + 0.05) / (dark + 0.05);
}

/** A light tint of the brand colour, light enough to sit behind text. */
function tint(rgb: Rgb, amount: number, minLuminance: number): Rgb {
  let result = blend(rgb, amount);
  for (let step = 0; step < 40 && luminance(result) < minLuminance; step += 1) {
    result = blend(result, 0.08);
  }
  return result;
}

/** A deeper version of the brand colour that still passes AA on the tint. */
function deepen(rgb: Rgb, background: Rgb): Rgb {
  let result = rgb;
  for (let step = 0; step < 40 && contrastRatio(result, background) < 4.5; step += 1) {
    result = blend(result, -0.08);
  }
  return result;
}

/** The readable text colour on top of a solid brand background. */
function onBrand(rgb: Rgb): Rgb {
  return contrastRatio(WHITE, rgb) >= contrastRatio(INK, rgb) ? WHITE : INK;
}

export interface BrandPalette {
  /** The colour the owner chose. */
  brand: string;
  /** A deeper tone of it, readable on the soft tint. */
  strong: string;
  /** A very light tint, used behind brand text. */
  soft: string;
  /** An even lighter tint, used for table rows. */
  softer: string;
  /** The tint used when a soft surface is hovered. */
  softHover: string;
  /** The text colour that reads on the solid brand colour. */
  contrast: string;
  /** The focus ring around keyboard-focused fields. */
  focusRing: string;
}

/** Derive the whole palette from one brand colour. */
export function brandPalette(value: string | null | undefined): BrandPalette {
  const rgb = parseHexColor(value) ?? parseHexColor(DEFAULT_BRAND_COLOR)!;
  const soft = tint(rgb, 0.88, 0.86);
  const softer = tint(rgb, 0.94, 0.93);
  const softHover = tint(rgb, 0.8, 0.8);
  const strong = deepen(rgb, soft);
  return {
    brand: formatHexColor(rgb),
    strong: formatHexColor(strong),
    soft: formatHexColor(soft),
    softer: formatHexColor(softer),
    softHover: formatHexColor(softHover),
    contrast: formatHexColor(onBrand(rgb)),
    focusRing: `0 0 0 3px rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.28)`,
  };
}

/** Push the brand palette into the document so every surface follows it. */
export function applyBrandColor(value: string | null | undefined): void {
  if (typeof document === "undefined") return;
  const palette = brandPalette(value);
  const root = document.documentElement;
  root.style.setProperty("--brand", palette.brand);
  root.style.setProperty("--brand-strong", palette.strong);
  root.style.setProperty("--brand-soft", palette.soft);
  root.style.setProperty("--brand-softer", palette.softer);
  root.style.setProperty("--brand-soft-hover", palette.softHover);
  root.style.setProperty("--brand-contrast", palette.contrast);
  root.style.setProperty("--focus-ring", palette.focusRing);
}
