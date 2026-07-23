<script setup lang="ts">
defineOptions({ name: "RankingPage" });

import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { api, errMessage } from "../api";

const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true });
const { t } = useI18n();
const loading = ref(false);
const error = ref<string | null>(null);
const board = ref<Awaited<ReturnType<typeof api.leaderboard>>>([]);
const presets = ref<Awaited<ReturnType<typeof api.listModelPresets>>>([]);

const presetName = computed(() => {
  const map = new Map(presets.value.map((p) => [p.id, p.name]));
  return (id: string) => map.get(id) ?? id.slice(0, 8);
});

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
    const [nextBoard, nextPresets] = await Promise.all([
      api.leaderboard(),
      api.listModelPresets(),
    ]);
    board.value = nextBoard;
    presets.value = nextPresets;
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    loading.value = false;
  }
}

function exportCsv() {
  const lines = [
    "model,kind,rating,games,wins,win_rate",
    ...ranked.value.map(
      (e) =>
        `"${e.name.replace(/"/g, '""')}",${e.arena_kind},${e.rating},${e.games},${e.wins},${e.winRate}`,
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
watch(
  () => props.active,
  (v) => {
    if (v) void refresh();
  },
);
</script>

<template>
  <div class="ranking-page">
    <div class="ranking-inner">
      <header class="ranking-header">
        <div class="ranking-heading">
          <h1 class="ranking-title">{{ t("ranking.title") }}</h1>
          <p class="ranking-desc">{{ t("ranking.desc") }}</p>
        </div>
        <div class="ranking-actions">
          <button type="button" class="btn-ghost" :disabled="loading" @click="refresh">
            <v-icon icon="mdi-refresh" size="16" />
            {{ t("stats.refresh") }}
          </button>
          <button type="button" class="btn-secondary" :disabled="!ranked.length" @click="exportCsv">
            <v-icon icon="mdi-download-outline" size="16" />
            {{ t("stats.exportCsv") }}
          </button>
        </div>
      </header>

      <div v-if="error && ranked.length" class="banner error">{{ error }}</div>

      <section class="surface">
        <div v-if="loading && !ranked.length" class="rank-state">
          <v-progress-circular indeterminate color="primary" size="22" width="2" />
          <span>{{ t("common.loading") }}</span>
        </div>
        <div v-else-if="error && !ranked.length" class="rank-state error-state">
          <v-icon icon="mdi-alert-circle-outline" size="22" />
          <span>{{ error }}</span>
          <button type="button" class="btn-secondary" @click="refresh">{{ t("common.retry") }}</button>
        </div>
        <div v-else-if="!ranked.length" class="rank-state">{{ t("stats.emptyBoard") }}</div>
        <template v-else>
          <table class="rank-table">
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
              <tr
                v-for="(row, idx) in ranked"
                :key="row.model_preset_id + row.arena_kind + (row.category || '')"
              >
                <td class="col-rank">
                  <span class="rank-badge" :class="{ top: idx < 3 }">{{ idx + 1 }}</span>
                </td>
                <td class="model">{{ row.name }}</td>
                <td class="kind">{{ row.arena_kind }}</td>
                <td class="num strong">{{ Math.round(row.rating) }}</td>
                <td class="num">{{ row.games }}</td>
                <td class="num">{{ row.wins }}</td>
                <td class="num">{{ row.winRate }}%</td>
              </tr>
            </tbody>
          </table>

          <div class="mobile-rank-list">
            <div
              v-for="(row, idx) in ranked"
              :key="`mobile-${row.model_preset_id}-${row.arena_kind}-${row.category || ''}`"
              class="mobile-rank-row"
            >
              <span class="rank-badge" :class="{ top: idx < 3 }">{{ idx + 1 }}</span>
              <div class="mobile-rank-main">
                <div class="mobile-rank-primary">
                  <span class="model">{{ row.name }}</span>
                  <span class="mobile-rank-score">{{ Math.round(row.rating) }}</span>
                </div>
                <div class="mobile-rank-meta">
                  <span>{{ row.arena_kind }}</span>
                  <span>{{ row.games }} {{ t("stats.games") }}</span>
                  <span>{{ row.wins }} {{ t("stats.wins") }}</span>
                  <span class="mobile-rank-rate">{{ row.winRate }}%</span>
                </div>
              </div>
            </div>
          </div>
        </template>
      </section>
    </div>
  </div>
</template>

<style scoped>
.ranking-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.ranking-inner {
  max-width: 960px;
  margin: 0 auto;
  padding: 28px 20px 48px;
}
.ranking-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}
.ranking-heading {
  min-width: 0;
}
.ranking-title {
  margin: 0 0 5px;
  font-size: 1.42rem;
  font-weight: 680;
  letter-spacing: -0.025em;
}
.ranking-desc {
  margin: 0;
  font-size: 0.84rem;
  color: rgba(var(--v-theme-on-surface), 0.56);
  max-width: 48ch;
  line-height: 1.45;
}
.ranking-actions {
  display: flex;
  flex-shrink: 0;
  gap: 6px;
}
.btn-secondary,
.btn-ghost {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: 0;
  cursor: pointer;
  font: inherit;
  font-size: 0.8rem;
  font-weight: 620;
  border-radius: 9px;
  height: 34px;
  padding: 0 11px;
  transition: background 140ms ease, transform 100ms ease-out, opacity 120ms ease;
}
.btn-secondary:active,
.btn-ghost:active {
  transform: scale(0.97);
}
.btn-secondary:disabled,
.btn-ghost:disabled {
  opacity: 0.45;
  cursor: default;
}
.btn-secondary {
  background: rgba(var(--v-theme-primary), 0.12);
  color: rgb(var(--v-theme-primary));
}
.btn-secondary:hover:not(:disabled) {
  background: rgba(var(--v-theme-primary), 0.18);
}
.btn-ghost {
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.7);
}
.btn-ghost:hover:not(:disabled) {
  background: rgba(var(--v-theme-on-surface), 0.06);
}
.banner.error {
  border-radius: 10px;
  padding: 9px 11px;
  margin-bottom: 10px;
  background: rgba(var(--v-theme-error), 0.1);
  color: rgb(var(--v-theme-error));
  font-size: 0.8rem;
}
.surface {
  border-radius: 16px;
  border: 1px solid rgba(var(--v-border-color), 0.38);
  background: color-mix(in srgb, rgb(var(--v-theme-surface)) 92%, transparent);
  overflow: hidden;
}
.rank-state {
  min-height: 132px;
  padding: 22px 18px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  text-align: center;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.86rem;
}
.error-state {
  flex-direction: column;
  color: rgb(var(--v-theme-error));
}
.rank-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.84rem;
}
.rank-table th,
.rank-table td {
  padding: 10px 12px;
  text-align: left;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.25);
}
.rank-table th {
  font-size: 0.7rem;
  font-weight: 650;
  color: rgba(var(--v-theme-on-surface), 0.52);
  letter-spacing: 0.015em;
}
.rank-table tbody tr:last-child td {
  border-bottom: 0;
}
.rank-table tbody tr:hover td {
  background: rgba(var(--v-theme-on-surface), 0.025);
}
.col-rank {
  width: 48px;
}
.num {
  text-align: right !important;
  font-variant-numeric: tabular-nums;
}
.strong {
  font-weight: 700;
  color: rgb(var(--v-theme-primary));
}
.model {
  font-weight: 620;
}
.kind {
  color: rgba(var(--v-theme-on-surface), 0.58);
  text-transform: lowercase;
}
.rank-badge {
  display: inline-grid;
  place-items: center;
  min-width: 24px;
  height: 24px;
  padding: 0 5px;
  border-radius: 8px;
  background: rgba(var(--v-theme-on-surface), 0.07);
  font-weight: 700;
  font-size: 0.74rem;
  font-variant-numeric: tabular-nums;
}
.rank-badge.top {
  background: linear-gradient(135deg, rgba(var(--v-theme-primary), 0.17), rgba(var(--v-theme-tertiary), 0.13));
  color: rgb(var(--v-theme-primary));
}
.mobile-rank-list {
  display: none;
}

