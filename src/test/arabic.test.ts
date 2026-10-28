import { describe, expect, it } from "vitest";

import { normalizeDigits, toArabicDigits } from "../lib/format";
import { normalizeSearch, toFtsQuery } from "../lib/arabic";

describe("Arabic digits", () => {
  it("converts ASCII digits to Arabic-Indic", () => {
    expect(toArabicDigits("1234")).toBe("١٢٣٤");
  });

  it("converts Arabic-Indic digits back to ASCII", () => {
    expect(normalizeDigits("١٢٣٤")).toBe("1234");
    expect(normalizeDigits("۱۲۳۴")).toBe("1234");
  });
});

describe("Arabic search normalization", () => {
  it("unifies hamza and alef variants", () => {
    expect(normalizeSearch("أحمد")).toBe(normalizeSearch("احمد"));
    expect(normalizeSearch("إسلام")).toBe(normalizeSearch("اسلام"));
  });

  it("unifies alef maqsura, teh marbuta and ya", () => {
    expect(normalizeSearch("مدرسة")).toBe(normalizeSearch("مدرسه"));
    expect(normalizeSearch("مصطفى")).toBe(normalizeSearch("مصطفي"));
  });

  it("strips diacritics and tatweel", () => {
    expect(normalizeSearch("مُحَمَّد")).toBe("محمد");
    expect(normalizeSearch("محـمد")).toBe("محمد");
  });

  it("quotes every term so FTS operators cannot leak", () => {
    expect(toFtsQuery("حليب كامل")).toBe('"حليب" "كامل"');
    expect(toFtsQuery("a OR b")).toBe('"a" "or" "b"');
    expect(toFtsQuery("   ")).toBe("");
  });
});
