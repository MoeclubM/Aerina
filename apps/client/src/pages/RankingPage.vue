<script setup lang="ts">
defineOptions({ name: "RankingPage" });

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, errMessage } from "../api";

const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true });
const { t } = useI18n();
const router = useRouter();
const windowWidth = ref(window.innerWidth);
const mobile = computed(() => windowWidth.value < 960);
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
    board.value = await api.leaderboard();
    presets.value = await api.listModelPresets();
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

function handleResize() { windowWidth.value = window.innerWidth; }

onMounted(() => {
  window.addEventListener("resize", handleResize);
  refresh();
});
onUnmounted(() => { window.removeEventListener("resize", handleResize); });
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
        <div class="ranking-header-left">
          <button v-if="mobile" type="button" class="ranking-back-btn" @click="router.push('/')">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 18l-6-6 6-6"/></svg>
          </button>
          <div>
            <h1 class="ranking-title">{{ t("ranking.title") }}</h1>
            <p class="ranking-desc">{{ t("ranking.desc") }}</p>
          </div>
        </div>
        <div class="ranking-actions">
          <button type="button" class="btn-ghost" :disabled="loading" @click="refresh">
            {{ t("stats.refresh") }}
          </button>
          <button type="button" class="btn-secondary" :disabled="!ranked.length" @click="exportCsv">
            {{ t("stats.exportCsv") }}
          </button>
        </div>
      </header>

      <div v-if="error" class="banner error">{{ error }}</div>

      <section class="surface">
        <div v-if="!ranked.length" class="empty">{{ t("stats.emptyBoard") }}</div>
        <table v-else class="rank-table">
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
  max-width: 920px;
  margin: 0 auto;
  padding: 28px 20px 48px;
}
.ranking-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}
.ranking-header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ranking-back-btn {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 0;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgb(var(--v-theme-on-surface));
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.12s ease, transform 100ms ease-out;
}
.ranking-back-btn:active {
  transform: scale(0.92);
  background: rgba(var(--v-theme-on-surface), 0.12);
}
.ranking-title {
  margin: 0 0 6px;
  font-size: 1.45rem;
  font-weight: 650;
  letter-spacing: -0.02em;
}
.ranking-desc {
  margin: 0;
  font-size: 0.88rem;
  color: rgba(var(--v-theme-on-surface), 0.56);
  max-width: 46ch;
  line-height: 1.45;
}
.ranking-actions {
  display: flex;
  gap: 8px;
}
.btn-secondary,
.btn-ghost {
  border: 0;
  cursor: pointer;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  border-radius: 10px;
  height: 34px;
  padding: 0 12px;
  transition: transform 100ms ease-out, opacity 0.12s ease;
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
  background: rgba(var(--v-theme-primary), 0.14);
  color: rgb(var(--v-theme-primary));
}
.btn-ghost {
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.75);
}
.banner.error {
  border-radius: 12px;
  padding: 10px 12px;
  margin-bottom: 12px;
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
  font-size: 0.86rem;
}
.surface {
  border-radius: 18px;
  border: 1px solid rgba(var(--v-border-color), 0.4);
  background: color-mix(in srgb, rgb(var(--v-theme-surface)) 88%, transparent);
  backdrop-filter: blur(18px) saturate(160%);
  -webkit-backdrop-filter: blur(18px) saturate(160%);
  padding: 8px 4px;
  overflow: auto;
}
.empty {
  padding: 28px 18px;
  text-align: center;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.9rem;
}
.rank-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.88rem;
}
.rank-table th,
.rank-table td {
  padding: 10px 12px;
  text-align: left;
  border-bottom: 1px solid rgba(var(--v-border-color), 0.28);
}
.rank-table th {
  font-size: 0.74rem;
  font-weight: 620;
  color: rgba(var(--v-theme-on-surface), 0.55);
  letter-spacing: 0.01em;
}
.rank-table tbody tr:last-child td {
  border-bottom: 0;
}
.rank-table tbody tr:hover td {
  background: rgba(var(--v-theme-on-surface), 0.03);
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
  font-weight: 600;
}
.kind {
  color: rgba(var(--v-theme-on-surface), 0.62);
  text-transform: lowercase;
}
.rank-badge {
  display: inline-grid;
  place-items: center;
  min-width: 24px;
  height: 24px;
  border-radius: 8px;
  background: rgba(var(--v-theme-on-surface), 0.08);
  font-weight: 700;
  font-size: 0.78rem;
}
.rank-badge.top {
  background: rgba(var(--v-theme-primary), 0.16);
  color: rgb(var(--v-theme-primary));
}

@media (prefers-reduced-transparency: reduce) {
  .surface {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: rgb(var(--v-theme-surface));
  }
}
</style>