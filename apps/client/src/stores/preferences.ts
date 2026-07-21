import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import { useTheme } from "vuetify";
import { useI18n } from "vue-i18n";
import { detectSystemLocale, type AppLocale } from "../plugins/i18n";

const THEME_KEY = "aerina.themeMode";
const LOCALE_KEY = "aerina.localeMode";
const ACCENT_KEY = "aerina.accentColor";

export type ThemeMode = "system" | "light" | "dark";
export type LocaleMode = "system" | AppLocale;

/** Brand blue default — matches logo / rail mark */
export const DEFAULT_ACCENT = "#1565FF";

export const ACCENT_PRESETS = [
  { id: "blue", labelKey: "appearance.accentBlue", value: "#1565FF" },
  { id: "indigo", labelKey: "appearance.accentIndigo", value: "#4F46E5" },
  { id: "violet", labelKey: "appearance.accentViolet", value: "#7C3AED" },
  { id: "teal", labelKey: "appearance.accentTeal", value: "#0D9488" },
  { id: "green", labelKey: "appearance.accentGreen", value: "#16A34A" },
  { id: "orange", labelKey: "appearance.accentOrange", value: "#EA580C" },
  { id: "rose", labelKey: "appearance.accentRose", value: "#E11D48" },
  { id: "slate", labelKey: "appearance.accentSlate", value: "#475569" },
] as const;

function readThemeMode(): ThemeMode {
  const v = localStorage.getItem(THEME_KEY);
  if (v === "light" || v === "dark" || v === "system") return v;
  return "system";
}

function readLocaleMode(): LocaleMode {
  const v = localStorage.getItem(LOCALE_KEY);
  if (v === "system" || v === "en" || v === "zh-CN") return v;
  return "system";
}

function normalizeHex(input: string): string | null {
  const v = input.trim();
  if (/^#[0-9A-Fa-f]{6}$/.test(v)) return v.toUpperCase();
  if (/^[0-9A-Fa-f]{6}$/.test(v)) return `#${v.toUpperCase()}`;
  return null;
}

function readAccentColor(): string {
  const raw = localStorage.getItem(ACCENT_KEY);
  if (!raw) return DEFAULT_ACCENT;
  return normalizeHex(raw) ?? DEFAULT_ACCENT;
}

function systemIsDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

/** Lift primary slightly for dark surfaces (Apple-like vibrancy on dark). */
export function lightenHex(hex: string, amount = 0.22): string {
  const h = normalizeHex(hex) ?? DEFAULT_ACCENT;
  const r = parseInt(h.slice(1, 3), 16);
  const g = parseInt(h.slice(3, 5), 16);
  const b = parseInt(h.slice(5, 7), 16);
  const mix = (c: number) => Math.min(255, Math.round(c + (255 - c) * amount));
  return `#${[mix(r), mix(g), mix(b)].map((n) => n.toString(16).padStart(2, "0")).join("").toUpperCase()}`;
}

export function applyAccentToTheme(
  theme: ReturnType<typeof useTheme>,
  accent: string,
) {
  const lightPrimary = normalizeHex(accent) ?? DEFAULT_ACCENT;
  const darkPrimary = lightenHex(lightPrimary, 0.28);
  theme.themes.value.light.colors.primary = lightPrimary;
  theme.themes.value.dark.colors.primary = darkPrimary;
  document.documentElement.style.setProperty("--aerina-accent", lightPrimary);
  document.documentElement.style.setProperty("--aerina-accent-dark", darkPrimary);
}

export const usePreferencesStore = defineStore("preferences", () => {
  const themeMode = ref<ThemeMode>(readThemeMode());
  const localeMode = ref<LocaleMode>(readLocaleMode());
  const accentColor = ref(readAccentColor());
  const systemDark = ref(systemIsDark());

  const resolvedTheme = computed<"light" | "dark">(() => {
    if (themeMode.value === "system") return systemDark.value ? "dark" : "light";
    return themeMode.value;
  });

  const resolvedLocale = computed<AppLocale>(() => {
    if (localeMode.value === "system") return detectSystemLocale();
    return localeMode.value;
  });

  function setThemeMode(mode: ThemeMode) {
    themeMode.value = mode;
    localStorage.setItem(THEME_KEY, mode);
  }

  function setLocaleMode(mode: LocaleMode) {
    localeMode.value = mode;
    localStorage.setItem(LOCALE_KEY, mode);
  }

  function setAccentColor(color: string) {
    const hex = normalizeHex(color);
    if (!hex) throw new Error(`invalid accent color: ${color}`);
    accentColor.value = hex;
    localStorage.setItem(ACCENT_KEY, hex);
  }

  function bindRuntime() {
    const theme = useTheme();
    const { locale } = useI18n();

    const applyTheme = () => {
      theme.change(resolvedTheme.value);
      applyAccentToTheme(theme, accentColor.value);
    };
    const applyLocale = () => {
      locale.value = resolvedLocale.value;
      document.documentElement.lang = resolvedLocale.value;
    };

    applyTheme();
    applyLocale();

    watch(resolvedTheme, applyTheme);
    watch(resolvedLocale, applyLocale);
    watch(accentColor, (c) => applyAccentToTheme(theme, c));

    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onScheme = () => {
      systemDark.value = mql.matches;
    };
    mql.addEventListener("change", onScheme);

    return () => mql.removeEventListener("change", onScheme);
  }

  return {
    themeMode,
    localeMode,
    accentColor,
    resolvedTheme,
    resolvedLocale,
    setThemeMode,
    setLocaleMode,
    setAccentColor,
    bindRuntime,
  };
});
