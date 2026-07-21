<script setup lang="ts">
defineOptions({ name: "SettingsHubPage" });

import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

const { t } = useI18n();
const router = useRouter();

const items = computed(() => [
  {
    to: "/settings/providers",
    title: t("settings.providersTitle"),
    desc: t("settings.providersDesc"),
    icon: "mdi-server",
  },
  {
    to: "/settings/mcp",
    title: t("settings.mcpTitle"),
    desc: t("settings.mcpDesc"),
    icon: "mdi-connection",
  },
  {
    to: "/settings/appearance",
    title: t("settings.appearanceTitle"),
    desc: t("settings.appearanceDesc"),
    icon: "mdi-palette-outline",
  },
]);
</script>

<template>
  <div class="settings-page">
    <div class="settings-page-inner">
      <div class="settings-hero mb-5">
        <div class="text-h5 mb-1">{{ t("settings.title") }}</div>
        <div class="text-body-2 text-medium-emphasis">{{ t("settings.desc") }}</div>
      </div>
      <div class="settings-grid">
        <button
          v-for="item in items"
          :key="item.to"
          type="button"
          class="settings-card"
          @click="router.push(item.to)"
        >
          <div class="settings-card-icon">
            <v-icon :icon="item.icon" size="22" />
          </div>
          <div class="settings-card-body">
            <div class="settings-card-title">{{ item.title }}</div>
            <div class="settings-card-desc">{{ item.desc }}</div>
          </div>
          <v-icon icon="mdi-chevron-right" size="20" class="text-medium-emphasis" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.settings-hero {
  padding-bottom: 4px;
}
.settings-card {
  transition: border-color 0.12s ease, background 0.12s ease, transform 0.12s ease;
}
.settings-card:active {
  transform: scale(0.995);
}
.settings-page-inner {
  max-width: 760px;
  margin: 0 auto;
  padding: 28px 20px 40px;
}
.settings-grid {
  display: grid;
  gap: 10px;
}
.settings-card {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 100%;
  text-align: left;
  border: 1px solid var(--aerina-border);
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  box-shadow: var(--aerina-shadow-glass);
  border-radius: 14px;
  padding: 14px 16px;
  cursor: pointer;
  color: inherit;
}
.settings-card:hover {
  border-color: rgba(var(--v-theme-primary), 0.35);
  background: var(--aerina-material-heavy);
  transform: translateY(-1px);
}
.settings-card-icon {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-primary), 0.12);
  color: rgb(var(--v-theme-primary));
  flex: 0 0 auto;
}
.settings-card-body {
  flex: 1;
  min-width: 0;
}
.settings-card-title {
  font-weight: 600;
  margin-bottom: 2px;
}
.settings-card-desc {
  font-size: 0.82rem;
  color: rgba(var(--v-theme-on-surface), 0.6);
}
</style>
