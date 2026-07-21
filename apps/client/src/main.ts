import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { router } from "./router";
import { createAppI18n, detectSystemLocale, type AppLocale } from "./plugins/i18n";
import { createAppVuetify } from "./plugins/vuetify";
import "katex/dist/katex.min.css";
import "./styles/chat.css";
import "./styles/code-theme.css";

function initialTheme(): "light" | "dark" {
  const saved = localStorage.getItem("aerina.themeMode");
  if (saved === "light" || saved === "dark") return saved;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function seedAccentCssVar() {
  const raw = localStorage.getItem("aerina.accentColor") || "#1565FF";
  const hex = /^#[0-9A-Fa-f]{6}$/.test(raw) ? raw.toUpperCase() : "#1565FF";
  document.documentElement.style.setProperty("--aerina-accent", hex);
}

function initialLocale(): AppLocale {
  const saved = localStorage.getItem("aerina.localeMode");
  if (saved === "en" || saved === "zh-CN") return saved;
  return detectSystemLocale();
}

seedAccentCssVar();
const app = createApp(App);
app.use(createPinia());
app.use(createAppI18n(initialLocale()));
app.use(createAppVuetify(initialTheme()));
app.use(router);
app.mount("#app");
