<script setup lang="ts">
defineOptions({ name: "AppearancePage" });

import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import {
  ACCENT_PRESETS,
  DEFAULT_ACCENT,
  usePreferencesStore,
  type LocaleMode,
  type ThemeMode,
} from "../../stores/preferences";

const { t } = useI18n();
const router = useRouter();
const preferences = usePreferencesStore();
const error = ref("");

const themeItems = computed(() => [
  { title: t("common.system"), value: "system" as ThemeMode },
  { title: t("common.light"), value: "light" as ThemeMode },
  { title: t("common.dark"), value: "dark" as ThemeMode },
]);

const localeItems = computed(() => [
  { title: t("common.system"), value: "system" as LocaleMode },
  { title: "English", value: "en" as LocaleMode },
  { title: "简体中文", value: "zh-CN" as LocaleMode },
]);

const accentPresets = computed(() =>
  ACCENT_PRESETS.map((item) => ({
    ...item,
    title: t(item.labelKey),
  })),
);

const customAccent = ref(preferences.accentColor);
watch(
  () => preferences.accentColor,
  (value) => {
    customAccent.value = value;
  },
);

function selectPreset(value: string) {
  preferences.setAccentColor(value);
  customAccent.value = value;
}

function applyCustomAccent() {
  try {
    preferences.setAccentColor(customAccent.value);
    error.value = "";
  } catch (e) {
    error.value = String(e);
  }
}

function resetAccent() {
  preferences.setAccentColor(DEFAULT_ACCENT);
  customAccent.value = DEFAULT_ACCENT;
}
</script>

<template>
  <div class="appearance-page settings-page">
    <div class="appearance-inner">
      <header class="appearance-header">
        <button
          type="button"
          class="settings-mobile-back"
          :aria-label="t('common.back')"
          @click="router.push('/settings')"
        >
          <v-icon icon="mdi-arrow-left" size="22" />
        </button>
        <div class="appearance-header-text">
          <h1 class="appearance-title">{{ t("appearance.title") }}</h1>
          <p class="appearance-desc">{{ t("appearance.desc") }}</p>
        </div>
      </header>

      <section class="settings-surface">
        <div class="setting-block">
          <div class="section-title">{{ t("appearance.theme") }}</div>
          <div class="segmented" role="radiogroup" :aria-label="t('appearance.theme')">
            <button
              v-for="item in themeItems"
              :key="item.value"
              type="button"
              class="segmented-item"
              role="radio"
              :aria-checked="preferences.themeMode === item.value"
              :class="{ active: preferences.themeMode === item.value }"
              @click="preferences.setThemeMode(item.value)"
            >
              {{ item.title }}
            </button>
          </div>
        </div>

        <div class="setting-block">
          <div class="section-title">{{ t("appearance.accent") }}</div>
          <div class="section-desc">{{ t("appearance.accentHint") }}</div>
          <div class="accent-grid">
            <button
              v-for="item in accentPresets"
              :key="item.id"
              type="button"
              class="accent-swatch"
              :class="{ active: preferences.accentColor.toUpperCase() === item.value.toUpperCase() }"
              :style="{ '--swatch': item.value }"
              :title="item.title"
              :aria-label="item.title"
              @click="selectPreset(item.value)"
            >
              <span class="accent-swatch-fill" />
            </button>
          </div>
          <div class="accent-custom">
            <label class="field-label" for="accent-hex">{{ t("appearance.customAccent") }}</label>
            <div class="accent-custom-row">
              <label class="accent-native-wrap" :title="t('appearance.customAccent')">
                <input
                  class="accent-native"
                  type="color"
                  :value="preferences.accentColor"
                  :aria-label="t('appearance.customAccent')"
                  @input="selectPreset(($event.target as HTMLInputElement).value)"
                />
              </label>
              <input
                id="accent-hex"
                v-model="customAccent"
                class="field-input accent-hex"
                type="text"
                spellcheck="false"
                autocomplete="off"
                @keydown.enter.prevent="applyCustomAccent"
                @blur="applyCustomAccent"
              />
              <button type="button" class="btn-ghost" @click="resetAccent">
                {{ t("appearance.resetAccent") }}
              </button>
            </div>
          </div>
        </div>

        <div class="setting-block setting-block-last">
          <div class="section-title">{{ t("appearance.language") }}</div>
          <div class="segmented" role="radiogroup" :aria-label="t('appearance.language')">
            <button
              v-for="item in localeItems"
              :key="item.value"
              type="button"
              class="segmented-item"
              role="radio"
              :aria-checked="preferences.localeMode === item.value"
              :class="{ active: preferences.localeMode === item.value }"
              @click="preferences.setLocaleMode(item.value)"
            >
              {{ item.title }}
            </button>
          </div>
        </div>
      </section>

      <div v-if="error" class="error-banner">{{ error }}</div>
    </div>
  </div>
</template>

