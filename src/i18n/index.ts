import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import ptBR from "./locales/pt-BR.json";

export const SUPPORTED_LANGUAGES = [
  { code: "pt-BR", label: "Português (Brasil)" },
  { code: "en", label: "English" },
] as const;

export type Language = (typeof SUPPORTED_LANGUAGES)[number]["code"];

/** Maps any detected locale (pt, pt-PT, en-US…) onto a supported language. */
export function normalizeLanguage(lng: string): Language {
  return lng.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const resources = {
  en: { translation: en },
  "pt-BR": { translation: ptBR },
} as const;

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: "en",
    supportedLngs: SUPPORTED_LANGUAGES.map((l) => l.code),
    interpolation: { escapeValue: false },
    detection: {
      order: ["localStorage", "navigator"],
      lookupLocalStorage: "chainfolio.language",
      caches: ["localStorage"],
      convertDetectedLanguage: normalizeLanguage,
    },
  });

export default i18n;
