import { describe, it, expect } from "vitest";
import translationEN from "../locales/en/translation.json";
import translationJA from "../locales/ja/translation.json";

function extractAllKeys(obj: Record<string, unknown>, prefix = ""): string[] {
  let keys: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullPath = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      keys = keys.concat(extractAllKeys(value as Record<string, unknown>, fullPath));
    } else {
      keys.push(fullPath);
    }
  }
  return keys.sort();
}

describe("i18n bilingual dictionary parity (EN ↔ JA)", () => {
  const enKeys = extractAllKeys(translationEN);
  const jaKeys = extractAllKeys(translationJA);

  it("should have identical key counts between EN and JA", () => {
    expect(jaKeys.length).toBe(enKeys.length);
  });

  it("should have zero missing keys in Japanese dictionary compared to English", () => {
    const missingInJa = enKeys.filter((key) => !jaKeys.includes(key));
    expect(missingInJa).toEqual([]);
  });

  it("should have zero missing keys in English dictionary compared to Japanese", () => {
    const missingInEn = jaKeys.filter((key) => !enKeys.includes(key));
    expect(missingInEn).toEqual([]);
  });

  it("should have non-empty string values for all keys in both dictionaries", () => {
    const checkValues = (obj: Record<string, unknown>, path = "") => {
      for (const [key, val] of Object.entries(obj)) {
        const fullPath = path ? `${path}.${key}` : key;
        if (typeof val === "object" && val !== null) {
          checkValues(val as Record<string, unknown>, fullPath);
        } else {
          expect(typeof val).toBe("string");
          expect((val as string).trim().length).toBeGreaterThan(0);
        }
      }
    };

    checkValues(translationEN);
    checkValues(translationJA);
  });
});
