/**
 * Arabic text helpers used by the UI layer.
 *
 * The normalization mirrors the Rust implementation in `domain/arabic.rs`
 * character for character, so a search typed in the UI and a search typed in
 * the database agree.
 *
 * Every code point is written as an escape so the file cannot be corrupted by
 * an editor that mangles Arabic literals.
 */

/**
 * Map a single character to its normalized form.
 *
 * Returns `null` for characters that are dropped (tashkeel, harakat,
 * tatweel and the Quranic marks).
 */
function mapChar(ch: string): string | null {
  const code = ch.codePointAt(0) ?? 0;

  // tashkeel / harakat / superscript alef / quranic marks / tatweel
  if (
    (code >= 0x064b && code <= 0x0652) ||
    code === 0x0670 ||
    (code >= 0x0610 && code <= 0x061a) ||
    (code >= 0x06d6 && code <= 0x06ed) ||
    code === 0x0640
  ) {
    return null;
  }

  // Arabic-Indic digits -> ASCII
  if (code >= 0x0660 && code <= 0x0669) return String(code - 0x0660);
  // Eastern Arabic-Indic digits -> ASCII
  if (code >= 0x06f0 && code <= 0x06f9) return String(code - 0x06f0);

  switch (code) {
    // hamza family -> alef
    case 0x0622:
    case 0x0623:
    case 0x0625:
    case 0x0671:
    case 0x0672:
    case 0x0673:
      return "ا";
    // alef maqsura -> yeh
    case 0x0649:
      return "ي";
    // teh marbuta -> heh
    case 0x0629:
      return "ه";
    // waw with hamza -> waw
    case 0x0624:
      return "و";
    // yeh with hamza -> yeh
    case 0x0626:
      return "ي";
    default:
      return ch;
  }
}

/**
 * Normalize a string for search: unify hamza and alef variants, drop
 * diacritics and tatweel, convert digits to ASCII and lowercase.
 */
export function normalizeSearch(value: string): string {
  let out = "";
  let pendingSpace = false;

  for (const ch of value) {
    if (/\s/.test(ch)) {
      pendingSpace = out.length > 0;
      continue;
    }
    const mapped = mapChar(ch);
    if (mapped === null) continue;
    if (pendingSpace) {
      out += " ";
      pendingSpace = false;
    }
    out += mapped.toLowerCase();
  }
  return out;
}

/**
 * Normalize a query into an FTS5 trigram MATCH expression.
 *
 * Every term is quoted so that FTS operators inside a user query can never
 * change the meaning of the search.
 */
export function toFtsQuery(value: string): string {
  const normalized = normalizeSearch(value);
  if (normalized.length === 0) return "";
  return normalized
    .split(" ")
    .filter((term) => term.length > 0)
    .map((term) => `"${term.replace(/"/g, '""')}"`)
    .join(" ");
}

/** The Arabic plural-free label for a count of items. */
export function itemCount(count: number, singular: string, dual: string, plural: string): string {
  if (count === 0) return `لا يوجد ${plural}`;
  if (count === 1) return `${singular} واحد`;
  if (count === 2) return `${dual} اثنان`;
  if (count <= 10) return `${count} ${plural}`;
  return `${count} ${singular}`;
}
