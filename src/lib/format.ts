/**
 * Formatting helpers.
 *
 * Money is always handled as integer minor units. These helpers only format
 * values that Rust has already computed, or convert user input text into
 * minor units for display purposes.
 */

const ARABIC_INDIC = ["٠", "١", "٢", "٣", "٤", "٥", "٦", "٧", "٨", "٩"];

/** Convert ASCII digits to Arabic-Indic digits. */
export function toArabicDigits(value: string): string {
  return value.replace(/[0-9]/g, (digit) => ARABIC_INDIC[Number(digit)] ?? digit);
}

/** Convert Arabic-Indic and Eastern Arabic-Indic digits to ASCII digits. */
export function normalizeDigits(value: string): string {
  return value
    .replace(/[٠-٩]/g, (d) => String(d.charCodeAt(0) - 0x0660))
    .replace(/[۰-۹]/g, (d) => String(d.charCodeAt(0) - 0x06f0));
}

/** The number of decimal places a currency uses. */
export function currencyExponent(code: string, fallback = 2): number {
  switch (code.toUpperCase()) {
    case "EGP":
    case "USD":
    case "EUR":
    case "SAR":
    case "AED":
      return 2;
    default:
      return fallback;
  }
}

/** Format integer minor units as a major-unit decimal string. */
export function formatMinor(minor: number, exponent = 2): string {
  const negative = minor < 0;
  const abs = Math.abs(minor);
  const scale = 10 ** exponent;
  const major = Math.trunc(abs / scale);
  const frac = abs % scale;
  const sign = negative ? "-" : "";
  if (exponent === 0) return `${sign}${major}`;
  return `${sign}${major}.${String(frac).padStart(exponent, "0")}`;
}

/**
 * Parse a user-typed amount into integer minor units.
 *
 * Accepts Arabic-Indic digits, `.` and `٫` as the decimal separator and
 * thousands separators. Returns `null` when the text is not a valid amount.
 */
export function parseAmount(text: string, exponent = 2): number | null {
  const cleaned = normalizeDigits(text)
    .trim()
    .replace(/[,\s٫,]/g, "")
    .replace(/٫/g, ".");
  if (cleaned.length === 0) return null;
  if (!/^\d*\.?\d*$/.test(cleaned)) return null;

  const [intPart = "0", fracPart = ""] = cleaned.split(".");
  if (intPart.length === 0 && fracPart.length === 0) return null;

  const intValue = Number(intPart || "0");
  if (!Number.isFinite(intValue)) return null;

  let frac = fracPart.slice(0, exponent).padEnd(exponent, "0");
  let minor = intValue * 10 ** exponent + Number(frac || "0");

  // Round half-up on the first dropped digit.
  if (fracPart.length > exponent) {
    const next = Number(fracPart[exponent]);
    const half = 5 * 10 ** Math.max(0, fracPart.length - exponent - 1);
    if (next * half >= 10 ** Math.max(0, fracPart.length - exponent - 1)) minor += 1;
  }
  return minor;
}

/** Format integer thousandths as a decimal string. */
export function formatThousandths(thousandths: number): string {
  const negative = thousandths < 0;
  const abs = Math.abs(thousandths);
  const major = Math.trunc(abs / 1000);
  const frac = abs % 1000;
  const sign = negative ? "-" : "";
  if (frac === 0) return `${sign}${major}`;
  if (frac % 100 === 0) return `${sign}${major}.${frac / 100}`;
  if (frac % 10 === 0) return `${sign}${major}.${String(frac / 10).padStart(2, "0")}`;
  return `${sign}${major}.${String(frac).padStart(3, "0")}`;
}

/** Parse a user-typed quantity into integer thousandths. */
export function parseQuantity(text: string): number | null {
  const cleaned = normalizeDigits(text).trim().replace(/٫/g, ".");
  if (cleaned.length === 0) return null;
  if (!/^\d*\.?\d*$/.test(cleaned)) return null;
  const [intPart = "0", fracPart = ""] = cleaned.split(".");
  const intValue = Number(intPart || "0");
  if (!Number.isFinite(intValue)) return null;
  const frac = (fracPart + "000").slice(0, 3);
  const value = intValue * 1000 + Number(frac || "0");
  return value > 0 ? value : null;
}

/** Today's date as `YYYY-MM-DD` in the local timezone. */
export function todayIso(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

/** Format an ISO date using the user's preferred pattern. */
export function formatDate(iso: string, pattern = "dd/MM/yyyy"): string {
  if (!iso) return "";
  const [year, month, day] = iso.split("-");
  if (!year || !month || !day) return iso;
  return pattern.replace("yyyy", year).replace("MM", month).replace("dd", day);
}

/** Format a byte count for display. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} كيلوبايت`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} ميجابايت`;
}

/** Format a timestamp as a readable Arabic-neutral string. */
export function formatTimestamp(value: string): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("ar-EG", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
