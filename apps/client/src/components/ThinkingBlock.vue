<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
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
const liveDurationMs = ref(0);
let liveStartedAt = 0;
let liveTimer: ReturnType<typeof setInterval> | undefined;

watch(
  () => props.streaming,
  (streaming) => {
    if (streaming) {
      open.value = true;
      liveStartedAt = performance.now();
      liveDurationMs.value = 0;
      liveTimer = setInterval(() => {
        liveDurationMs.value = performance.now() - liveStartedAt;
      }, 100);
    } else if (liveTimer) {
      clearInterval(liveTimer);
      liveTimer = undefined;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  if (liveTimer) clearInterval(liveTimer);
});

function toggle() {
  open.value = !open.value;
}

const visible = computed(() =>
  Boolean(
    props.text.trim() ||
    props.streaming ||
    (props.tokens != null && props.tokens > 0) ||
    (props.durationMs != null && props.durationMs > 1),
  ),
);

const metaText = computed(() => {
  const parts: string[] = [];
  let tokens = props.tokens;
  let estimated = false;
  if ((tokens == null || tokens <= 0) && props.text.trim()) {
    const cjkCount = (props.text.match(/[\u3400-\u9fff\uf900-\ufaff]/g) || []).length;
    const otherLength = props.text.replace(/[\u3400-\u9fff\uf900-\ufaff]/g, "").trim().length;
    tokens = Math.max(1, cjkCount + Math.ceil(otherLength / 4));
    estimated = true;
  }
  if (tokens != null) {
    const value = tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : String(tokens);
    parts.push(`${estimated ? "≈" : ""}${value} tok`);
  }
  const measuredDuration = props.durationMs != null && props.durationMs > 1
    ? props.durationMs
    : undefined;
  const durationMs = measuredDuration ?? (props.streaming ? liveDurationMs.value : undefined);
  if (durationMs != null) {
    parts.push(durationMs >= 1000 ? `${(durationMs / 1000).toFixed(1)}s` : `${Math.round(durationMs)}ms`);
  }
  return parts.join(" · ");
});
</script>

<template>
  <div v-if="visible" class="thinking">
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
  max-width: 100%;
  min-width: 0;
  gap: 6px;
  margin: 0;
  min-height: 40px;
  padding: 0 12px 0 8px;
  border: 0;
  border-radius: 12px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 600;
  letter-spacing: 0;
  cursor: pointer;
  transition: background var(--aerina-spring), color var(--aerina-spring);
  -webkit-tap-highlight-color: transparent;
}

.thinking-toggle:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.78);
}

.thinking-toggle:active {
  background: color-mix(in srgb, var(--miuix-surface-container) 90%, var(--miuix-on-container) 10%);
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
  min-width: 0;
  overflow: hidden;
  color: rgba(var(--v-theme-on-surface), 0.42);
  font-size: 0.72rem;
  font-weight: 500;
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  padding: 12px 14px;
  border: 0;
  border-radius: 16px;
  background: var(--miuix-surface-container-high);
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
  overflow-x: hidden;
  overflow-y: auto;
}

@media (prefers-reduced-motion: reduce) {
  .thinking-live {
    animation: none;
  }
}
</style>
