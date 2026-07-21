<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(
  defineProps<{
    text?: string;
    streaming?: boolean;
    defaultOpen?: boolean;
    tokens?: number;
    durationMs?: number;
  }>(),
  {
    text: "",
    streaming: false,
    defaultOpen: false,
  },
);

const { t } = useI18n();
const open = ref(props.defaultOpen || props.streaming);

watch(
  () => props.streaming,
  (streaming) => {
    if (streaming) open.value = true;
  },
);

function toggle() {
  open.value = !open.value;
}

const metaText = computed(() => {
  const parts: string[] = [];
  if (props.tokens != null) {
    parts.push(`${props.tokens >= 1000 ? (props.tokens / 1000).toFixed(1) + "k" : props.tokens} tok`);
  }
  if (props.durationMs != null) {
    parts.push(props.durationMs >= 1000 ? (props.durationMs / 1000).toFixed(1) + "s" : Math.round(props.durationMs) + "ms");
  }
  return parts.join(" · ");
});
</script>

<template>
  <div v-if="text || streaming" class="thinking">
    <button type="button" class="thinking-toggle" @click="toggle">
      <v-icon
        :icon="open ? 'mdi-chevron-down' : 'mdi-chevron-right'"
        size="16"
        class="thinking-chevron"
      />
      <v-icon icon="mdi-creation-outline" size="15" class="thinking-spark" />
      <span class="thinking-label">{{ t("chat.thinking") }}</span>
      <span v-if="metaText" class="thinking-meta">· {{ metaText }}</span>
      <span v-if="streaming" class="thinking-live" aria-hidden="true" />
    </button>
    <div v-show="open" class="thinking-panel">
      <pre class="thinking-body">{{ text || "…" }}</pre>
    </div>
  </div>
</template>

<style scoped>
.thinking {
  margin: 0 0 10px;
  max-width: 100%;
}

.thinking-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  padding: 4px 8px 4px 4px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, transform 100ms ease-out;
  -webkit-tap-highlight-color: transparent;
}

.thinking-toggle:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.78);
}

.thinking-toggle:active {
  transform: scale(0.98);
}

.thinking-toggle:focus {
  outline: none;
}

.thinking-toggle:focus-visible {
  box-shadow: 0 0 0 2px rgba(var(--v-theme-primary), 0.35);
}

.thinking-chevron {
  opacity: 0.7;
  flex: 0 0 auto;
}

.thinking-spark {
  opacity: 0.85;
  color: rgb(var(--v-theme-primary));
  flex: 0 0 auto;
}

.thinking-label {
  line-height: 1.2;
}

.thinking-meta {
  font-size: 0.72rem;
  font-weight: 500;
  color: rgba(var(--v-theme-on-surface), 0.42);
  font-variant-numeric: tabular-nums;
}

.thinking-live {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgb(var(--v-theme-primary));
  margin-left: 2px;
  animation: thinking-pulse 1.1s ease-in-out infinite;
}

@keyframes thinking-pulse {
  0%,
  100% {
    opacity: 0.35;
    transform: scale(0.9);
  }
  50% {
    opacity: 1;
    transform: scale(1);
  }
}

.thinking-panel {
  margin-top: 4px;
  margin-left: 4px;
  padding: 8px 12px 10px;
  border-left: 2px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 0 10px 10px 0;
  background: rgba(var(--v-theme-on-surface), 0.03);
}

.thinking-body {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: ui-monospace, "SF Mono", SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.78rem;
  line-height: 1.55;
  color: rgba(var(--v-theme-on-surface), 0.62);
  max-height: 240px;
  overflow: auto;
}
</style>
