<script setup lang="ts">
defineOptions({ name: "ProfilePage" });

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import {
  api,
  errMessage,
  type ActivityDay,
  type SessionInfo,
  type StatsSummary,
} from "../api";

const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true });
const { t, locale } = useI18n();
const router = useRouter();

const loading = ref(false);
const error = ref<string | null>(null);
const message = ref<string | null>(null);
const session = ref<SessionInfo | null>(null);
const stats = ref<StatsSummary | null>(null);
const activity = ref<ActivityDay[]>([]);
const backups = ref<Awaited<ReturnType<typeof api.listBackups>>>([]);
const avatarUrl = ref<string | null>(null);

const profile = computed(() => session.value?.profile ?? null);
const initial = computed(() => {
  const name = profile.value?.display_name?.trim() || "?";
  return name.slice(0, 1).toUpperCase();
});

const cards = computed(() => [
  { label: t("stats.requests"), value: String(stats.value?.request_count ?? 0) },
  { label: t("stats.completed"), value: String(stats.value?.completed_count ?? 0) },
  { label: t("stats.tokens"), value: String(stats.value?.total_tokens ?? 0) },
  {
    label: t("stats.avgLatency"),
    value:
      stats.value?.avg_latency_ms != null
        ? `${Math.round(stats.value.avg_latency_ms)} ms`
        : "-",
  },
  {
    label: t("stats.cost"),
    value: (stats.value?.total_cost_usd ?? 0).toFixed(4),
  },
  { label: t("stats.votes"), value: String(stats.value?.arena_votes ?? 0) },
]);

const totalActiveDays = computed(
  () => activity.value.filter((d) => d.count > 0).length,
);
const totalActivity = computed(() =>
  activity.value.reduce((sum, d) => sum + d.count, 0),
);

/** GitHub-style weeks × weekdays grid for last ~53 weeks */
const heatmap = computed(() => {
  const byDate = new Map(activity.value.map((d) => [d.date, d.count]));
  if (!activity.value.length) {
    return { weeks: [] as Array<Array<{ date: string; count: number; level: number }>>, monthLabels: [] as string[] };
  }
  const end = new Date(activity.value[activity.value.length - 1].date + "T00:00:00");
  const start = new Date(activity.value[0].date + "T00:00:00");

  // Align start to Sunday (GitHub uses Sunday-first)
  const gridStart = new Date(start);
  gridStart.setDate(gridStart.getDate() - gridStart.getDay());

  const weeks: Array<Array<{ date: string; count: number; level: number }>> = [];
  const cursor = new Date(gridStart);
  while (cursor <= end || weeks.length < 53) {
    const week: Array<{ date: string; count: number; level: number }> = [];
    for (let i = 0; i < 7; i++) {
      const key = formatDate(cursor);
      const count = byDate.get(key) ?? 0;
      const inRange = cursor >= start && cursor <= end;
      week.push({
        date: key,
        count: inRange ? count : 0,
        level: inRange ? levelOf(count) : -1,
      });
      cursor.setDate(cursor.getDate() + 1);
    }
    weeks.push(week);
    if (cursor > end && weeks.length >= 52) break;
    if (weeks.length >= 53) break;
  }

  const monthLabels: string[] = [];
  let lastMonth = -1;
  for (const week of weeks) {
    const mid = week[0];
    const d = new Date(mid.date + "T00:00:00");
    const m = d.getMonth();
    if (m !== lastMonth) {
      monthLabels.push(monthName(m));
      lastMonth = m;
    } else {
      monthLabels.push("");
    }
  }
  return { weeks, monthLabels };
});

const weekdayLabels = computed(() => {
  const mon = locale.value.startsWith("zh") ? ["日", "一", "二", "三", "四", "五", "六"] : ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  return mon;
});

function formatDate(d: Date) {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function monthName(m: number) {
  if (locale.value.startsWith("zh")) {
    return `${m + 1}月`;
  }
  return ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][m];
}

function levelOf(count: number) {
  if (count <= 0) return 0;
  if (count === 1) return 1;
  if (count <= 3) return 2;
  if (count <= 6) return 3;
  return 4;
}

