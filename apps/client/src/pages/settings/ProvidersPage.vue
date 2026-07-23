<script setup lang="ts">
defineOptions({ name: "ProvidersPage" });

import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { api, errMessage, type ProviderKind } from "../../api";

const { t } = useI18n();
const router = useRouter();

const providers = ref<Awaited<ReturnType<typeof api.listProviders>>>([]);
const presets = ref<Awaited<ReturnType<typeof api.listModelPresets>>>([]);
const showForm = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const form = ref({
  name: "",
  kind: "open_ai_compatible" as ProviderKind,
  base_url: "https://api.openai.com/v1",
  api_key: "",
});

const kindItems = computed(() =>
  (["open_ai_compatible", "open_ai_responses", "anthropic"] as ProviderKind[]).map((value) => ({
    title: t(`providers.kinds.${value}`),
    value,
  })),
);

async function refresh() {
  providers.value = await api.listProviders();
  presets.value = await api.listModelPresets();
}

function kindLabel(kind: string) {
  const key = `providers.kinds.${kind}`;
  const label = t(key);
  return label === key ? kind : label;
}

function modelCount(providerId: string) {
  return presets.value.filter((p) => p.provider_id === providerId).length;
}

function resetForm() {
  form.value = {
    name: "",
    kind: "open_ai_compatible",
    base_url: "https://api.openai.com/v1",
    api_key: "",
  };
}

async function save() {
  error.value = null;
  saving.value = true;
  try {
    await api.upsertProvider({
      name: form.value.name.trim() || t("providers.title"),
      kind: form.value.kind,
      base_url: form.value.base_url.trim(),
      api_key: form.value.api_key.trim() || undefined,
    });
    showForm.value = false;
    resetForm();
    await refresh();
  } catch (e) {
    error.value = errMessage(e);
  } finally {
    saving.value = false;
  }
}

async function remove(id: string) {
  await api.deleteProvider(id);
  await refresh();
}

onMounted(refresh);
</script>

