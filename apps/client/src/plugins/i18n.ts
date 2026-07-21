import { createI18n } from "vue-i18n";
import en from "../locales/en.json";
import zhCN from "../locales/zh-CN.json";

export type AppLocale = "en" | "zh-CN";

export function detectSystemLocale(): AppLocale {
  const lang = (navigator.language || "en").toLowerCase();
  if (lang.startsWith("zh")) return "zh-CN";
  return "en";
}

export function createAppI18n(locale: AppLocale) {
  return createI18n({
    legacy: false,
    locale,
    fallbackLocale: "en",
    messages: {
      en,
      "zh-CN": zhCN,
    },
  });
}