<style scoped>
.appearance-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.appearance-inner {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 28px 20px 48px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.appearance-header {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 4px;
}
.appearance-header-text {
  flex: 1;
  min-width: 0;
}
.appearance-title {
  margin: 0 0 6px;
  color: rgb(var(--v-theme-on-background));
  font-size: 1.5rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.15;
}
.appearance-desc,
.section-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.82rem;
  line-height: 1.45;
}
.appearance-desc {
  font-size: 0.9rem;
}
.settings-surface {
  padding: 18px;
  border: 0;
  border-radius: 16px;
  background: var(--miuix-surface-container);
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
  box-shadow: none;
}
.setting-block {
  padding: 2px 0 20px;
  margin-bottom: 18px;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.28);
}
.setting-block-last {
  padding-bottom: 0;
  margin-bottom: 0;
  border-bottom: 0;
}
.section-title {
  margin-bottom: 5px;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.92rem;
  font-weight: 620;
  letter-spacing: -0.01em;
}
.segmented {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  max-width: 430px;
  margin-top: 10px;
  padding: 4px;
  border: 0;
  border-radius: 12px;
  background: var(--miuix-surface);
}
.segmented-item {
  min-width: 0;
  min-height: 34px;
  padding: 6px 10px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.65);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 570;
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease, transform 100ms ease-out;
}
.segmented-item:hover,
.segmented-item:focus-visible {
  color: rgb(var(--v-theme-on-surface));
  outline: none;
}
.segmented-item.active {
  background: var(--miuix-surface-container-high);
  color: rgb(var(--v-theme-on-surface));
  box-shadow: none;
}
.segmented-item:active {
  background: color-mix(in srgb, var(--miuix-surface-container-high) 90%, var(--miuix-on-container) 10%);
  transform: none;
}
.accent-grid {
  display: grid;
  grid-template-columns: repeat(8, 34px);
  gap: 10px;
  margin: 14px 0 16px;
}
.accent-swatch {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  padding: 0;
  border: 2px solid transparent;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  transition: border-color 120ms ease, transform 100ms ease-out;
}
.accent-swatch:hover,
.accent-swatch:focus-visible {
  border-color: rgba(var(--v-theme-on-surface), 0.24);
  outline: none;
}
.accent-swatch.active {
  border-color: rgb(var(--v-theme-primary));
}
.accent-swatch:active {
  transform: scale(0.94);
}
.accent-swatch-fill {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--swatch);
  box-shadow: 0 1px 4px rgba(12, 20, 36, 0.18);
}
.accent-custom {
  max-width: 470px;
}
.field-label {
  display: block;
  margin-bottom: 5px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.78rem;
  font-weight: 560;
}
.accent-custom-row {
  display: grid;
  grid-template-columns: 42px minmax(100px, 160px) auto;
  align-items: center;
  gap: 8px;
}
.accent-native-wrap {
  width: 40px;
  height: 40px;
  overflow: hidden;
  border: 1px solid rgba(var(--v-border-color), 0.5);
  border-radius: 16px;
  cursor: pointer;
}
.accent-native {
  width: 52px;
  height: 52px;
  margin: -5px;
  border: 0;
  padding: 0;
  cursor: pointer;
}
.field-input {
  width: 100%;
  min-width: 0;
  height: var(--aerina-control-height);
  box-sizing: border-box;
  padding: 0 16px;
  border: 0;
  border-radius: var(--aerina-control-radius);
  outline: none;
  background: var(--miuix-secondary-container);
  color: rgb(var(--v-theme-on-surface));
  font: inherit;
  font-size: 0.9rem;
  transition: border-color 140ms ease, background 140ms ease, box-shadow 140ms ease;
}
.field-input:focus {
  background: var(--miuix-secondary-container);
  box-shadow: inset 0 0 0 2px rgb(var(--v-theme-primary));
}
.btn-ghost {
  min-height: var(--aerina-control-height);
  padding: 0 16px;
  border: 0;
  border-radius: var(--aerina-control-radius);
  background: var(--miuix-secondary-container);
  color: rgb(var(--v-theme-on-surface));
  font: inherit;
  font-size: 0.82rem;
  font-weight: 600;
  cursor: pointer;
}
.btn-ghost:hover,
.btn-ghost:focus-visible {
  background: color-mix(in srgb, var(--miuix-secondary-container) 94%, var(--miuix-on-container) 6%);
  outline: none;
}
.error-banner {
  padding: 10px 12px;
  border-radius: 12px;
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
  font-size: 0.86rem;
  overflow-wrap: anywhere;
}

@media (max-width: 679px) {
  .appearance-inner {
    width: 100%;
    padding: 18px 14px 36px;
    gap: 13px;
  }
  .appearance-header {
    align-items: center;
    margin-bottom: 0;
  }
  .appearance-title {
    max-width: 100%;
    overflow: hidden;
    font-size: 1.15rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .appearance-desc {
    display: none;
  }
  .settings-surface {
    padding: 15px 14px 14px;
    border-radius: 16px;
  }
  .accent-grid {
    grid-template-columns: repeat(4, 34px);
  }
  .accent-custom-row {
    grid-template-columns: 42px minmax(0, 1fr);
  }
  .accent-custom-row .btn-ghost {
    grid-column: 1 / -1;
    justify-self: start;
  }
}

@media (max-width: 360px) {
  .segmented-item {
    padding-inline: 6px;
    font-size: 0.76rem;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .settings-surface {
    background: var(--miuix-surface-container);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}
</style>