<template>
  <div class="settings-page providers-page">
    <div class="settings-page-inner">
      <header class="settings-page-header">
        <button
          type="button"
          class="settings-back-btn"
          :title="t('common.back')"
          @click="router.push('/settings')"
        >
          <v-icon icon="mdi-arrow-left" size="20" />
        </button>
        <div class="settings-page-heading">
          <h1 class="settings-page-title">{{ t("providers.title") }}</h1>
          <p class="settings-page-desc">{{ t("providers.desc") }}</p>
        </div>
        <button
          type="button"
          class="settings-primary-btn"
          @click="showForm = !showForm"
        >
          <v-icon icon="mdi-plus" size="18" />
          <span>{{ t("providers.add") }}</span>
        </button>
      </header>

      <v-alert
        v-if="error"
        type="error"
        variant="tonal"
        density="comfortable"
        class="mb-4"
        :text="error"
      />

      <v-expand-transition>
        <section v-if="showForm" class="settings-panel mb-4">
          <div class="settings-panel-title">{{ t("providers.add") }}</div>
          <div class="settings-panel-body">
            <div class="settings-form-grid">
              <v-text-field
                v-model="form.name"
                :label="t('providers.name')"
                :placeholder="t('providers.namePlaceholder')"
              />
              <v-select
                v-model="form.kind"
                :items="kindItems"
                item-title="title"
                item-value="value"
                :label="t('providers.kind')"
                :hint="t('providers.kindHint')"
                persistent-hint
              />
              <v-text-field
                class="settings-form-span"
                v-model="form.base_url"
                :label="t('providers.baseUrl')"
                :placeholder="t('providers.baseUrlPlaceholder')"
                type="url"
              />
              <v-text-field
                class="settings-form-span"
                v-model="form.api_key"
                :label="t('providers.apiKey')"
                :placeholder="t('providers.apiKeyPlaceholder')"
                type="password"
                autocomplete="off"
              />
            </div>
          </div>
          <div class="settings-panel-actions">
            <button type="button" class="settings-ghost-btn" @click="showForm = false; resetForm()">
              {{ t("providers.cancelAdd") }}
            </button>
            <button
              type="button"
              class="settings-primary-btn"
              :disabled="!form.base_url.trim() || saving"
              @click="save"
            >
              {{ t("providers.saveProvider") }}
            </button>
          </div>
        </section>
      </v-expand-transition>

      <section v-if="!providers.length" class="settings-panel settings-empty-panel">
        <div class="settings-empty-title">{{ t("providers.empty") }}</div>
        <div class="settings-empty-desc">{{ t("providers.emptyHint") }}</div>
        <button type="button" class="settings-primary-btn mt-4" @click="showForm = true">
          <v-icon icon="mdi-plus" size="18" />
          <span>{{ t("providers.add") }}</span>
        </button>
      </section>

      <section v-else class="provider-list">
        <button
          v-for="provider in providers"
          :key="provider.id"
          type="button"
          class="provider-card"
          @click="router.push(`/settings/providers/${provider.id}`)"
        >
          <span class="provider-card-icon">
            <v-icon icon="mdi-server" size="20" />
          </span>
          <span class="provider-card-main">
            <span class="provider-card-title">{{ provider.name }}</span>
            <span class="provider-card-sub">
              {{ kindLabel(provider.kind) }} · {{ provider.base_url }}
            </span>
          </span>
          <span class="provider-card-meta">
            <span class="provider-chip">
              {{ t("providers.modelsCount", { n: modelCount(provider.id) }) }}
            </span>
            <span
              class="provider-card-delete"
              role="button"
              :title="t('common.delete')"
              @click.stop="remove(provider.id)"
            >
              <v-icon icon="mdi-delete-outline" size="18" />
            </span>
            <v-icon icon="mdi-chevron-right" size="18" class="provider-card-chevron" />
          </span>
        </button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.providers-page {
  height: 100%;
  overflow: auto;
  background: transparent;
}
.settings-page-inner {
  max-width: 760px;
  margin: 0 auto;
  padding: 24px 20px 48px;
}
.settings-page-header {
  display: grid;
  grid-template-columns: 40px minmax(0, 1fr) auto;
  gap: 12px 14px;
  align-items: start;
  margin-bottom: 22px;
}
.settings-back-btn {
  width: 40px;
  height: 40px;
  border: 0;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.78);
  cursor: pointer;
  transition: background 0.12s ease, transform 0.1s ease;
}
.settings-back-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}
.settings-back-btn:active {
  transform: scale(0.96);
}
.settings-page-heading {
  min-width: 0;
  padding-top: 2px;
}
.settings-page-title {
  margin: 0 0 4px;
  font-size: 1.35rem;
  font-weight: 700;
  letter-spacing: 0;
  line-height: 1.25;
}
.settings-page-desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.9rem;
  line-height: 1.45;
}
.settings-primary-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 40px;
  padding: 0 14px;
  border: 0;
  border-radius: 12px;
  background: rgb(var(--v-theme-primary));
  color: rgb(var(--v-theme-on-primary));
  font: inherit;
  font-size: 0.9rem;
  font-weight: 620;
  cursor: pointer;
  white-space: nowrap;
  transition: filter 0.12s ease, transform 0.1s ease, opacity 0.12s ease;
}
.settings-primary-btn:hover {
  filter: brightness(1.04);
}
.settings-primary-btn:active {
  transform: scale(0.98);
}
.settings-primary-btn:disabled {
  opacity: 0.5;
  cursor: default;
  filter: none;
}
.settings-ghost-btn {
  min-height: 40px;
  padding: 0 14px;
  border: 0;
  border-radius: 12px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font: inherit;
  font-size: 0.9rem;
  font-weight: 600;
  cursor: pointer;
}
.settings-ghost-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.06);
}
.settings-panel {
  border: 1px solid var(--aerina-border);
  border-radius: 16px;
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  overflow: hidden;
}
.settings-panel-title {
  padding: 16px 18px 0;
  font-size: 0.98rem;
  font-weight: 680;
}
.settings-panel-body {
  padding: 14px 18px 4px;
}
.settings-form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px 12px;
}
.settings-form-span {
  grid-column: 1 / -1;
}
.settings-panel-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 8px 14px 14px;
}
.settings-empty-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 42px 20px;
}
.settings-empty-title {
  font-weight: 680;
  margin-bottom: 6px;
}
.settings-empty-desc {
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-size: 0.9rem;
  line-height: 1.5;
  max-width: 360px;
}
.provider-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.provider-card {
  width: 100%;
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  text-align: left;
  border: 1px solid var(--aerina-border);
  background: var(--aerina-material);
  backdrop-filter: var(--aerina-blur);
  -webkit-backdrop-filter: var(--aerina-blur);
  border-radius: 14px;
  padding: 14px 16px;
  color: inherit;
  cursor: pointer;
  transition: border-color 0.12s ease, background 0.12s ease, transform 0.1s ease;
}
.provider-card:hover {
  border-color: rgba(var(--v-theme-primary), 0.32);
  background: var(--aerina-material-heavy);
  transform: translateY(-1px);
}
.provider-card:active {
  transform: scale(0.995);
}
.provider-card-icon {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  background: rgba(var(--v-theme-primary), 0.12);
  color: rgb(var(--v-theme-primary));
  flex: 0 0 auto;
}
.provider-card-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.provider-card-title {
  font-weight: 650;
  font-size: 0.98rem;
  line-height: 1.3;
}
.provider-card-sub {
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.82rem;
  line-height: 1.35;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.provider-card-meta {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 0 0 auto;
}
.provider-chip {
  display: inline-flex;
  align-items: center;
  min-height: 26px;
  padding: 0 10px;
  border-radius: 999px;
  font-size: 0.75rem;
  font-weight: 650;
  background: rgba(var(--v-theme-on-surface), 0.06);
  color: rgba(var(--v-theme-on-surface), 0.72);
  white-space: nowrap;
}
.provider-card-delete {
  width: 34px;
  height: 34px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  color: rgba(var(--v-theme-on-surface), 0.55);
  transition: background 0.12s ease, color 0.12s ease;
}
.provider-card-delete:hover {
  background: rgba(var(--v-theme-error), 0.12);
  color: rgb(var(--v-theme-error));
}
.provider-card-chevron {
  color: rgba(var(--v-theme-on-surface), 0.38);
  margin-inline-start: 2px;
}

@media (max-width: 700px) {
  .settings-page-inner {
    padding-top: 18px;
  }
  .settings-page-header {
    grid-template-columns: 40px minmax(0, 1fr);
    align-items: center;
    margin-bottom: 14px;
  }
  .settings-page-title {
    display: none;
  }
  .settings-page-desc {
    font-size: 0.78rem;
  }
  .settings-primary-btn {
    grid-column: 1 / -1;
    width: 100%;
  }
  .settings-form-grid {
    grid-template-columns: 1fr;
  }
  .provider-card {
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 12px;
  }
  .provider-card-meta {
    grid-column: 2;
    justify-content: flex-start;
  }
}
</style>