function cellTitle(cell: { date: string; count: number; level: number }) {
  if (cell.level < 0) return "";
  return t("profile.heatmapCell", { date: cell.date, n: cell.count });
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    session.value = await api.sessionInfo();
    const [s, heat, b] = await Promise.all([
      api.statsSummary(),
      api.activityHeatmap(365),
      api.listBackups(),
    ]);
    stats.value = s;
    activity.value = heat;
    backups.value = b;
    if (session.value.profile.avatar_path) {
      avatarUrl.value = await api.getProfileAvatarDataUrl(session.value.profile.id);
    } else {
      avatarUrl.value = null;
    }
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
    backups.value = await api.listBackups();
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

function exportStats() {
  const blob = new Blob(
    [JSON.stringify({ summary: stats.value, activity: activity.value }, null, 2)],
    { type: "application/json" },
  );
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `aerina-stats-${Date.now()}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function onSessionChanged() {
  void refresh();
}

onMounted(() => {
  window.addEventListener("aerina:session-changed", onSessionChanged);
  void refresh();
});
onUnmounted(() => {
  window.removeEventListener("aerina:session-changed", onSessionChanged);
});
watch(
  () => props.active,
  (v) => {
    if (v) void refresh();
  },
);
</script>

<template>
  <div class="profile-page">
    <div class="profile-inner">
      <header class="profile-hero">
        <div class="profile-identity">
          <div class="profile-avatar-lg">
            <img v-if="avatarUrl" :src="avatarUrl" alt="" />
            <span v-else>{{ initial }}</span>
          </div>
          <div class="profile-id-text">
            <h1 class="profile-name">{{ profile?.display_name || t("profile.current") }}</h1>
            <p class="profile-meta">
              {{ t("profile.localHintShort") }}

            </p>
            <div class="profile-links">
              <button type="button" class="link-btn" @click="router.push('/settings/appearance')">
                {{ t("profile.edit") }}
              </button>
            </div>
          </div>
        </div>
        <div class="profile-hero-actions">
          <button type="button" class="btn-ghost" :disabled="loading" @click="refresh">
            {{ t("stats.refresh") }}
          </button>
          <button type="button" class="btn-secondary" @click="exportStats">
            {{ t("stats.exportJson") }}
          </button>
          <button type="button" class="btn-primary" @click="createBackup">
            {{ t("stats.createBackup") }}
          </button>
        </div>
      </header>

      <div v-if="error" class="banner error">{{ error }}</div>
      <div v-if="message" class="banner info">{{ message }}</div>

      <section class="surface">
        <div class="surface-head">
          <div>
            <div class="surface-title">{{ t("profile.activity") }}</div>
            <div class="surface-desc">
              {{ t("profile.activitySummary", { days: totalActiveDays, n: totalActivity }) }}
            </div>
          </div>
        </div>

        <div class="heatmap-wrap">
          <div class="heatmap-months">
            <span
              v-for="(label, i) in heatmap.monthLabels"
              :key="i"
              class="heatmap-month"
            >{{ label }}</span>
          </div>
          <div class="heatmap-body">
            <div class="heatmap-weekdays">
              <span
                v-for="(w, i) in weekdayLabels"
                :key="w"
                class="heatmap-weekday"
                :class="{ dim: i % 2 === 1 }"
              >{{ i % 2 === 1 ? w : "" }}</span>
            </div>
            <div class="heatmap-grid">
              <div v-for="(week, wi) in heatmap.weeks" :key="wi" class="heatmap-week">
                <div
                  v-for="cell in week"
                  :key="cell.date + cell.level"
                  class="heatmap-cell"
                  :class="cell.level < 0 ? 'out' : `lv-${cell.level}`"
                  :title="cellTitle(cell)"
                />
              </div>
            </div>
          </div>
          <div class="heatmap-legend">
            <span>{{ t("profile.less") }}</span>
            <i class="heatmap-cell lv-0" />
            <i class="heatmap-cell lv-1" />
            <i class="heatmap-cell lv-2" />
            <i class="heatmap-cell lv-3" />
            <i class="heatmap-cell lv-4" />
            <span>{{ t("profile.more") }}</span>
          </div>
        </div>
      </section>

      <section class="surface">
        <div class="surface-title mb">{{ t("stats.summary") }}</div>
        <div class="metric-grid">
          <div v-for="card in cards" :key="card.label" class="metric">
            <div class="metric-label">{{ card.label }}</div>
            <div class="metric-value">{{ card.value }}</div>
          </div>
        </div>
      </section>

      <section class="surface">
        <div class="surface-title mb">{{ t("stats.backups") }}</div>
        <div v-if="!backups.length" class="empty">{{ t("stats.emptyBackups") }}</div>
        <div v-else class="backup-list">
          <div v-for="b in backups" :key="b.name" class="backup-row">
            <div class="backup-meta">
              <div class="backup-name">{{ b.name }}</div>
              <div class="backup-path">{{ b.path }}</div>
            </div>
            <button type="button" class="btn-secondary" @click="restore(b.name)">
              {{ t("stats.restore") }}
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.profile-page {
  height: 100%;
  overflow: auto;
  background: rgb(var(--v-theme-background));
}
.profile-inner {
  max-width: 920px;
  margin: 0 auto;
  padding: 28px 20px 48px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.profile-hero {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.profile-identity {
  display: flex;
  gap: 14px;
  align-items: center;
  min-width: 0;
}
.profile-avatar-lg {
  width: 72px;
  height: 72px;
  border-radius: 20px;
  overflow: hidden;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-primary), 0.16);
  color: rgb(var(--v-theme-primary));
  font-size: 1.6rem;
  font-weight: 700;
  flex: 0 0 auto;
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-on-surface), 0.06);
}
.profile-avatar-lg img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center;
}
.profile-name {
  margin: 0 0 4px;
  font-size: 1.45rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.15;
}
.profile-meta {
  margin: 0;
  font-size: 0.84rem;
  color: rgba(var(--v-theme-on-surface), 0.56);
  line-height: 1.4;
}
.profile-links {
  display: flex;
  gap: 12px;
  margin-top: 8px;
}
.link-btn {
  border: 0;
  background: transparent;
  color: rgb(var(--v-theme-primary));
  font: inherit;
  font-size: 0.84rem;
  font-weight: 600;
  padding: 0;
  cursor: pointer;
}
.profile-hero-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.btn-primary,
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
.btn-primary:active,
.btn-secondary:active,
.btn-ghost:active {
  transform: scale(0.97);
}
.btn-primary {
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
}
.btn-secondary {
  background: rgba(var(--v-theme-primary), 0.14);
  color: rgb(var(--v-theme-primary));
}
.btn-ghost {
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.75);
}
.btn-ghost:disabled {
  opacity: 0.45;
}

.banner {
  border-radius: 12px;
  padding: 10px 12px;
  font-size: 0.86rem;
}
.banner.error {
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
}
.banner.info {
  background: rgba(var(--v-theme-info), 0.12);
  color: rgb(var(--v-theme-info));
}

.surface {
  border-radius: 18px;
  border: 1px solid rgba(var(--v-border-color), 0.4);
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  backdrop-filter: blur(18px) saturate(160%);
  -webkit-backdrop-filter: blur(18px) saturate(160%);
  padding: 16px 18px 18px;
}
.surface-head {
  margin-bottom: 14px;
}
.surface-title {
  font-size: 0.95rem;
  font-weight: 650;
  letter-spacing: -0.01em;
}
.surface-title.mb {
  margin-bottom: 12px;
}
.surface-desc {
  margin-top: 4px;
  font-size: 0.8rem;
  color: rgba(var(--v-theme-on-surface), 0.55);
}

.heatmap-wrap {
  overflow-x: auto;
  padding-bottom: 4px;
}
.heatmap-months {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 12px;
  gap: 3px;
  margin-left: 28px;
  margin-bottom: 4px;
  min-width: max-content;
}
.heatmap-month {
  font-size: 0.68rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
  height: 14px;
  overflow: visible;
  white-space: nowrap;
}
.heatmap-body {
  display: flex;
  gap: 6px;
  min-width: max-content;
}
.heatmap-weekdays {
  display: grid;
  grid-template-rows: repeat(7, 10px);
  gap: 3px;
  width: 22px;
  flex: 0 0 auto;
}
.heatmap-weekday {
  font-size: 0.62rem;
  line-height: 10px;
  color: rgba(var(--v-theme-on-surface), 0.5);
}
.heatmap-grid {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: 10px;
  gap: 3px;
}
.heatmap-week {
  display: grid;
  grid-template-rows: repeat(7, 10px);
  gap: 3px;
}
.heatmap-cell {
  width: 10px;
  height: 10px;
  border-radius: 2px;
  display: block;
  box-shadow: inset 0 0 0 1px rgba(var(--v-theme-on-surface), 0.04);
}
.heatmap-cell.out {
  background: transparent;
  box-shadow: none;
}
.heatmap-cell.lv-0 {
  background: rgba(var(--v-theme-on-surface), 0.08);
}
.heatmap-cell.lv-1 {
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 35%, transparent);
}
.heatmap-cell.lv-2 {
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 55%, transparent);
}
.heatmap-cell.lv-3 {
  background: color-mix(in srgb, rgb(var(--v-theme-primary)) 75%, transparent);
}
.heatmap-cell.lv-4 {
  background: rgb(var(--v-theme-primary));
}
.heatmap-legend {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  margin-top: 10px;
  font-size: 0.72rem;
  color: rgba(var(--v-theme-on-surface), 0.55);
}
.heatmap-legend .heatmap-cell {
  width: 10px;
  height: 10px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
  gap: 10px;
}
.metric {
  border-radius: 14px;
  border: 1px solid rgba(var(--v-border-color), 0.35);
  background: rgba(var(--v-theme-on-surface), 0.03);
  padding: 12px 12px 10px;
}
.metric-label {
  font-size: 0.72rem;
  color: rgba(var(--v-theme-on-surface), 0.55);
  margin-bottom: 6px;
}
.metric-value {
  font-size: 1.15rem;
  font-weight: 680;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
}

.empty {
  font-size: 0.86rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
  padding: 8px 0;
}
.backup-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.backup-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid rgba(var(--v-border-color), 0.35);
  background: rgba(var(--v-theme-on-surface), 0.02);
}
.backup-meta {
  flex: 1;
  min-width: 0;
}
.backup-name {
  font-weight: 600;
  font-size: 0.88rem;
}
.backup-path {
  font-size: 0.74rem;
  color: rgba(var(--v-theme-on-surface), 0.5);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

@media (prefers-reduced-transparency: reduce) {
  .surface {
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
    background: rgb(var(--v-theme-surface));
  }
}

@media (max-width: 600px) {
  .profile-inner {
    padding: 18px 14px 36px;
  }
  .profile-hero-actions {
    width: 100%;
  }
}
</style>
