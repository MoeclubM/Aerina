<script setup lang="ts">
defineOptions({ name: "SettingsHubPage" });

import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";

const { t } = useI18n();
const router = useRouter();

const groups = computed(() => [
  {
    key: "personalization",
    title: t("settings.personalization"),
    items: [
      {
        to: "/settings/profile",
        title: t("settings.profileTitle"),
        desc: t("settings.profileDesc"),
        icon: "mdi-account-circle-outline",
      },
      {
        to: "/settings/assistants",
        title: t("settings.assistantRolesTitle"),
        desc: t("settings.assistantRolesDesc"),
        icon: "mdi-account-cog-outline",
      },
      {
        to: "/settings/appearance",
        title: t("settings.appearanceTitle"),
        desc: t("settings.appearanceDesc"),
        icon: "mdi-palette-outline",
      },
    ],
  },
  {
    key: "connections",
    title: t("settings.modelsAndTools"),
    items: [
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
    ],
  },
]);
</script>

<template>
  <div class="settings-page">
    <div class="settings-page-inner">
      <header class="settings-hero">
        <h1 class="settings-title">{{ t("settings.title") }}</h1>
        <p class="settings-desc">{{ t("settings.desc") }}</p>
      </header>

      <section v-for="group in groups" :key="group.key" class="settings-group">
        <h2 class="settings-group-title">{{ group.title }}</h2>
        <div class="settings-list">
          <button
            v-for="item in group.items"
            :key="item.to"
            type="button"
            class="settings-row"
            @click="router.push(item.to)"
          >
            <span class="settings-row-icon">
              <v-icon :icon="item.icon" size="19" />
            </span>
            <span class="settings-row-body">
              <span class="settings-row-title">{{ item.title }}</span>
              <span class="settings-row-desc">{{ item.desc }}</span>
            </span>
            <v-icon icon="mdi-chevron-right" size="18" class="settings-row-chevron" />
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.settings-page-inner {
  width: 100%;
  max-width: 720px;
  margin: 0 auto;
  padding: 26px 20px 40px;
  box-sizing: border-box;
}
.settings-hero {
  margin-bottom: 20px;
}
.settings-title {
  margin: 0 0 4px;
  font-size: 1.38rem;
  line-height: 1.25;
  font-weight: 680;
  letter-spacing: -0.025em;
}
.settings-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.84rem;
  line-height: 1.45;
}
.settings-group + .settings-group {
  margin-top: 20px;
}
.settings-group-title {
  margin: 0 0 7px 4px;
  color: rgba(var(--v-theme-on-surface), 0.66);
  font-size: 0.875rem;
  font-weight: 700;
  letter-spacing: 0;
  line-height: 1.4;
}
.settings-list {
  overflow: hidden;
  border: 0;
  border-radius: 16px;
  background: var(--miuix-surface-container);
}
.settings-row {
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr) 18px;
  align-items: center;
  gap: 11px;
  width: 100%;
  min-height: 68px;
  padding: 10px 16px;
  border: 0;
  border-bottom: 1px solid var(--miuix-divider);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
  transition: background var(--aerina-spring);
}
.settings-row:last-child {
  border-bottom: 0;
}
.settings-row:hover,
.settings-row:focus-visible {
  background: color-mix(in srgb, var(--miuix-surface-container) 94%, var(--miuix-on-container) 6%);
  outline: none;
}
.settings-row:active {
  background: color-mix(in srgb, var(--miuix-surface-container) 90%, var(--miuix-on-container) 10%);
}
.settings-row-icon {
  width: 36px;
  height: 36px;
  display: grid;
  place-items: center;
  border-radius: 12px;
  background: rgba(var(--v-theme-primary), 0.11);
  color: rgb(var(--v-theme-primary));
}
.settings-row-body {
  display: grid;
  min-width: 0;
  gap: 2px;
}
.settings-row-title {
  overflow: hidden;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.88rem;
  font-weight: 630;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.settings-row-desc {
  overflow: hidden;
  color: var(--miuix-summary);
  font-size: 0.8125rem;
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.settings-row-chevron {
  color: rgba(var(--v-theme-on-surface), 0.42);
}

@media (max-width: 679px) {
  .settings-page-inner {
    width: 100%;
    padding: 18px 14px 28px;
  }
  .settings-hero {
    margin-bottom: 16px;
  }
  .settings-title {
    display: none;
  }
  .settings-desc {
    font-size: 0.76rem;
  }
  .settings-group + .settings-group {
    margin-top: 17px;
  }
  .settings-group-title {
    margin-left: 3px;
    font-size: 0.8125rem;
  }
  .settings-list {
    border-radius: 16px;
  }
  .settings-row {
    min-height: 68px;
    padding: 10px 16px;
  }
  .settings-row-desc {
    white-space: normal;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .settings-list {
    background: var(--miuix-surface-container);
  }
}
</style>
