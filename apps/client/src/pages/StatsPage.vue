<script setup lang="ts">
defineOptions({ name: "StatsPage" });

import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, errMessage } from "../api";

const { t } = useI18n();
const loading = ref(false);
const stats = ref<Awaited<ReturnType<typeof api.statsSummary>> | null>(null);
const board = ref<Awaited<ReturnType<typeof api.leaderboard>>>([]);
const presets = ref<Awaited<ReturnType<typeof api.listModelPresets>>>([]);
const backups = ref<Awaited<ReturnType<typeof api.listBackups>>>([]);
const message = ref<string | null>(null);
const error = ref<string | null>(null);

const presetName = computed(() => {
  const map = new Map(presets.value.map((p) => [p.id, p.name]));
  return (id: string) => map.get(id) ?? id.slice(0, 8);
});

const cards = computed(() => [
  { label: t("stats.requests"), value: String(stats.value?.request_count ?? 0) },
  { label: t("stats.completed"), value: String(stats.value?.completed_count ?? 0) },
  { label: t("stats.failed"), value: String(stats.value?.failed_count ?? 0) },
  { label: t("stats.tokens"), value: String(stats.value?.total_tokens ?? 0) },
  {
    label: t("stats.avgLatency"),
    value:
      stats.value?.avg_latency_ms != null
        ? `${Math.round(stats.value.avg_latency_ms)} ms`
        : "-",
  },
  { label: t("stats.votes"), value: String(stats.value?.arena_votes ?? 0) },
  {
    label: t("stats.cost"),
    value: (stats.value?.total_cost_usd ?? 0).toFixed(4),
  },
  { label: t("stats.images"), value: String(stats.value?.image_count ?? 0) },
]);

const ranked = computed(() =>
  [...board.value]
    .map((e) => ({
      ...e,
      name: presetName.value(e.model_preset_id),
      winRate: e.games > 0 ? Math.round((e.wins / e.games) * 100) : 0,
    }))
    .sort((a, b) => b.rating - a.rating || b.games - a.games),
);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    stats.value = await api.statsSummary();
    board.value = await api.leaderboard();
    presets.value = await api.listModelPresets();
    backups.value = await api.listBackups();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    loading.value = false;
  }
}

async function createBackup() {
  error.value = null;
  message.value = null;
  try {
    const info = await api.createBackup();
    message.value = info.name;
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  }
}

async function restore(name: string) {
  error.value = null;
  message.value = null;
  try {
    message.value = await api.restoreBackup(name);
  } catch (e) {
    error.value = errMessage(e);
  }
}

function exportStats(format: "json" | "csv") {
  if (format === "json") {
    const blob = new Blob(
      [JSON.stringify({ summary: stats.value, leaderboard: board.value }, null, 2)],
      { type: "application/json" },
    );
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `aerina-stats-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    return;
  }
  const lines = [
    "model,kind,rating,games,wins",
    ...ranked.value.map(
      (e) =>
        `"${e.name.replace(/"/g, '""')}",${e.arena_kind},${e.rating},${e.games},${e.wins}`,
    ),
  ];
  const blob = new Blob([lines.join("\n")], { type: "text/csv" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `aerina-leaderboard-${Date.now()}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}

onMounted(refresh);
</script>

<template>
  <div class="stats-page">
    <div class="stats-inner">
      <header class="stats-header">
        <div>
          <div class="stats-title">{{ t("stats.title") }}</div>
          <div class="stats-desc">{{ t("stats.desc") }}</div>
        </div>
        <div class="stats-actions">
          <v-btn size="small" variant="text" prepend-icon="mdi-refresh" :loading="loading" @click="refresh">
            {{ t("stats.refresh") }}
          </v-btn>
          <v-btn size="small" variant="tonal" @click="exportStats('json')">{{ t("stats.exportJson") }}</v-btn>
          <v-btn size="small" variant="tonal" @click="exportStats('csv')">{{ t("stats.exportCsv") }}</v-btn>
          <v-btn size="small" color="primary" @click="createBackup">{{ t("stats.createBackup") }}</v-btn>
        </div>
      </header>

      <v-alert v-if="error" type="error" variant="tonal" density="compact" class="mb-4" :text="error" />
      <v-alert v-if="message" type="info" variant="tonal" density="compact" class="mb-4" :text="message" />

      <section class="stats-section">
        <div class="stats-section-title">{{ t("stats.summary") }}</div>
        <div class="stats-grid">
          <div v-for="card in cards" :key="card.label" class="stats-metric">
            <div class="stats-metric-label">{{ card.label }}</div>
            <div class="stats-metric-value">{{ card.value }}</div>
          </div>
        </div>
      </section>

      <section class="stats-section">
        <div class="stats-section-title">{{ t("stats.leaderboard") }}</div>
        <div class="stats-panel">
          <div v-if="!ranked.length" class="stats-empty">{{ t("stats.emptyBoard") }}</div>
          <table v-else class="stats-table">
            <thead>
              <tr>
                <th class="col-rank">#</th>
                <th>{{ t("stats.model") }}</th>
                <th>{{ t("stats.kind") }}</th>
                <th class="num">{{ t("stats.rating") }}</th>
                <th class="num">{{ t("stats.games") }}</th>
                <th class="num">{{ t("stats.wins") }}</th>
                <th class="num">{{ t("stats.winRate") }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, idx) in ranked" :key="row.model_preset_id + row.arena_kind + (row.category || '')">
                <td class="col-rank">{{ idx + 1 }}</td>
                <td class="model">{{ row.name }}</td>
                <td>{{ row.arena_kind }}</td>
                <td class="num strong">{{ Math.round(row.rating) }}</td>
                <td class="num">{{ row.games }}</td>
                <td class="num">{{ row.wins }}</td>
                <td class="num">{{ row.winRate }}%</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section class="stats-section">
        <div class="stats-section-title">{{ t("stats.backups") }}</div>
        <div class="stats-panel">
          <div v-if="!backups.length" class="stats-empty">{{ t("stats.emptyBackups") }}</div>
          <div v-else class="backup-list">
            <div v-for="b in backups" :key="b.name" class="backup-row">
              <div class="backup-meta">
                <div class="backup-name">{{ b.name }}</div>
                <div class="backup-path">{{ b.path }}</div>
              </div>
              <v-btn size="small" variant="tonal" @click="restore(b.name)">{{ t("stats.restore") }}</v-btn>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
