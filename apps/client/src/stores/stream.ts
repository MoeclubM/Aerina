import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { UsageReport } from "../api";

export interface StreamCandidate {
  candidateId: string;
  slotLabel: string;
  modelPresetId?: string;
  modelName?: string;
  status?: "pending" | "streaming" | "completed" | "failed" | "cancelled";
  text: string;
  thinking: string;
  done: boolean;
  promptTokens?: number;
  completionTokens?: number;
  outputTokens?: number;
  totalTokens?: number;
  latencyMs?: number;
  ttftMs?: number;
  reasoningTokens?: number;
  reasoningDurationMs?: number;
  error?: string;
}

export const useStreamStore = defineStore("stream", () => {
  const conversationId = ref<string | null>(null);
  const isStreaming = ref(false);
  const error = ref<string | null>(null);
  const byCandidate = ref<Record<string, StreamCandidate>>({});

  const pendingText = new Map<string, string>();
  const pendingThinking = new Map<string, string>();
  let raf = 0;

  const candidates = computed(() => Object.values(byCandidate.value));
  const combinedText = computed(() =>
    candidates.value.map((c) => c.text).join("\n\n"),
  );

  function ensure(candidateId: string, slotLabel = "?"): StreamCandidate {
    return (
      byCandidate.value[candidateId] ?? {
        candidateId,
        slotLabel,
        text: "",
        thinking: "",
        done: false,
      }
    );
  }

  function flushPending() {
    raf = 0;
    if (!pendingText.size && !pendingThinking.size) return;
    const next = { ...byCandidate.value };
    const ids = new Set([...pendingText.keys(), ...pendingThinking.keys()]);
    for (const candidateId of ids) {
      const cur = next[candidateId] ?? ensure(candidateId);
      next[candidateId] = {
        ...cur,
        text: cur.text + (pendingText.get(candidateId) ?? ""),
        thinking: cur.thinking + (pendingThinking.get(candidateId) ?? ""),
      };
    }
    pendingText.clear();
    pendingThinking.clear();
    byCandidate.value = next;
  }

  function scheduleFlush() {
    if (raf) return;
    raf = requestAnimationFrame(flushPending);
  }

  function begin(id: string) {
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    pendingText.clear();
    pendingThinking.clear();
    conversationId.value = id;
    isStreaming.value = true;
    error.value = null;
    byCandidate.value = {};
  }

  function applyCandidateStatus(
    candidateId: string,
    slotLabel: string,
    modelPresetId: string,
    modelName: string,
    status: "pending" | "streaming" | "completed" | "failed" | "cancelled",
  ) {
    flushPending();
    const cur = ensure(candidateId, slotLabel);
    byCandidate.value = {
      ...byCandidate.value,
      [candidateId]: {
        ...cur,
        slotLabel,
        modelPresetId,
        modelName,
        status,
        done: status === "completed" || status === "failed" || status === "cancelled",
      },
    };
  }

  function streamStart(candidateId: string, slotLabel: string) {
    flushPending();
    const cur = ensure(candidateId, slotLabel);
    byCandidate.value = {
      ...byCandidate.value,
      [candidateId]: {
        ...cur,
        slotLabel,
        done: false,
        status: "streaming",
      },
    };
  }

  function appendDelta(candidateId: string, delta: string) {
    if (!byCandidate.value[candidateId] && !pendingText.has(candidateId)) {
      byCandidate.value = {
        ...byCandidate.value,
        [candidateId]: ensure(candidateId),
      };
    }
    pendingText.set(candidateId, (pendingText.get(candidateId) ?? "") + delta);
    scheduleFlush();
  }

  function appendThinking(candidateId: string, delta: string) {
    if (!byCandidate.value[candidateId] && !pendingThinking.has(candidateId)) {
      byCandidate.value = {
        ...byCandidate.value,
        [candidateId]: ensure(candidateId),
      };
    }
    pendingThinking.set(
      candidateId,
      (pendingThinking.get(candidateId) ?? "") + delta,
    );
    scheduleFlush();
  }

  function applyUsage(candidateId: string, usage: UsageReport) {
    flushPending();
    const cur = ensure(candidateId);
    byCandidate.value = {
      ...byCandidate.value,
      [candidateId]: {
        ...cur,
        promptTokens: usage.prompt_tokens,
        completionTokens: usage.completion_tokens,
        outputTokens: usage.output_tokens,
        totalTokens: usage.total_tokens,
        latencyMs: usage.latency_ms,
        ttftMs: usage.ttft_ms,
        reasoningTokens: usage.reasoning_tokens,
        reasoningDurationMs: usage.reasoning_duration_ms,
      },
    };
  }

  function markDone(candidateId: string) {
    flushPending();
    const cur = byCandidate.value[candidateId];
    if (!cur) return;
    byCandidate.value = {
      ...byCandidate.value,
      [candidateId]: { ...cur, done: true, status: "completed" },
    };
  }

  function markError(candidateId: string, message: string) {
    flushPending();
    const cur = ensure(candidateId);
    const cancelled = message === "cancelled";
    byCandidate.value = {
      ...byCandidate.value,
      [candidateId]: {
        ...cur,
        done: true,
        status: cancelled ? "cancelled" : "failed",
        error: cancelled ? undefined : message,
      },
    };
    if (!cancelled) error.value = message;
  }

  function finish() {
    flushPending();
    isStreaming.value = false;
  }

  function fail(message: string) {
    flushPending();
    isStreaming.value = false;
    error.value = message;
  }

  function clearError() {
    error.value = null;
  }

  function reset() {
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    pendingText.clear();
    pendingThinking.clear();
    conversationId.value = null;
    isStreaming.value = false;
    error.value = null;
    byCandidate.value = {};
  }

  return {
    conversationId,
    isStreaming,
    error,
    byCandidate,
    candidates,
    combinedText,
    begin,
    applyCandidateStatus,
    streamStart,
    appendDelta,
    appendThinking,
    applyUsage,
    markDone,
    markError,
    finish,
    fail,
    clearError,
    reset,
  };
});
