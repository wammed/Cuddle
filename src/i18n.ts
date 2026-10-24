import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import translationEN from "./locales/en/translation.json";
import translationJA from "./locales/ja/translation.json";

export const resources = {
  en: {
    translation: translationEN,
  },
  ja: {
    translation: translationJA,
  },
} as const;

export type SupportedLanguage = "ja" | "en";

// Detect initial language: localStorage -> navigator.language -> "en"
const getInitialLanguage = (): SupportedLanguage => {
  const saved = localStorage.getItem("fileworkbench_language");
  if (saved === "ja" || saved === "en") {
    return saved;
  }
  const sysLang = navigator.language.toLowerCase();
  if (sysLang.startsWith("ja")) {
    return "ja";
  }
  return "en";
};

i18n.use(initReactI18next).init({
  resources,
  lng: getInitialLanguage(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false, // React already escapes values
  },
});

export const changeLanguage = (lang: SupportedLanguage) => {
  localStorage.setItem("fileworkbench_language", lang);
  return i18n.changeLanguage(lang);
};

export default i18n;