@media (max-width: 679px) {
  .ranking-inner {
    width: 100%;
    padding: 18px 12px 28px;
    box-sizing: border-box;
  }
  .ranking-header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .ranking-title {
    display: none;
  }
  .ranking-desc {
    max-width: 24ch;
    font-size: 0.75rem;
    line-height: 1.35;
  }
  .ranking-actions {
    margin-left: auto;
  }
  .btn-secondary,
  .btn-ghost {
    width: 34px;
    padding: 0;
    font-size: 0;
  }
  .btn-secondary :deep(.v-icon),
  .btn-ghost :deep(.v-icon) {
    font-size: 17px !important;
  }
  .surface {
    width: 100%;
    min-width: 0;
    border-radius: 14px;
  }
  .rank-table {
    display: none;
  }
  .mobile-rank-list {
    display: block;
  }
  .mobile-rank-row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 62px;
    padding: 9px 12px;
    border-bottom: 1px solid rgba(var(--v-border-color), 0.24);
  }
  .mobile-rank-row:last-child {
    border-bottom: 0;
  }
  .mobile-rank-main {
    flex: 1;
    min-width: 0;
  }
  .mobile-rank-primary {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .mobile-rank-primary .model {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.84rem;
  }
  .mobile-rank-score {
    color: rgb(var(--v-theme-primary));
    font-size: 0.86rem;
    font-weight: 720;
    font-variant-numeric: tabular-nums;
  }
  .mobile-rank-meta {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    align-items: center;
    gap: 7px;
    min-width: 0;
    margin-top: 4px;
    color: rgba(var(--v-theme-on-surface), 0.48);
    font-size: 0.65rem;
    white-space: nowrap;
  }
  .mobile-rank-meta > span:first-child {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .mobile-rank-meta span:not(:last-child)::after {
    content: "·";
    margin-left: 7px;
    color: rgba(var(--v-theme-on-surface), 0.24);
  }
  .mobile-rank-rate {
    color: rgba(var(--v-theme-on-surface), 0.7);
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
}

@media (max-width: 420px) {
  .mobile-rank-row {
    min-height: 68px;
  }
  .mobile-rank-meta {
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 10px;
    row-gap: 2px;
    white-space: normal;
  }
  .mobile-rank-meta > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mobile-rank-meta > span:nth-child(even) {
    justify-self: end;
  }
  .mobile-rank-meta span::after {
    display: none;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .surface {
    background: rgb(var(--v-theme-surface));
  }
}
</style>
