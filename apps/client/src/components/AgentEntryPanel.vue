<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{ desktop?: boolean }>();

const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
</script>

<template>
  <section class="agent-entry-panel" :class="{ desktop }" aria-labelledby="agent-entry-title">
    <div class="agent-entry-head">
      <div class="agent-entry-icon"><v-icon icon="mdi-robot-outline" size="24" /></div>
      <div class="agent-entry-copy">
        <h2 id="agent-entry-title" class="agent-entry-title">{{ t("chat.agentMode") }}</h2>
        <p class="agent-entry-desc">{{ t("chat.agentModeEntryHint") }}</p>
      </div>
      <v-chip size="small" variant="tonal" color="primary">{{ t("chat.comingSoon") }}</v-chip>
    </div>

    <div class="agent-entry-preview" aria-hidden="true">
      <span><v-icon icon="mdi-format-list-checks" size="16" />{{ t("chat.agentPlan") }}</span>
      <span><v-icon icon="mdi-tools" size="16" />{{ t("chat.agentTools") }}</span>
      <span><v-icon icon="mdi-timeline-check-outline" size="16" />{{ t("chat.agentSteps") }}</span>
    </div>

    <button type="button" class="agent-entry-close" @click="emit('close')">
      {{ t("common.close") }}
    </button>
  </section>
</template>

<style scoped>
.agent-entry-panel {
  padding: 20px 18px calc(18px + env(safe-area-inset-bottom, 0px));
  border-radius: 32px 32px 0 0;
  background: var(--miuix-background);
  color: rgb(var(--v-theme-on-surface));
}
.agent-entry-panel.desktop {
  padding: 24px;
  border: 0;
  border-radius: 32px;
  background: var(--miuix-background);
  box-shadow: 0 18px 48px rgb(0 0 0 / 24%);
}
.agent-entry-head {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr) auto;
  align-items: center;
  gap: 12px;
}
.agent-entry-icon {
  width: 44px;
  height: 44px;
  display: grid;
  place-items: center;
  border-radius: 16px;
  color: rgb(var(--v-theme-primary));
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 12%, var(--miuix-surface-container));
}
.agent-entry-copy {
  min-width: 0;
}
.agent-entry-title {
  margin: 0;
  color: rgb(var(--v-theme-on-surface));
  font-size: 1rem;
  font-weight: 720;
  letter-spacing: -0.015em;
}
.agent-entry-desc {
  margin: 3px 0 0;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.76rem;
  line-height: 1.45;
  text-wrap: pretty;
}
.agent-entry-preview {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 7px;
  margin-top: 16px;
}
.agent-entry-preview span {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-width: 0;
  min-height: 38px;
  padding: 0 8px;
  border-radius: 16px;
  color: rgba(var(--v-theme-on-surface), 0.68);
  background: var(--miuix-surface-container-high);
  font-size: 0.7rem;
  font-weight: 620;
}
.agent-entry-close {
  width: 100%;
  height: 40px;
  margin-top: 16px;
  border: 0;
  border-radius: 16px;
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  font: inherit;
  font-size: 0.86rem;
  font-weight: 700;
  cursor: pointer;
  transition: background var(--aerina-spring), box-shadow var(--aerina-spring);
}
.agent-entry-close:hover,
.agent-entry-close:focus-visible {
  outline: none;
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 92%, rgb(var(--v-theme-on-primary)) 8%);
  box-shadow: 0 0 0 2px color-mix(in srgb, rgb(var(--v-theme-primary)) 32%, transparent);
}
.agent-entry-close:active {
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 84%, rgb(var(--v-theme-on-primary)) 16%);
}
@media (max-width: 420px) {
  .agent-entry-head {
    grid-template-columns: 44px minmax(0, 1fr);
  }
  .agent-entry-head .v-chip {
    grid-column: 2;
    justify-self: start;
  }
  .agent-entry-preview {
    grid-template-columns: 1fr;
  }
  .agent-entry-preview span {
    justify-content: flex-start;
  }
}
</style>
