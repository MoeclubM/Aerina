<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import { isTauri } from "@tauri-apps/api/core";
import { usePreferencesStore } from "./stores/preferences";

const { t } = useI18n();
const route = useRoute();
const preferences = usePreferencesStore();
const htmlPreviewWindow = computed(() => route.name === "html-preview");
let unbind: (() => void) | undefined;

const showCustomTitleBar = ref(false);
const isMaximized = ref(false);
let appWindow: {
  minimize: () => Promise<void>;
  toggleMaximize: () => Promise<void>;
  close: () => Promise<void>;
  isMaximized: () => Promise<boolean>;
  onResized: (handler: () => void) => Promise<() => void>;
} | null = null;
let unlistenResize: (() => void) | undefined;

async function refreshMaximized() {
  if (!appWindow) return;
  isMaximized.value = await appWindow.isMaximized();
}

const minimize = () => {
  void appWindow?.minimize();
};
const toggleMaximize = async () => {
  await appWindow?.toggleMaximize();
  await refreshMaximized();
};
const close = () => {
  void appWindow?.close();
};

onMounted(async () => {
  unbind = preferences.bindRuntime();

  if (!isTauri()) return;

  const isWindows = navigator.userAgent.toLowerCase().includes("windows");
  if (!isWindows) return;

  showCustomTitleBar.value = true;
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    appWindow = getCurrentWindow();
    await refreshMaximized();
    unlistenResize = await appWindow.onResized(() => {
      void refreshMaximized();
    });
  } catch (e) {
    console.error("Failed to load Tauri window API", e);
  }
});

onUnmounted(() => {
  unbind?.();
  unlistenResize?.();
});
</script>

<template>
  <v-app class="aerina-app" :class="{ 'has-custom-titlebar': showCustomTitleBar, 'html-preview-window': htmlPreviewWindow }">
    <div v-if="showCustomTitleBar" class="custom-titlebar" data-tauri-drag-region>
      <div class="titlebar-drag" data-tauri-drag-region />
      <div class="titlebar-actions">
        <button class="titlebar-btn" type="button" :title="t('common.minimize') || 'Minimize'" @click="minimize">
          <v-icon icon="mdi-minus" size="16" />
        </button>
        <button
          class="titlebar-btn"
          type="button"
          :title="t('common.maximize') || 'Maximize'"
          @click="toggleMaximize"
        >
          <v-icon :icon="isMaximized ? 'mdi-checkbox-blank-outline' : 'mdi-window-maximize'" size="14" />
        </button>
        <button class="titlebar-btn close" type="button" :title="t('common.close') || 'Close'" @click="close">
          <v-icon icon="mdi-close" size="16" />
        </button>
      </div>
    </div>
    <div class="app-content-container">
      <router-view />
    </div>
  </v-app>
</template>

<style>
.aerina-app,
.aerina-app .v-application__wrap {
  height: 100%;
  min-height: 100%;
  position: relative;
}

/* Unified Merged Titlebar Overlay */
.custom-titlebar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  user-select: none;
  z-index: 1005;
  pointer-events: none;
  background: transparent;
}
.titlebar-drag {
  flex: 1 1 auto;
  min-width: 0;
  height: 100%;
  pointer-events: auto;
}
.titlebar-actions {
  display: flex;
  height: 100%;
  flex: 0 0 auto;
  pointer-events: auto;
}
.titlebar-btn {
  width: 44px;
  height: 100%;
  display: grid;
  place-items: center;
  background: transparent;
  border: 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}
.titlebar-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
  color: rgb(var(--v-theme-on-surface));
}
.titlebar-btn:active {
  background: rgba(var(--v-theme-on-surface), 0.16);
}
.titlebar-btn.close:hover {
  background: #e81123 !important;
  color: #ffffff !important;
}
.titlebar-btn.close:active {
  background: #f1707a !important;
  color: #ffffff !important;
}

.app-content-container {
  flex: 1;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>